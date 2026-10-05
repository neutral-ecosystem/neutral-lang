// SPDX-License-Identifier: Apache-2.0

//! Typed generated metadata and result layout for binary release packages.

use crate::constants;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Reviewed source branch used for release candidates.
const CANDIDATE_REF: &str = "main";
/// Distribution channel represented by the assembled binary package.
const BINARY_CHANNEL: &str = "github-binaries";
/// State of a locally assembled, not yet published package.
const ASSEMBLED_STATUS: &str = "assembled";

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
