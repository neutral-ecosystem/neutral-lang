// SPDX-License-Identifier: Apache-2.0

//! Conformance-suite activation derived from the portable manifest.

use std::{fs, path::Path};

use crate::constants;

/// Derives the highest active suite stage, ignoring planned suites and future cases.
pub(crate) fn active_conformance_stage(root: &Path) -> Result<u8, String> {
    let manifest = root.join(constants::PORTABLE_CONFORMANCE_MANIFEST_FILE);
    if !manifest.is_file() {
        return Ok(0);
    }
    let content = fs::read_to_string(&manifest)
        .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
    parse_active_stage(&content)
}

/// Parses only suite-level activation, never case-level future activation.
fn parse_active_stage(manifest: &str) -> Result<u8, String> {
    let mut maximum = 0;
    let mut in_suite = false;
    let mut stage = None;
    let mut required = false;
    for line in manifest.lines().map(str::trim) {
        if line == "[[suite]]" || line == "[[case]]" {
            if in_suite && required {
                maximum = maximum.max(stage.ok_or("required suite has no active_from_stage")?);
            }
            in_suite = line == "[[suite]]";
            stage = None;
            required = false;
            continue;
        }
        if !in_suite {
            continue;
        }
        if let Some(value) = line.strip_prefix("active_from_stage =") {
            stage = Some(
                value
                    .trim()
                    .parse::<u8>()
                    .map_err(|error| format!("invalid active_from_stage: {error}"))?,
            );
        } else if line == "status = \"required\"" {
            required = true;
        }
    }
    if in_suite && required {
        maximum = maximum.max(stage.ok_or("required suite has no active_from_stage")?);
    }
    Ok(maximum)
}

#[cfg(test)]
#[path = "../tests/unit/portable_stage.rs"]
mod tests;
