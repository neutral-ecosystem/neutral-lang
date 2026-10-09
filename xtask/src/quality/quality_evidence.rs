// SPDX-License-Identifier: Apache-2.0

//! Automatically retained, source-bound quality measurements.

use super::gate::QualityGate;
use super::measurement_store::{self, MeasurementStore};
use crate::config::quality_settings::QualitySettings;
use crate::constants::{flags, reports};
use crate::*;
use serde::{Deserialize, Serialize};
use std::process::Stdio;

/// Successful tool invocation supporting a measurement.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    /// Executable used by the measurement.
    program: String,
    /// Actual measured executable/subcommand version, not a configured expectation.
    tool_version: String,
    /// Exact argument vector.
    arguments: Vec<String>,
    /// Monotonic elapsed wall time.
    elapsed_ms: u128,
    /// Basename of captured stdout and stderr reports.
    report: String,
}

/// Automatically generated provenance; acceptance is computed from tool reports.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    /// Supported schema.
    schema_version: u32,
    /// Stable measurement name.
    gate: QualityGate,
    /// Checkout commit used by the tool.
    source_commit: String,
    /// Digest of actual tracked and untracked measurement inputs.
    inputs_sha256: String,
    /// Exact threshold policy digest.
    policy_sha256: String,
    /// Selected measurement compiler.
    toolchain: String,
    /// Completion time for freshness checks.
    finished_at_unix_ms: u128,
    /// Successful invocations.
    invocations: Vec<Invocation>,
    /// Report filenames and exact-byte SHA-256 digests.
    reports: BTreeMap<String, String>,
}

/// Active measurement; failures never produce a passing receipt.
pub(crate) struct Measurement {
    /// Keeps the gate exclusively owned until validation and receipt publication finish.
    _store: MeasurementStore,
    /// Destination beneath ignored generated results.
    pub(crate) directory: PathBuf,
    /// Provenance assembled from tool invocations and reports.
    receipt: Receipt,
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

/// Accepts only one safe workspace-relative filename.
fn plain_filename(value: &str) -> bool {
    is_safe_relative_path(Path::new(value)) && Path::new(value).components().count() == 1
}

/// Resolves the configurable ignored measurement root.
fn generated_root(settings: &QualitySettings) -> Result<PathBuf, String> {
    Ok(result_root()?.join(&settings.evidence_root))
}

/// Hashes ordered code, tests, fixtures, locks, and configuration, including new files.
pub(crate) fn input_digest(root: &Path) -> Result<String, String> {
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .map_err(|error| format!("could not list gate inputs: {error}"))?;
    if !output.status.success() {
        return Err("could not list tracked gate inputs".to_owned());
    }
    let paths = String::from_utf8(output.stdout)
        .map_err(|error| format!("non-UTF-8 gate input: {error}"))?;
    let mut hash = Sha256::new();
    hash.update(b"neutral-quality-inputs/2\0");
    let paths = paths
        .split('\0')
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    for relative in paths {
        let path = Path::new(relative);
        // Only known guide locations are excluded: Markdown can contain executable fixtures.
        let guide = path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
            && (path.parent() == Some(Path::new("docs"))
                || path.file_name().and_then(std::ffi::OsStr::to_str) == Some("README.md")
                || path == Path::new(constants::QUALITY_STATUS_FILE));
        if guide
            || (path.starts_with(constants::QUALITY_EVIDENCE_DIRECTORY)
                && path.file_name().and_then(std::ffi::OsStr::to_str) == Some("record.toml"))
        {
            continue;
        }
        let metadata = match fs::symlink_metadata(root.join(path)) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("could not inspect gate input {relative}: {error}")),
        };
        if !metadata.is_file() {
            return Err(format!("gate input must be a regular file: {relative}"));
        }
        let bytes = fs::read(root.join(path))
            .map_err(|error| format!("could not read gate input {relative}: {error}"))?;
        hash.update((relative.len() as u64).to_be_bytes());
        hash.update(relative.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Resolves the release's ignored local snapshot directory; raw reports never belong in Git.
fn retained_root(root: &Path) -> Result<PathBuf, String> {
    let version = workspace_package_version(&read_workspace_text(
        root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    Ok(root
        .join(constants::QUALITY_EVIDENCE_DIRECTORY)
        .join(format!("v{version}"))
        .join("gates"))
}

/// Verifies generated or retained reports for the current input bytes.
pub(crate) fn verify_all() -> Result<(), String> {
    let root = workspace_root()?;
    let settings = QualitySettings::load(&root)?;
    verify_all_with(&root, &settings)
}

/// Verifies all gates against one policy snapshot, including retention callers.
fn verify_all_with(root: &Path, settings: &QualitySettings) -> Result<(), String> {
    let digest = input_digest(root)?;
    let generated = generated_root(settings)?.join(&digest);
    let retained = retained_root(root)?.join(&digest);
    for gate in QualityGate::RELEASE_REQUIRED {
        verify_gate_at(gate, &digest, &generated, &retained, settings)?;
    }
    Ok(())
}

/// Verifies one current gate so release preparation can reuse valid expensive measurements.
pub(crate) fn verify_gate(gate: QualityGate) -> Result<(), String> {
    let root = workspace_root()?;
    verify_gate_with(&root, &QualitySettings::load(&root)?, gate)
}

/// Checks generated and locally retained evidence under the same acceptance policy.
fn verify_gate_with(
    root: &Path,
    settings: &QualitySettings,
    gate: QualityGate,
) -> Result<(), String> {
    let digest = input_digest(root)?;
    let generated = generated_root(settings)?.join(&digest);
    let retained = retained_root(root)?.join(&digest);
    verify_gate_at(gate, &digest, &generated, &retained, settings)
}

/// Checks one gate at precomputed roots, keeping aggregate input hashing request-local.
fn verify_gate_at(
    gate: QualityGate,
    digest: &str,
    generated: &Path,
    retained: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let generated_gate = generated.join(gate.as_str());
    // A later failed explicit rerun supersedes older retained success.
    let candidates = if fs::symlink_metadata(generated_gate.join("current")).is_ok() {
        vec![generated_gate]
    } else {
        vec![generated_gate, retained.join(gate.as_str())]
    };
    let accepted =
        candidates.iter().any(
            |directory| match verify(directory, gate, digest, settings) {
                Ok(()) => true,
                Err(error) => {
                    errors.push(error);
                    false
                }
            },
        );
    if !accepted {
        return Err(format!(
            "missing or invalid {gate} evidence for current inputs: {}; run `cargo xtask release prepare` to regenerate required measurements",
            errors.join("; ")
        ));
    }
    Ok(())
}

/// Copies verified reports beside an approval without changing its historical record.
pub(crate) fn retain() -> Result<(), String> {
    let root = workspace_root()?;
    let settings = QualitySettings::load(&root)?;
    verify_all_with(&root, &settings)?;
    let digest = input_digest(&root)?;
    let generated = generated_root(&settings)?.join(&digest);
    let retained = retained_root(&root)?.join(&digest);
    for gate in QualityGate::RELEASE_REQUIRED {
        let source = measurement_store::resolve(&generated.join(gate.as_str()))?;
        let destination = retained.join(gate.as_str());
        if verify(&destination, gate, &digest, &settings).is_ok() {
            continue;
        }
        verify(&source, gate, &digest, &settings)?;
        let receipt = read_receipt(&source)?;
        fs::create_dir_all(&destination)
            .map_err(|error| format!("could not retain gate: {error}"))?;
        for filename in receipt.reports.keys() {
            fs::copy(source.join(filename), destination.join(filename))
                .map_err(|error| format!("could not retain report: {error}"))?;
        }
        fs::copy(
            source.join(reports::RECEIPT),
            destination.join(reports::RECEIPT),
        )
        .map_err(|error| format!("could not retain receipt: {error}"))?;
    }
    Ok(())
}

/// Loads one strict receipt schema.
fn read_receipt(directory: &Path) -> Result<Receipt, String> {
    serde_json::from_slice(
        &fs::read(directory.join(reports::RECEIPT))
            .map_err(|error| format!("{}: {error}", directory.display()))?,
    )
    .map_err(|error| format!("invalid quality receipt: {error}"))
}

/// Rejects wrong-source, wrong-policy, modified, or incomplete measurements.
fn verify(
    directory: &Path,
    gate: QualityGate,
    inputs: &str,
    settings: &QualitySettings,
) -> Result<(), String> {
    let resolved = measurement_store::resolve(directory)?;
    let directory = resolved.as_path();
    let receipt = read_receipt(directory)?;
    if receipt.schema_version != 2
        || receipt.gate != gate
        || receipt.inputs_sha256 != inputs
        || receipt.policy_sha256 != settings.policy_sha256
        || receipt.source_commit.len() != 40
        || receipt.toolchain.is_empty()
        || receipt.finished_at_unix_ms == 0
        || receipt.invocations.is_empty()
    {
        return Err(format!("{gate} receipt is stale or incomplete"));
    }
    validate_reports(&receipt, directory, settings)
}

/// Parses actual retained JSON output from a measurement tool.
fn json_report(directory: &Path, name: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(
        &fs::read(directory.join(name)).map_err(|error| format!("missing {name}: {error}"))?,
    )
    .map_err(|error| format!("invalid {name}: {error}"))
}

/// Checks exact report bytes and independent measured acceptance criteria.
fn validate_reports(
    receipt: &Receipt,
    directory: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    if receipt.gate.requires_nightly() && !receipt.toolchain.contains("-nightly") {
        return Err("analysis measurement was not produced with nightly".to_owned());
    }
    if receipt.invocations.is_empty() {
        return Err("measurement has no invocations".to_owned());
    }
    for (filename, expected) in &receipt.reports {
        if !plain_filename(filename)
            || !is_sha256(expected)
            || sha256_file(&directory.join(filename))? != *expected
        {
            return Err(format!("modified or unsafe report: {filename}"));
        }
    }
    for invocation in &receipt.invocations {
        if invocation.tool_version.trim().is_empty() {
            return Err("measurement has no tool version".to_owned());
        }
        if !plain_filename(&invocation.report) {
            return Err("unsafe invocation report".to_owned());
        }
        for suffix in ["stdout", "stderr"] {
            if !receipt
                .reports
                .contains_key(&format!("{}.{suffix}", invocation.report))
            {
                return Err("invocation has no captured report".to_owned());
            }
        }
    }
    match receipt.gate {
        QualityGate::Coverage => validate_coverage(receipt, directory, settings),
        QualityGate::Mutation => validate_mutation(receipt, directory, settings),
        QualityGate::Fuzz => validate_fuzz(receipt, directory, settings),
        QualityGate::PerformanceRelease | QualityGate::PerformanceSoak => {
            validate_performance(receipt, directory, settings)
        }
        QualityGate::Advisories => validate_advisories(receipt, directory, settings),
    }
}

/// Validates native coverage measurements against the configured acceptance policy.
fn validate_coverage(
    receipt: &Receipt,
    directory: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    require_report(receipt, "coverage.json")?;
    let report = json_report(directory, "coverage.json")?;
    for (metric, threshold) in [
        ("lines", settings.policy.coverage.minimum_line_percent),
        (
            "functions",
            settings.policy.coverage.minimum_function_percent,
        ),
        ("regions", settings.policy.coverage.minimum_region_percent),
    ] {
        let observed = report
            .pointer(&format!("/data/0/totals/{metric}/percent"))
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| format!("coverage has no {metric} percentage"))?;
        let minimum = threshold;
        if !observed.is_finite() || observed < minimum || observed > 100.0 {
            return Err(format!(
                "measured coverage {metric} {observed} fails minimum {minimum}"
            ));
        }
    }
    Ok(())
}

/// Validates native mutation measurements against the configured acceptance policy.
fn validate_mutation(
    receipt: &Receipt,
    directory: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    require_report(receipt, "outcomes.json")?;
    let targets = &settings.policy.mutation.critical_targets;
    if !receipt.invocations.iter().any(|run| {
        let selected = run
            .arguments
            .windows(2)
            .filter(|args| args[0] == "--file")
            .map(|args| args[1].as_str())
            .collect::<BTreeSet<_>>();
        selected == targets.iter().map(String::as_str).collect::<BTreeSet<_>>()
            && run.arguments.contains(&"--no-config".to_owned())
            && run
                .arguments
                .contains(&"--cargo-arg=--workspace".to_owned())
            && !run.arguments.iter().any(|argument| {
                matches!(
                    argument.split('=').next().unwrap_or(argument),
                    "--exclude"
                        | "--exclude-re"
                        | "--re"
                        | "--shard"
                        | "--limit"
                        | "--package"
                        | "-p"
                        | "--config"
                        | "--skip-calls"
                )
            })
    }) {
        return Err("mutation does not cover the complete configured critical scope".to_owned());
    }
    let report = json_report(directory, "outcomes.json")?;
    let count = |name: &str| {
        report[name]
            .as_u64()
            .ok_or_else(|| format!("mutation has no {name}"))
    };
    let caught = count("caught")?;
    let missed = count("missed")?;
    let minimum = settings.policy.mutation.minimum_caught_percent;
    #[allow(clippy::cast_precision_loss)]
    let percentage = 100.0 * caught as f64
        / caught
            .checked_add(missed)
            .ok_or("mutation count overflow")? as f64;
    if caught == 0 || count("timeout")? != 0 || count("success")? != 0 || percentage < minimum {
        return Err("mutation results fail the caught threshold".to_owned());
    }
    Ok(())
}

/// Validates native fuzz measurements against the configured acceptance policy.
fn validate_fuzz(
    receipt: &Receipt,
    directory: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    let minimum = u128::from(settings.policy.fuzz.minimum_seconds_per_target);
    for target in &settings.policy.fuzz.targets {
        if !receipt.invocations.iter().any(|run| {
            run.report == *target
                && run.elapsed_ms >= minimum * 1_000
                && run
                    .arguments
                    .contains(&format!("-max_total_time={minimum}"))
        }) {
            return Err(format!(
                "fuzz target {target} has no full measured campaign"
            ));
        }
        let output = fs::read_to_string(directory.join(format!("{target}.stderr")))
            .map_err(|error| format!("missing fuzz log: {error}"))?;
        let seconds = output
            .lines()
            .filter_map(|line| line.strip_prefix("Done "))
            .filter_map(|line| line.split_once(" runs in ").map(|(_, time)| time))
            .filter_map(|time| time.split_whitespace().next()?.parse::<u128>().ok())
            .max();
        if seconds.is_none_or(|seconds| seconds < minimum) {
            return Err(format!(
                "fuzzer {target} did not confirm its full run duration"
            ));
        }
    }
    Ok(())
}

/// Validates native performance measurements against the configured acceptance policy.
fn validate_performance(
    receipt: &Receipt,
    directory: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    for report in [
        reports::BENCHMARK_STDOUT,
        reports::MASSIF,
        reports::MEMCHECK_STDERR,
    ] {
        require_report(receipt, report)?;
    }
    let profile = if receipt.gate == QualityGate::PerformanceSoak {
        "extended-soak"
    } else {
        "release"
    };
    if !receipt.invocations.iter().any(|run| {
        run.report == "benchmark" && run.arguments.last().is_some_and(|arg| arg == profile)
    }) {
        return Err("performance profile does not match its gate".to_owned());
    }
    let output = fs::read_to_string(directory.join(reports::BENCHMARK_STDOUT))
        .map_err(|error| format!("missing benchmark: {error}"))?;
    for phase in &settings.policy.performance.required_phases {
        if !output.contains(&format!("name={phase} ")) {
            return Err(format!("benchmark has no {phase} measurement"));
        }
    }
    let massif = fs::read_to_string(directory.join(reports::MASSIF))
        .map_err(|error| format!("missing heap profile: {error}"))?;
    let memcheck = fs::read_to_string(directory.join(reports::MEMCHECK_STDERR))
        .map_err(|error| format!("missing allocation review: {error}"))?;
    if !massif.contains("mem_heap_B=")
        || !memcheck.contains("ERROR SUMMARY: 0 errors")
        || !receipt.invocations.iter().any(|run| {
            run.report == "memcheck" && run.arguments.contains(&"--error-exitcode=1".to_owned())
        })
    {
        return Err("performance has no successful heap/allocation review".to_owned());
    }
    Ok(())
}

/// Validates native advisories measurements against the configured acceptance policy.
fn validate_advisories(
    receipt: &Receipt,
    directory: &Path,
    settings: &QualitySettings,
) -> Result<(), String> {
    for lock in &settings.advisory_locks {
        if !receipt.invocations.iter().any(|run| {
            run.arguments
                .windows(2)
                .any(|args| args == ["--file", lock.as_str()])
        }) {
            return Err(format!("advisory scan omitted {lock}"));
        }
    }
    let age = u128::from(settings.advisory_max_age_seconds);
    let now = unix_time_millis()?;
    if receipt.finished_at_unix_ms > now || now - receipt.finished_at_unix_ms > age * 1_000 {
        return Err("advisory scan is stale; rerun release quality".to_owned());
    }
    for invocation in &receipt.invocations {
        let report = json_report(directory, &format!("{}.stdout", invocation.report))?;
        if report
            .pointer("/vulnerabilities/found")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
            || report
                .pointer("/vulnerabilities/count")
                .and_then(serde_json::Value::as_u64)
                != Some(0)
        {
            return Err("advisory scan does not prove zero vulnerabilities".to_owned());
        }
    }
    Ok(())
}

/// Requires a native acceptance report to be covered by the receipt's checksums.
fn require_report(receipt: &Receipt, filename: &str) -> Result<(), String> {
    if receipt.reports.contains_key(filename) {
        Ok(())
    } else {
        Err(format!("unhashed acceptance report: {filename}"))
    }
}

/// Scans every declared dependency lock against a freshly fetched advisory database.
pub(crate) fn advisory_scan() -> Result<(), String> {
    let mut measurement = Measurement::begin(QualityGate::Advisories)?;
    let locks = measurement.settings.advisory_locks.clone();
    let database = measurement.directory.join("advisory-db");
    let database = database
        .to_str()
        .ok_or("non-UTF-8 advisory database path")?;
    for (index, lock) in locks.iter().enumerate() {
        if !is_safe_relative_path(Path::new(lock)) {
            return Err("unsafe advisory lockfile".to_owned());
        }
        measurement.cargo(
            &["audit", "--json", "--db", database, "--file", lock],
            &format!("lock-{index}"),
        )?;
    }
    measurement.finish()
}

#[cfg(test)]
#[path = "../../tests/unit/quality_evidence.rs"]
mod tests;
