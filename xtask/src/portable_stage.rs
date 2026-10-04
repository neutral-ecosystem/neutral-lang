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
    let manifest: crate::configuration_models::ConformanceActivation =
        crate::configuration::parse(manifest, "conformance activation")?;
    let mut maximum = 0;
    for suite in manifest.suite {
        if suite.status == "required" {
            let stage = suite
                .active_from_stage
                .ok_or("required suite has no active_from_stage")?;
            maximum = maximum.max(stage);
        }
    }
    Ok(maximum)
}

#[cfg(test)]
#[path = "../tests/unit/portable_stage.rs"]
mod tests;
