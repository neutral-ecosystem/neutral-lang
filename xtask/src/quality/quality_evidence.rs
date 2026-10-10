// SPDX-License-Identifier: Apache-2.0

//! Coordinate source-bound measurement retention and evidence verification.
//!
//! Receipts bind tool invocations and report hashes to repository inputs and
//! quality policy. Child modules own fingerprinting, execution and report
//! acceptance; the measurement store isolates runs under exclusive ownership.
//! Generated reports remain
//! local evidence, not source files or manually asserted passing results.

mod inputs;
mod measurement;
mod validation;
pub(crate) use inputs::input_digest;
pub(crate) use measurement::Measurement;
use validation::{read_receipt, validate_reports, verify};

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

/// Accepts only one safe workspace-relative filename.
fn plain_filename(value: &str) -> bool {
    is_safe_relative_path(Path::new(value)) && Path::new(value).components().count() == 1
}

/// Resolves the configurable ignored measurement root.
fn generated_root(settings: &QualitySettings) -> Result<PathBuf, String> {
    Ok(result_root()?.join(&settings.evidence_root))
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
