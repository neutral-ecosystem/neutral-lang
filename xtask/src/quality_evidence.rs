// SPDX-License-Identifier: Apache-2.0

//! Automatically retained, source-bound quality measurements.

use super::*;
use serde::{Deserialize, Serialize};
use std::process::Stdio;

/// Successful tool invocation supporting a measurement.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    /// Executable used by the measurement.
    program: String,
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
    gate: String,
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

/// Required release measurements, independent of historical status labels.
const REQUIRED_GATES: &[&str] = &[
    "coverage",
    "mutation",
    "fuzz",
    "performance-release",
    "performance-soak",
    "advisories",
];

/// Active measurement; failures never produce a passing receipt.
pub(super) struct Measurement {
    /// Destination beneath ignored generated results.
    pub(super) directory: PathBuf,
    /// Provenance assembled from tool invocations and reports.
    receipt: Receipt,
}

impl Measurement {
    /// Starts a measurement and invalidates any earlier generated receipt.
    pub(super) fn begin(gate: &str) -> Result<Self, String> {
        if !REQUIRED_GATES.contains(&gate) {
            return Err(format!("unknown quality gate: {gate}"));
        }
        let root = workspace_root()?;
        let toolchain = command_output(&rustc_command()?, &["--version"])?;
        if matches!(gate, "coverage" | "fuzz") && !toolchain.contains("-nightly") {
            return Err("coverage and fuzz measurements require the isolated nightly toolchain; use RUSTUP_TOOLCHAIN=nightly".to_owned());
        }
        let inputs_sha256 = input_digest(&root)?;
        let directory = generated_root()?.join(&inputs_sha256).join(gate);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("could not create gate reports: {error}"))?;
        let old = directory.join("receipt.json");
        if old.exists() {
            fs::remove_file(old)
                .map_err(|error| format!("could not invalidate receipt: {error}"))?;
        }
        Ok(Self {
            directory,
            receipt: Receipt {
                schema_version: 1,
                gate: gate.to_owned(),
                inputs_sha256,
                source_commit: command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?,
                policy_sha256: sha256_file(&root.join(constants::QUALITY_GATES_FILE))?,
                toolchain,
                finished_at_unix_ms: 0,
                invocations: Vec::new(),
                reports: BTreeMap::new(),
            },
        })
    }

    /// Captures a command's reports and rejects unsuccessful exits.
    pub(super) fn run(
        &mut self,
        program: &str,
        arguments: &[&str],
        report: &str,
    ) -> Result<(), String> {
        if !plain_filename(report) {
            return Err("tool report must be a plain filename".to_owned());
        }
        let stdout = fs::File::create(self.directory.join(format!("{report}.stdout")))
            .map_err(|error| format!("could not create tool output: {error}"))?;
        let stderr = fs::File::create(self.directory.join(format!("{report}.stderr")))
            .map_err(|error| format!("could not create tool errors: {error}"))?;
        println!(
            "{} measuring {}: {program} {}; reports {}",
            constants::INFO,
            self.receipt.gate,
            arguments.join(" "),
            self.directory.display()
        );
        let start = Instant::now();
        let status = Command::new(program)
            .current_dir(workspace_root()?)
            .args(arguments)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .status()
            .map_err(|error| format!("could not run measured tool: {error}"))?;
        if !status.success() {
            return Err(format!(
                "measurement {} failed with {status}; reports {}",
                self.receipt.gate,
                self.directory.display()
            ));
        }
        self.receipt.invocations.push(Invocation {
            program: program.to_owned(),
            arguments: arguments.iter().map(|value| (*value).to_owned()).collect(),
            elapsed_ms: start.elapsed().as_millis(),
            report: report.to_owned(),
        });
        self.add_report(&format!("{report}.stdout"))?;
        self.add_report(&format!("{report}.stderr"))
    }

    /// Runs Cargo through the configured local executable.
    pub(super) fn cargo(&mut self, arguments: &[&str], report: &str) -> Result<(), String> {
        self.run(&cargo_command()?, arguments, report)
    }

    /// Copies and hashes a native tool report before retaining evidence.
    pub(super) fn copy_report(&mut self, source: &Path, filename: &str) -> Result<(), String> {
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
    pub(super) fn finish(mut self) -> Result<(), String> {
        let root = workspace_root()?;
        if input_digest(&root)? != self.receipt.inputs_sha256
            || sha256_file(&root.join(constants::QUALITY_GATES_FILE))? != self.receipt.policy_sha256
        {
            return Err("quality inputs changed during measurement; rerun the gate".to_owned());
        }
        self.receipt.finished_at_unix_ms = unix_time_millis()?;
        validate_reports(&self.receipt, &self.directory)?;
        let json = serde_json::to_string_pretty(&self.receipt)
            .map_err(|error| format!("could not serialize receipt: {error}"))?;
        fs::write(self.directory.join("receipt.json"), format!("{json}\n"))
            .map_err(|error| format!("could not write receipt: {error}"))?;
        println!(
            "{} verified {} evidence: {}",
            constants::INFO,
            self.receipt.gate,
            self.directory.display()
        );
        Ok(())
    }
}

/// Accepts only one safe workspace-relative filename.
fn plain_filename(value: &str) -> bool {
    is_safe_relative_path(Path::new(value)) && Path::new(value).components().count() == 1
}

/// Resolves the configurable ignored measurement root.
fn generated_root() -> Result<PathBuf, String> {
    let relative = PathBuf::from(automation_value("quality", "evidence_root")?);
    if !is_safe_relative_path(&relative) {
        return Err("quality evidence root must be relative".to_owned());
    }
    Ok(result_root()?.join(relative))
}

/// Hashes ordered code, tests, fixtures, locks, and configuration, including new files.
pub(super) fn input_digest(root: &Path) -> Result<String, String> {
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
    let paths = paths
        .split('\0')
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    for relative in paths {
        let path = Path::new(relative);
        if path.starts_with("quality")
            || path.starts_with("docs")
            || !matches!(
                path.extension().and_then(std::ffi::OsStr::to_str),
                Some("rs" | "toml" | "lock" | "neu" | "json")
            )
        {
            continue;
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

/// Resolves the release's durable machine evidence directory.
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
pub(super) fn verify_all() -> Result<(), String> {
    let root = workspace_root()?;
    let digest = input_digest(&root)?;
    let generated = generated_root()?.join(&digest);
    let retained = retained_root(&root)?.join(&digest);
    for gate in REQUIRED_GATES {
        let mut errors = Vec::new();
        let accepted = [generated.join(gate), retained.join(gate)]
            .iter()
            .any(|directory| match verify(directory, gate, &digest, &root) {
                Ok(()) => true,
                Err(error) => {
                    errors.push(error);
                    false
                }
            });
        if !accepted {
            return Err(format!(
                "missing or invalid {gate} evidence for current inputs: {}; run the documented quality measurements and retain them with `cargo xtask quality approve --release <version>`",
                errors.join("; ")
            ));
        }
    }
    Ok(())
}

/// Copies verified reports beside an approval without changing its historical record.
pub(super) fn retain() -> Result<(), String> {
    verify_all()?;
    let root = workspace_root()?;
    let digest = input_digest(&root)?;
    let generated = generated_root()?.join(&digest);
    let retained = retained_root(&root)?.join(&digest);
    for gate in REQUIRED_GATES {
        let source = generated.join(gate);
        let destination = retained.join(gate);
        if verify(&destination, gate, &digest, &root).is_ok() {
            continue;
        }
        verify(&source, gate, &digest, &root)?;
        let receipt = read_receipt(&source)?;
        fs::create_dir_all(&destination)
            .map_err(|error| format!("could not retain gate: {error}"))?;
        for filename in receipt.reports.keys() {
            fs::copy(source.join(filename), destination.join(filename))
                .map_err(|error| format!("could not retain report: {error}"))?;
        }
        fs::copy(
            source.join("receipt.json"),
            destination.join("receipt.json"),
        )
        .map_err(|error| format!("could not retain receipt: {error}"))?;
    }
    Ok(())
}

/// Loads one strict receipt schema.
fn read_receipt(directory: &Path) -> Result<Receipt, String> {
    serde_json::from_slice(
        &fs::read(directory.join("receipt.json"))
            .map_err(|error| format!("{}: {error}", directory.display()))?,
    )
    .map_err(|error| format!("invalid quality receipt: {error}"))
}

/// Rejects wrong-source, wrong-policy, modified, or incomplete measurements.
fn verify(directory: &Path, gate: &str, inputs: &str, root: &Path) -> Result<(), String> {
    let receipt = read_receipt(directory)?;
    if receipt.schema_version != 1
        || receipt.gate != gate
        || receipt.inputs_sha256 != inputs
        || receipt.policy_sha256 != sha256_file(&root.join(constants::QUALITY_GATES_FILE))?
        || receipt.source_commit.len() != 40
        || receipt.toolchain.is_empty()
        || receipt.finished_at_unix_ms == 0
        || receipt.invocations.is_empty()
    {
        return Err(format!("{gate} receipt is stale or incomplete"));
    }
    validate_reports(&receipt, directory)
}

/// Parses actual retained JSON output from a measurement tool.
fn json_report(directory: &Path, name: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(
        &fs::read(directory.join(name)).map_err(|error| format!("missing {name}: {error}"))?,
    )
    .map_err(|error| format!("invalid {name}: {error}"))
}

/// Checks exact report bytes and independent measured acceptance criteria.
fn validate_reports(receipt: &Receipt, directory: &Path) -> Result<(), String> {
    if matches!(receipt.gate.as_str(), "coverage" | "fuzz")
        && !receipt.toolchain.contains("-nightly")
    {
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
    match receipt.gate.as_str() {
        "coverage" => validate_coverage(receipt, directory),
        "mutation" => validate_mutation(receipt, directory),
        "fuzz" => validate_fuzz(receipt, directory),
        "performance-release" | "performance-soak" => validate_performance(receipt, directory),
        "advisories" => validate_advisories(receipt, directory),
        _ => Err("unknown evidence gate".to_owned()),
    }
}

/// Validates native coverage measurements against the configured acceptance policy.
fn validate_coverage(receipt: &Receipt, directory: &Path) -> Result<(), String> {
    require_report(receipt, "coverage.json")?;
    let report = json_report(directory, "coverage.json")?;
    for (metric, threshold) in [
        ("lines", "minimum_line_percent"),
        ("functions", "minimum_function_percent"),
        ("regions", "minimum_region_percent"),
    ] {
        let observed = report
            .pointer(&format!("/data/0/totals/{metric}/percent"))
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| format!("coverage has no {metric} percentage"))?;
        let minimum = quality_value("coverage", threshold)?
            .parse::<f64>()
            .map_err(|error| format!("invalid threshold: {error}"))?;
        if !observed.is_finite() || observed < minimum || observed > 100.0 {
            return Err(format!(
                "measured coverage {metric} {observed} fails minimum {minimum}"
            ));
        }
    }
    Ok(())
}

/// Validates native mutation measurements against the configured acceptance policy.
fn validate_mutation(receipt: &Receipt, directory: &Path) -> Result<(), String> {
    require_report(receipt, "outcomes.json")?;
    let target = quality_value("mutation", "critical_target")?;
    if !receipt.invocations.iter().any(|run| {
        run.arguments
            .windows(2)
            .any(|args| args == ["--file", target.as_str()])
    }) {
        return Err("mutation does not cover the configured target".to_owned());
    }
    let report = json_report(directory, "outcomes.json")?;
    let count = |name: &str| {
        report[name]
            .as_u64()
            .ok_or_else(|| format!("mutation has no {name}"))
    };
    let caught = count("caught")?;
    let missed = count("missed")?;
    let minimum = quality_value("mutation", "minimum_caught_percent")?
        .parse::<f64>()
        .map_err(|error| format!("invalid mutation threshold: {error}"))?;
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
fn validate_fuzz(receipt: &Receipt, directory: &Path) -> Result<(), String> {
    let minimum = quality_value("fuzz", "minimum_seconds_per_target")?
        .parse::<u128>()
        .map_err(|error| format!("invalid fuzz budget: {error}"))?;
    for target in quality_array("fuzz", "targets")? {
        if !receipt.invocations.iter().any(|run| {
            run.report == target
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
fn validate_performance(receipt: &Receipt, directory: &Path) -> Result<(), String> {
    for report in ["benchmark.stdout", "massif.out", "memcheck.stderr"] {
        require_report(receipt, report)?;
    }
    let profile = if receipt.gate == "performance-soak" {
        "extended-soak"
    } else {
        "release"
    };
    if !receipt.invocations.iter().any(|run| {
        run.report == "benchmark" && run.arguments.last().is_some_and(|arg| arg == profile)
    }) {
        return Err("performance profile does not match its gate".to_owned());
    }
    let output = fs::read_to_string(directory.join("benchmark.stdout"))
        .map_err(|error| format!("missing benchmark: {error}"))?;
    for phase in [
        "compile-end-to-end",
        "reader-validation",
        "artifact-encoding",
        "artifact-decoding",
        "probe-traversal",
        "declaration-growth",
        "concurrent-isolation",
    ] {
        if !output.contains(&format!("name={phase} ")) {
            return Err(format!("benchmark has no {phase} measurement"));
        }
    }
    let massif = fs::read_to_string(directory.join("massif.out"))
        .map_err(|error| format!("missing heap profile: {error}"))?;
    let memcheck = fs::read_to_string(directory.join("memcheck.stderr"))
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
fn validate_advisories(receipt: &Receipt, directory: &Path) -> Result<(), String> {
    let policy = read_workspace_text(&workspace_root()?, constants::DEPENDENCY_SOURCES_FILE)?;
    let mut locks = vec![constants::CARGO_LOCK_FILE.to_owned()];
    locks.extend(configuration_array_from(
        &policy,
        "",
        "isolated_tool_lockfiles",
    )?);
    for lock in locks {
        if !receipt.invocations.iter().any(|run| {
            run.arguments
                .windows(2)
                .any(|args| args == ["--file", lock.as_str()])
        }) {
            return Err(format!("advisory scan omitted {lock}"));
        }
    }
    let age = automation_value("quality", "advisory_max_age_seconds")?
        .parse::<u128>()
        .map_err(|error| format!("invalid advisory freshness: {error}"))?;
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
pub(super) fn advisory_scan() -> Result<(), String> {
    let root = workspace_root()?;
    let policy = read_workspace_text(&root, constants::DEPENDENCY_SOURCES_FILE)?;
    let mut locks = vec![constants::CARGO_LOCK_FILE.to_owned()];
    locks.extend(configuration_array_from(
        &policy,
        "",
        "isolated_tool_lockfiles",
    )?);
    let mut measurement = Measurement::begin("advisories")?;
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
#[path = "../tests/unit/quality_evidence.rs"]
mod tests;
