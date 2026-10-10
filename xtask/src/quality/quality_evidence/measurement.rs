// SPDX-License-Identifier: Apache-2.0

//! Execute quality tools and retain the reports supporting a measurement.
//!
//! A measurement records actual commands, tool versions, timing and captured
//! output in its exclusively owned run directory. Finishing verifies unchanged
//! inputs and validates native reports before atomically publishing their receipt.
//! Failed or interrupted runs must not become reusable passing evidence.

use super::{
    BTreeMap, Command, Instant, Invocation, MeasurementStore, Path, PathBuf, QualityGate,
    QualitySettings, Receipt, Stdio, cargo_command, command_output, constants, flags,
    generated_root, input_digest, output, plain_filename, progress, reports, rustc_command,
    sha256_file, unix_time_millis, validate_reports, workspace_root,
};
use std::fs;

/// Active measurement; failures never produce a passing receipt.
pub(crate) struct Measurement {
    /// Keeps the gate exclusively owned until validation and receipt publication finish.
    pub(super) _store: MeasurementStore,
    /// Destination beneath ignored generated results.
    pub(crate) directory: PathBuf,
    /// Provenance assembled from tool invocations and reports.
    pub(super) receipt: Receipt,
    /// Command-local policy snapshot used for execution and final validation.
    pub(crate) settings: QualitySettings,
}

impl Measurement {
    /// Starts a measurement and invalidates any earlier generated receipt.
    pub(crate) fn begin(gate: QualityGate) -> Result<Self, String> {
        let root = workspace_root()?;
        let settings = QualitySettings::load(&root)?;
        Self::begin_with_settings(gate, settings)
    }

    /// Starts a measurement using the command's already validated policy snapshot.
    pub(crate) fn begin_with_settings(
        gate: QualityGate,
        settings: QualitySettings,
    ) -> Result<Self, String> {
        let root = workspace_root()?;
        let toolchain = command_output(&rustc_command()?, &[flags::VERSION])?;
        if gate.requires_nightly() && !toolchain.contains("-nightly") {
            return Err("coverage and fuzz measurements require the isolated nightly toolchain; use RUSTUP_TOOLCHAIN=nightly".to_owned());
        }
        let inputs_sha256 = input_digest(&root)?;
        let store_root = generated_root(&settings)?
            .join(&inputs_sha256)
            .join(gate.as_str());
        let store = MeasurementStore::begin(&store_root)?;
        let directory = store.directory.clone();
        output::start(format!("{gate}: collecting source-bound measurements"));
        output::file("reports", &directory);
        Ok(Self {
            _store: store,
            directory,
            receipt: Receipt {
                schema_version: 2,
                gate,
                inputs_sha256,
                source_commit: command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?,
                policy_sha256: settings.policy_sha256.clone(),
                toolchain,
                finished_at_unix_ms: 0,
                invocations: Vec::new(),
                reports: BTreeMap::new(),
            },
            settings,
        })
    }

    /// Captures a command's reports and rejects unsuccessful exits.
    pub(crate) fn run(
        &mut self,
        program: &str,
        arguments: &[&str],
        report: &str,
    ) -> Result<(), String> {
        self.run_with_progress(program, arguments, report, None)
    }

    /// Captures tool reports with optional estimated wall-clock progress.
    fn run_with_progress(
        &mut self,
        program: &str,
        arguments: &[&str],
        report: &str,
        budget_seconds: Option<u64>,
    ) -> Result<(), String> {
        if !plain_filename(report) {
            return Err("tool report must be a plain filename".to_owned());
        }
        let version_arguments: Vec<&str> = if program == cargo_command()?
            && arguments.first().is_some_and(|argument| {
                matches!(*argument, "llvm-cov" | "mutants" | "fuzz" | "audit")
            }) {
            vec![arguments[0], flags::VERSION]
        } else {
            vec![flags::VERSION]
        };
        let tool_version = command_output(program, &version_arguments)?;
        if tool_version.trim().is_empty() {
            return Err("measurement tool reported an empty version".to_owned());
        }
        let stdout = fs::File::create(self.directory.join(format!("{report}.stdout")))
            .map_err(|error| format!("could not create tool output: {error}"))?;
        let stderr = fs::File::create(self.directory.join(format!("{report}.stderr")))
            .map_err(|error| format!("could not create tool errors: {error}"))?;
        let label = format!("{}/{report}", self.receipt.gate);
        output::start(&label);
        output::invocation(program, arguments);
        let start = Instant::now();
        let mut child = Command::new(program)
            .current_dir(workspace_root()?)
            .args(arguments)
            .env(constants::CARGO_TERM_COLOR_ENV, "never")
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("could not run measured tool: {error}"))?;
        let mutation_directory = (self.receipt.gate == QualityGate::Mutation)
            .then(|| self.directory.join("mutants.out"));
        let status = progress::wait(
            &mut child,
            &label,
            budget_seconds,
            mutation_directory.as_deref(),
        )
        .map_err(|error| format!("could not wait for measured tool: {error}"))?;
        if !status.success() {
            if let Some(directory) = &mutation_directory {
                for (label, name) in [
                    ("surviving mutants", "missed.txt"),
                    ("native reports", "outcomes.json"),
                ] {
                    let path = directory.join(name);
                    if path.is_file() {
                        output::file(label, &path);
                    }
                }
            }
            output::file(
                "tool errors",
                &self.directory.join(format!("{report}.stderr")),
            );
            return Err(format!(
                "{label} failed ({}, {status}); reports {}",
                output::duration(start.elapsed()),
                output::path(&self.directory)
            ));
        }
        output::pass(format!("{label} ({})", output::duration(start.elapsed())));
        self.receipt.invocations.push(Invocation {
            program: program.to_owned(),
            tool_version,
            arguments: arguments.iter().map(|value| (*value).to_owned()).collect(),
            elapsed_ms: start.elapsed().as_millis(),
            report: report.to_owned(),
        });
        self.add_report(&format!("{report}.stdout"))?;
        self.add_report(&format!("{report}.stderr"))
    }

    /// Runs Cargo through the configured local executable.
    pub(crate) fn cargo(&mut self, arguments: &[&str], report: &str) -> Result<(), String> {
        self.run(&cargo_command()?, arguments, report)
    }

    /// Runs a retained Cargo measurement with an approximate progress budget.
    pub(crate) fn cargo_with_progress(
        &mut self,
        arguments: &[&str],
        report: &str,
        budget_seconds: u64,
    ) -> Result<(), String> {
        self.run_with_progress(&cargo_command()?, arguments, report, Some(budget_seconds))
    }

    /// Copies and hashes a native tool report before retaining evidence.
    pub(crate) fn copy_report(&mut self, source: &Path, filename: &str) -> Result<(), String> {
        if !plain_filename(filename) {
            return Err("unsafe quality report filename".to_owned());
        }
        let destination = self.directory.join(filename);
        if source != destination {
            fs::copy(source, &destination)
                .map_err(|error| format!("could not retain report: {error}"))?;
        }
        self.add_report(filename)
    }

    /// Records the exact bytes of a generated report.
    fn add_report(&mut self, filename: &str) -> Result<(), String> {
        self.receipt.reports.insert(
            filename.to_owned(),
            sha256_file(&self.directory.join(filename))?,
        );
        Ok(())
    }

    /// Writes evidence only after validation and detection of mid-run input changes.
    pub(crate) fn finish(mut self) -> Result<(), String> {
        let root = workspace_root()?;
        if input_digest(&root)? != self.receipt.inputs_sha256
            || sha256_file(&root.join(constants::QUALITY_GATES_FILE))? != self.receipt.policy_sha256
        {
            return Err("quality inputs changed during measurement; rerun the gate".to_owned());
        }
        self.receipt.finished_at_unix_ms = unix_time_millis()?;
        validate_reports(&self.receipt, &self.directory, &self.settings)?;
        let json = serde_json::to_string_pretty(&self.receipt)
            .map_err(|error| format!("could not serialize receipt: {error}"))?;
        let pending = self.directory.join("receipt.pending");
        fs::write(&pending, format!("{json}\n"))
            .map_err(|error| format!("could not write receipt: {error}"))?;
        fs::rename(pending, self.directory.join(reports::RECEIPT))
            .map_err(|error| format!("could not publish receipt: {error}"))?;
        output::pass(format!(
            "{} ({} tool runs)",
            self.receipt.gate,
            self.receipt.invocations.len()
        ));
        output::file("receipt", &self.directory.join(reports::RECEIPT));
        Ok(())
    }
}
