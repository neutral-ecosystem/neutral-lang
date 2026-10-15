// SPDX-License-Identifier: Apache-2.0

//! Decide whether retained reports satisfy their configured quality gate.
//!
//! Verify receipt bindings and exact report hashes, then interpret native
//! coverage, mutation, fuzz, performance or advisory results against policy.
//! A successful tool exit or a receipt alone is insufficient: required reports
//! and their measured outcomes must also pass the gate-specific checks.

use super::{
    BTreeSet, Path, QualityGate, QualitySettings, Receipt, is_sha256, measurement_store,
    plain_filename, reports, sha256_file, unix_time_millis,
};
use std::fs;

/// Loads one strict receipt schema.
pub(super) fn read_receipt(directory: &Path) -> Result<Receipt, String> {
    serde_json::from_slice(
        &fs::read(directory.join(reports::RECEIPT))
            .map_err(|error| format!("{}: {error}", directory.display()))?,
    )
    .map_err(|error| format!("invalid quality receipt: {error}"))
}

/// Rejects wrong-source, wrong-policy, modified, or incomplete measurements.
pub(super) fn verify(
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
pub(super) fn validate_reports(
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
                        | "--iterate"
                        | "--in-diff"
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
    validate_phase_times(
        &output,
        &settings.policy.performance.maximum_ns_per_iteration,
    )?;
    let massif = fs::read_to_string(directory.join(reports::MASSIF))
        .map_err(|error| format!("missing heap profile: {error}"))?;
    let memcheck = fs::read_to_string(directory.join(reports::MEMCHECK_STDERR))
        .map_err(|error| format!("missing allocation review: {error}"))?;
    let peak = massif
        .lines()
        .filter_map(|line| line.strip_prefix("mem_heap_B="))
        .try_fold(None, |peak: Option<u64>, value| {
            let bytes = value
                .parse::<u64>()
                .map_err(|_| "invalid Massif heap sample".to_owned())?;
            Ok::<_, String>(Some(peak.map_or(bytes, |previous| previous.max(bytes))))
        })?
        .ok_or("performance has no heap samples")?;
    if peak > settings.policy.performance.maximum_heap_bytes {
        return Err(format!(
            "peak heap {peak} exceeds {} bytes",
            settings.policy.performance.maximum_heap_bytes
        ));
    }
    if !memcheck.contains("ERROR SUMMARY: 0 errors")
        || !receipt.invocations.iter().any(|run| {
            run.report == "memcheck" && run.arguments.contains(&"--error-exitcode=1".to_owned())
        })
    {
        return Err("performance has no successful heap/allocation review".to_owned());
    }
    Ok(())
}

/// Parses exact native phase records; every required operation must pass once, without averaging away failures.
fn validate_phase_times(
    output: &str,
    budgets: &std::collections::BTreeMap<String, u64>,
) -> Result<(), String> {
    let mut observed = BTreeSet::new();
    for line in output.lines() {
        let mut words = line.split_whitespace();
        if words.next() != Some("PASS") || words.next() != Some("performance") {
            continue;
        }
        let field = words.next().ok_or("malformed performance record")?;
        if field.starts_with("artifact-bytes=") {
            continue;
        }
        let phase = field
            .strip_prefix("name=")
            .ok_or("malformed performance record")?;
        let maximum = budgets
            .get(phase)
            .ok_or_else(|| format!("unconfigured performance phase {phase}"))?;
        let iterations = words
            .next()
            .and_then(|word| word.strip_prefix("iterations="))
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("invalid iterations for {phase}"))?;
        let elapsed = words
            .next()
            .and_then(|word| word.strip_prefix("elapsed-ns="))
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| format!("invalid elapsed time for {phase}"))?;
        if words.next().is_some() || !observed.insert(phase) {
            return Err(format!("duplicate or malformed performance phase {phase}"));
        }
        if u128::from(elapsed) > u128::from(*maximum) * u128::from(iterations) {
            return Err(format!(
                "performance {phase} exceeded {maximum} ns/iteration ({elapsed} ns / {iterations})"
            ));
        }
    }
    for phase in budgets.keys() {
        if !observed.contains(phase.as_str()) {
            return Err(format!("benchmark has no {phase} measurement"));
        }
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
