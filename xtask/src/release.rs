// SPDX-License-Identifier: Apache-2.0

//! Typed, fail-closed parsing for the explicit release-authority selection.

use super::constants;
use super::{configuration, configuration_models::Release};
use std::{collections::BTreeSet, fs, path::Path, process::Command};

/// Requires the release candidate to contain its approved baseline in Git history.
///
/// Later commits are qualified by the full release-quality workflow before packaging.
pub(crate) fn verify_approval_lineage(
    root: &Path,
    approved_commit: &str,
    candidate_commit: &str,
) -> Result<(), String> {
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "merge-base",
            "--is-ancestor",
            approved_commit,
            candidate_commit,
        ])
        .output()
        .map_err(|error| format!("could not verify release approval ancestry: {error}"))?;
    match output.status.code() {
        Some(0) => Ok(()),
        Some(1) => Err(format!(
            "release main HEAD {candidate_commit} does not descend from approved candidate {approved_commit}; evaluate and approve the intended release lineage"
        )),
        _ => Err(format!(
            "could not verify release approval ancestry: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

/// One explicitly selected release distribution channel.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DistributionChannel {
    /// Annotated source tag.
    SourceTag,
    /// GitHub-hosted binary assets.
    GithubBinaries,
    /// crates.io package publication.
    CratesIo,
}

/// One reviewed distribution selection for the `main`-head release candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleasePlan {
    /// Release tag that will name the publication after qualification completes.
    pub(crate) release_tag: String,
    /// Explicitly selected distribution channels.
    pub(crate) channels: BTreeSet<DistributionChannel>,
    /// Binary package names selected for GitHub distribution.
    pub(crate) binaries: Vec<String>,
}

impl ReleasePlan {
    /// Reads release scope and derives its eventual publication tag from the workspace version.
    pub(crate) fn read(path: &Path, package_version: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let config: Release = configuration::parse(&content, "release configuration")?;
        configuration::require_schema(config.schema_version, "release configuration")?;
        let mut channels = BTreeSet::new();
        if config.distribution.source_tag {
            channels.insert(DistributionChannel::SourceTag);
        }
        if config.distribution.github_binaries {
            channels.insert(DistributionChannel::GithubBinaries);
        }
        if config.distribution.crates_io {
            channels.insert(DistributionChannel::CratesIo);
        }
        let plan = Self {
            release_tag: format!("v{package_version}"),
            channels,
            binaries: config.distribution.binaries,
        };
        plan.validate()?;
        Ok(plan)
    }

    /// Rejects incomplete or malformed release-authority selections.
    fn validate(&self) -> Result<(), String> {
        if !self.release_tag.starts_with('v') || self.release_tag.chars().any(char::is_whitespace) {
            return Err("release tag must be a whitespace-free v-prefixed tag".to_owned());
        }
        if self.channels.is_empty() {
            return Err("at least one release distribution channel must be selected".to_owned());
        }
        if self.channels.contains(&DistributionChannel::GithubBinaries)
            && !self.channels.contains(&DistributionChannel::SourceTag)
        {
            return Err("GitHub binary distribution requires a source tag".to_owned());
        }
        if self.channels.contains(&DistributionChannel::GithubBinaries) && self.binaries.is_empty()
        {
            return Err("GitHub binary distribution requires at least one binary".to_owned());
        }
        if self.binaries.iter().any(|binary| {
            binary.is_empty()
                || !binary
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        }) {
            return Err("release binary names must be plain ASCII package names".to_owned());
        }
        if self.binaries.iter().collect::<BTreeSet<_>>().len() != self.binaries.len() {
            return Err("release binary names must be unique".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/release.rs"]
mod tests;
