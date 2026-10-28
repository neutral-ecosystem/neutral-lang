// SPDX-License-Identifier: Apache-2.0

//! Typed, commit-bound release evaluations and immutable quality approvals.
//!
//! Records are parsed once, checked against their document schema, and compared
//! with current policy digests before approval. Existing approvals are never
//! rewritten; only the human-readable ledger is regenerated from retained records.

use crate::constants::flags;
use crate::{
    Path, PathBuf, QualityProfile, SystemTime, UNIX_EPOCH, check_quality_inventory,
    collect_named_files, command_output, constants, fs, html_spdx_marker, is_sha256,
    line_spdx_marker, project_license, quality, quality_evidence, quality_profile_name,
    read_workspace_text, require_clean_checkout, require_main_head_checkout, result_root,
    rustc_command, sha256_file, validate_semver, verify_release_approval,
    workspace_package_version, workspace_root,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Schema of quality evaluations and approval records, independent of package versions.
const QUALITY_RECORD_SCHEMA_VERSION: u32 = 1;

/// One immutable release-quality record.
#[derive(Debug, Deserialize, Serialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualityApproval {
    /// Exact record schema.
    pub(crate) schema_version: u32,
    /// Release identifier with its `v` prefix.
    pub(crate) release: String,
    /// Exact approved Git commit.
    pub(crate) commit: String,
    /// Approval state.
    pub(crate) status: String,
    /// UTC-independent Unix timestamp or retained historical date.
    pub(crate) approved_at: String,
    /// Evaluation identifier that supported the approval.
    pub(crate) evaluation: String,
    /// SHA-256 of the quality-gate configuration used for evaluation.
    pub(crate) quality_gates_sha256: String,
}

/// Source and policy authority retained after a successful quality evaluation.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct QualityEvaluation {
    /// Exact record schema.
    schema_version: u32,
    /// Evaluated checkout.
    commit: String,
    /// Selected quality profile.
    profile: String,
    /// Evaluation outcome.
    status: String,
    /// Compiler identity reported during evaluation.
    toolchain: String,
    /// Exact inventory policy bytes.
    quality_manifest_sha256: String,
    /// Exact measurement policy bytes.
    quality_gates_sha256: String,
}

/// Serializes TOML with the owning project's license marker and no manual string escaping.
fn quality_record_toml(record: &impl Serialize, license: &str) -> Result<String, String> {
    let body = toml::to_string_pretty(record)
        .map_err(|error| format!("could not serialize quality record: {error}"))?;
    Ok(format!("{}\n\n{body}", line_spdx_marker(license)))
}

/// Rejects unknown document revisions before using their release authority.
fn check_record_schema(schema: u32) -> Result<(), String> {
    if schema != QUALITY_RECORD_SCHEMA_VERSION {
        return Err(format!("unsupported quality record schema: {schema}"));
    }
    Ok(())
}

/// Runs a quality profile and writes a deterministic commit-bound evaluation.
pub(crate) fn evaluate_quality(profile: QualityProfile) -> Result<(), String> {
    let commit = require_clean_checkout()?;
    quality(profile)?;
    let root = workspace_root()?;
    let profile_name = quality_profile_name(profile);
    let output = result_root()?
        .join(constants::QUALITY_EVALUATION_DIRECTORY)
        .join(&commit)
        .join(format!("{profile_name}.toml"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let manifest_sha256 = sha256_file(&root.join(constants::QUALITY_MANIFEST_FILE))?;
    let quality_gates_sha256 = sha256_file(&root.join(constants::QUALITY_GATES_FILE))?;
    let toolchain = command_output(&rustc_command()?, &[flags::VERSION])?;
    let record = QualityEvaluation {
        schema_version: QUALITY_RECORD_SCHEMA_VERSION,
        commit,
        profile: profile_name.to_owned(),
        status: "pass".to_owned(),
        toolchain,
        quality_manifest_sha256: manifest_sha256,
        quality_gates_sha256,
    };
    fs::write(
        &output,
        quality_record_toml(&record, &project_license(&root)?)?,
    )
    .map_err(|error| format!("could not write {}: {error}", output.display()))?;
    crate::output::file("quality evaluation", &output);
    Ok(())
}

/// Approves a passing release evaluation for the clean checked-out `main` HEAD.
pub(crate) fn approve_quality_release(release: &str) -> Result<(), String> {
    let version = release.strip_prefix('v').unwrap_or(release);
    validate_semver(version)?;
    let root = workspace_root()?;
    let workspace_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    if version != workspace_version {
        return Err(format!(
            "quality approval release {version} does not match workspace version {workspace_version}"
        ));
    }
    let commit = require_main_head_checkout()?;
    let evaluation_relative = PathBuf::from(constants::QUALITY_EVALUATION_DIRECTORY)
        .join(&commit)
        .join("release.toml");
    let evaluation = result_root()?.join(&evaluation_relative);
    let evaluation_content = fs::read_to_string(&evaluation).map_err(|error| {
        format!(
            "passing release evaluation is missing at {}: {error}; run `cargo xtask quality evaluate --profile release`",
            evaluation.display()
        )
    })?;
    let evaluated: QualityEvaluation =
        crate::configuration::parse(&evaluation_content, &evaluation.display().to_string())?;
    check_record_schema(evaluated.schema_version)?;
    if evaluated.commit != commit || evaluated.profile != "release" || evaluated.status != "pass" {
        return Err("release evaluation does not identify a passing current HEAD".to_owned());
    }
    if evaluated.quality_manifest_sha256
        != sha256_file(&root.join(constants::QUALITY_MANIFEST_FILE))?
        || evaluated.quality_gates_sha256 != sha256_file(&root.join(constants::QUALITY_GATES_FILE))?
    {
        return Err(
            "quality policy changed after evaluation; rerun the release evaluation".to_owned(),
        );
    }
    let evidence_directory = root
        .join(constants::QUALITY_EVIDENCE_DIRECTORY)
        .join(format!("v{version}"));
    if !evidence_directory.join("README.md").is_file() {
        return Err(format!(
            "release evidence directory is not prepared: {}",
            evidence_directory.display()
        ));
    }
    let approved_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))?
        .as_secs();
    let record = evidence_directory.join("record.toml");
    if record.exists() {
        verify_release_approval()?;
        quality_evidence::retain()?;
        crate::output::info(format!(
            "retained current measurement evidence; existing approval remains immutable: {}",
            record.display()
        ));
        return Ok(());
    }
    quality_evidence::retain()?;
    let approval = QualityApproval {
        schema_version: QUALITY_RECORD_SCHEMA_VERSION,
        release: format!("v{version}"),
        commit,
        status: "approved".to_owned(),
        approved_at: approved_at.to_string(),
        evaluation: evaluation_relative.to_string_lossy().into_owned(),
        quality_gates_sha256: evaluated.quality_gates_sha256,
    };
    fs::write(
        &record,
        quality_record_toml(&approval, &project_license(&root)?)?,
    )
    .map_err(|error| format!("could not write {}: {error}", record.display()))?;
    render_quality_status()?;
    crate::output::pass(format!("quality release v{version}"));
    Ok(())
}

/// Prints the current approved-release quality ledger.
pub(crate) fn quality_status() -> Result<(), String> {
    let approvals = read_quality_approvals()?;
    if approvals.is_empty() {
        crate::output::info("no approved quality releases");
    }
    for approval in approvals {
        crate::output::info(format!("{} {}", approval.release, approval.status));
    }
    Ok(())
}

/// Regenerates the checked-in human-readable quality status document.
pub(crate) fn render_quality_status() -> Result<(), String> {
    let approvals = read_quality_approvals()?;
    let root = workspace_root()?;
    let rendered = quality_status_markdown(&approvals, &project_license(&root)?)?;
    let path = root.join(constants::QUALITY_STATUS_FILE);
    fs::write(&path, rendered)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    crate::output::file("quality status", &path);
    Ok(())
}

/// Verifies release records and generated status documentation.
pub(crate) fn verify_quality_ledger() -> Result<(), String> {
    check_quality_inventory()?;
    let root = workspace_root()?;
    let approvals = read_quality_approvals()?;
    for approval in &approvals {
        if approval.status != "approved"
            || approval.approved_at.is_empty()
            || approval.evaluation.is_empty()
            || !is_sha256(&approval.quality_gates_sha256)
            || approval.commit.len() != 40
            || !approval
                .commit
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(format!(
                "quality approval v{} is incomplete or invalid",
                approval.release
            ));
        }
        let version = approval.release.strip_prefix('v').ok_or_else(|| {
            format!(
                "quality approval release must start with v: {}",
                approval.release
            )
        })?;
        validate_semver(version)?;
    }
    let expected = quality_status_markdown(&approvals, &project_license(&root)?)?;
    let status = read_workspace_text(&root, constants::QUALITY_STATUS_FILE)?;
    if status != expected {
        return Err(
            "quality status documentation is stale; run `cargo xtask quality render`".to_owned(),
        );
    }
    crate::output::pass("quality approval ledger");
    Ok(())
}

/// Reads every immutable release-quality record in release order.
pub(crate) fn read_quality_approvals() -> Result<Vec<QualityApproval>, String> {
    let root = workspace_root()?;
    let mut records = Vec::new();
    collect_named_files(
        &root.join(constants::QUALITY_EVIDENCE_DIRECTORY),
        "record.toml",
        &mut records,
    )?;
    let mut approvals = Vec::new();
    for path in records {
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let approval: QualityApproval =
            crate::configuration::parse(&content, &path.display().to_string())?;
        check_record_schema(approval.schema_version)?;
        let directory_release = path
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| {
                format!(
                    "quality approval has no release directory: {}",
                    path.display()
                )
            })?;
        if approval.release != directory_release {
            return Err(format!(
                "quality approval release {} does not match directory {directory_release}",
                approval.release
            ));
        }
        approvals.push(approval);
    }
    approvals.sort();
    Ok(approvals)
}

/// Renders the deterministic human-readable release-quality table.
pub(crate) fn quality_status_markdown(
    approvals: &[QualityApproval],
    license: &str,
) -> Result<String, String> {
    let mut output = format!(
        "{}\n<!-- Generated by `cargo xtask quality render`; do not edit. -->\n\n# Release quality status\n\n| Release | Status | Approved on (UTC, DD-MM-YYYY) |\n| --- | --- | --- |\n",
        html_spdx_marker(license)
    );
    for approval in approvals {
        let date = quality_approval_date(&approval.approved_at)?;
        writeln!(
            &mut output,
            "| `v{}` | {} | `{}` |",
            approval
                .release
                .strip_prefix('v')
                .unwrap_or(&approval.release),
            approval.status,
            date
        )
        .expect("writing to a String cannot fail");
    }
    output.push_str("\nRelease records are verified by `cargo xtask quality verify`.\n");
    Ok(output)
}

/// Formats Unix-second or historical ISO approval dates as UTC day-month-year dates.
pub(crate) fn quality_approval_date(value: &str) -> Result<String, String> {
    if value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return Ok(format!(
            "{}-{}-{}",
            &value[8..10],
            &value[5..7],
            &value[..4]
        ));
    }
    let seconds = value
        .parse::<u64>()
        .map_err(|_| format!("invalid quality approval timestamp: {value}"))?;
    let days = i64::try_from(seconds / 86_400)
        .map_err(|_| format!("quality approval timestamp is out of range: {value}"))?;
    let day = days
        .checked_add(719_468)
        .ok_or_else(|| format!("quality approval timestamp is out of range: {value}"))?;
    let era = day / 146_097;
    let day_of_era = day - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_piece = (5 * day_of_year + 2) / 153;
    let day_of_month = day_of_year - (153 * month_piece + 2) / 5 + 1;
    let month = month_piece + if month_piece < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    if year > 9_999 {
        return Err(format!(
            "quality approval timestamp is out of range: {value}"
        ));
    }
    Ok(format!("{day_of_month:02}-{month:02}-{year:04}"))
}

#[cfg(test)]
#[path = "../../tests/unit/quality_ledger.rs"]
mod tests;
