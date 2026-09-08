// SPDX-License-Identifier: Apache-2.0

//! Typed, fail-closed parsing for the explicit release-authority selection.

use std::{collections::BTreeSet, fs, path::Path};

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

/// One reviewed release-candidate and distribution selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReleasePlan {
    /// Annotated Git tag naming the selected release source.
    pub(crate) candidate_tag: String,
    /// Explicitly selected distribution channels.
    pub(crate) channels: BTreeSet<DistributionChannel>,
    /// Binary package names selected for GitHub distribution.
    pub(crate) binaries: Vec<String>,
}

impl ReleasePlan {
    /// Reads release scope and derives its tag from the workspace version.
    pub(crate) fn read(path: &Path, package_version: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if !required_bool(&content, "stage9_residual_risk")? {
            return Err("Stage 9 residual-risk approval is required".to_owned());
        }
        let mut channels = BTreeSet::new();
        if required_bool(&content, "source_tag")? {
            channels.insert(DistributionChannel::SourceTag);
        }
        if required_bool(&content, "github_binaries")? {
            channels.insert(DistributionChannel::GithubBinaries);
        }
        if required_bool(&content, "crates_io")? {
            channels.insert(DistributionChannel::CratesIo);
        }
        let plan = Self {
            candidate_tag: format!("v{package_version}"),
            channels,
            binaries: required_array(&content, "binaries")?,
        };
        plan.validate()?;
        Ok(plan)
    }

    /// Rejects incomplete or malformed release-authority selections.
    fn validate(&self) -> Result<(), String> {
        if !self.candidate_tag.starts_with('v')
            || self.candidate_tag.chars().any(char::is_whitespace)
        {
            return Err(
                "release candidate_tag must be a whitespace-free v-prefixed tag".to_owned(),
            );
        }
        if self.channels.is_empty() {
            return Err("at least one release distribution channel must be selected".to_owned());
        }
        if self.channels.contains(&DistributionChannel::GithubBinaries) && self.binaries.is_empty()
        {
            return Err("GitHub binary distribution requires at least one binary".to_owned());
        }
        if self.binaries.iter().any(|binary| {
            binary.is_empty()
                || binary.contains('/')
                || binary.contains('\\')
                || binary == "."
                || binary == ".."
        }) {
            return Err("release binary names must be plain package names".to_owned());
        }
        Ok(())
    }
}

/// Reads one required Boolean from the constrained release TOML.
fn required_bool(content: &str, key: &str) -> Result<bool, String> {
    match required_value(content, key)? {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("release {key} must be true or false")),
    }
}

/// Reads one required quoted-string array from the constrained release TOML.
fn required_array(content: &str, key: &str) -> Result<Vec<String>, String> {
    let value = required_value(content, key)?;
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("release {key} must be an array"))?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(str::trim)
        .map(|entry| {
            entry
                .strip_prefix('"')
                .and_then(|entry| entry.strip_suffix('"'))
                .map(str::to_owned)
                .ok_or_else(|| format!("release {key} entries must be quoted strings"))
        })
        .collect()
}

/// Finds one exact key in the constrained release TOML.
fn required_value<'a>(content: &'a str, key: &str) -> Result<&'a str, String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('['))
        .find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            (candidate.trim() == key).then_some(value.trim())
        })
        .ok_or_else(|| format!("release configuration has no {key}"))
}

#[cfg(test)]
#[path = "../tests/unit/release.rs"]
mod tests;
