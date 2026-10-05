// SPDX-License-Identifier: Apache-2.0

//! Typed generated metadata and result layout for binary release packages.

use crate::constants;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Reviewed source branch used for release candidates.
pub(crate) const CANDIDATE_REF: &str = "main";
/// Distribution channel represented by the assembled binary package.
const BINARY_CHANNEL: &str = "github-binaries";
/// State of a locally assembled, not yet published package.
const ASSEMBLED_STATUS: &str = "assembled";

/// Independent schema for generated release provenance and manifests.
pub(crate) const RELEASE_METADATA_SCHEMA: u32 = 1;

/// Exact build inputs recorded beside the assembled binaries.
#[derive(Serialize)]
pub(crate) struct BuildProvenance<'a> {
    /// Release metadata schema.
    pub(crate) schema_version: u32,
    /// Command responsible for building the package.
    pub(crate) builder: &'a str,
    /// Reviewed candidate branch.
    pub(crate) candidate_ref: &'a str,
    /// Exact candidate commit.
    pub(crate) candidate_commit: &'a str,
    /// Workspace-derived tag.
    pub(crate) release_tag: &'a str,
    /// Build host triple.
    pub(crate) target: &'a str,
    /// Selected compiler identity.
    pub(crate) rustc: String,
    /// Exact dependency lock digest.
    pub(crate) cargo_lock_sha256: String,
    /// Command for reproducing assembly.
    pub(crate) reproducible_command: &'a str,
}

/// One checksum-bound artifact, never a preformatted JSON fragment.
#[derive(Serialize)]
pub(crate) struct ReleaseArtifact {
    /// Package-relative filename.
    pub(crate) filename: String,
    /// Exact byte checksum.
    pub(crate) sha256: String,
    /// Project license expression.
    pub(crate) license: String,
    /// Package version that produced the bytes.
    pub(crate) producer_version: String,
    /// Exact candidate commit.
    pub(crate) source_commit: String,
    /// Selected distribution channel.
    pub(crate) channel: String,
}

/// Complete generated release manifest with typed artifact entries.
#[derive(Serialize)]
pub(crate) struct ReleaseManifest<'a> {
    /// Release metadata schema.
    pub(crate) schema_version: u32,
    /// Workspace-derived tag.
    pub(crate) release_tag: &'a str,
    /// Reviewed candidate branch.
    pub(crate) candidate_ref: &'a str,
    /// Exact candidate commit.
    pub(crate) candidate_commit: &'a str,
    /// Project license expression.
    pub(crate) license: &'a str,
    /// Targets included in this package.
    pub(crate) supported_targets: [&'a str; 1],
    /// Explicit registry authority.
    pub(crate) crates_io_selected: bool,
    /// Current package limitations.
    pub(crate) known_limitations: [&'static str; 2],
    /// Unselected capabilities.
    pub(crate) deferred: Vec<&'static str>,
    /// Exact artifact records in assembly order.
    pub(crate) artifacts: &'a [ReleaseArtifact],
}

/// Stable, typed shape of the generated package summary.
#[derive(Serialize)]
struct PackageSummary<'a> {
    /// Release tag derived from the workspace package version.
    release_tag: &'a str,
    /// Branch whose HEAD was qualified for the release.
    candidate_ref: &'static str,
    /// Exact source commit used to build the package.
    candidate_commit: &'a str,
    /// Rust target triple of the packaged binaries.
    host: &'a str,
    /// Selected publication channel.
    channel: &'static str,
    /// Local assembly state.
    status: &'static str,
}

/// Returns the release package directory beneath a validated results root.
pub(crate) fn package_output_directory(
    results_root: &Path,
    release_tag: &str,
    candidate_commit: &str,
    host: &str,
) -> PathBuf {
    results_root
        .join(constants::RELEASE_RESULT_DIRECTORY)
        .join(constants::RELEASE_PACKAGE_DIRECTORY)
        .join(release_tag)
        .join(candidate_commit)
        .join(host)
}

/// Serializes the package summary as deterministic, indented JSON.
pub(crate) fn package_summary_json(
    release_tag: &str,
    candidate_commit: &str,
    host: &str,
) -> Result<String, String> {
    let summary = PackageSummary {
        release_tag,
        candidate_ref: CANDIDATE_REF,
        candidate_commit,
        host,
        channel: BINARY_CHANNEL,
        status: ASSEMBLED_STATUS,
    };
    let mut json = serde_json::to_string_pretty(&summary)
        .map_err(|error| format!("could not serialize release package summary: {error}"))?;
    json.push('\n');
    Ok(json)
}
