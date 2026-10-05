// SPDX-License-Identifier: Apache-2.0

//! Runtime discovery of the repository owning the invocation.

use crate::constants;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Locates repository automation from the invocation directory, including nested tool workspaces.
pub(crate) fn discover(start: &Path) -> Result<PathBuf, String> {
    let start = fs::canonicalize(start)
        .map_err(|error| format!("could not resolve invocation directory: {error}"))?;
    for directory in start.ancestors() {
        if !directory.join(constants::AUTOMATION_CONFIG_FILE).is_file() {
            continue;
        }
        let manifest = directory.join(constants::WORKSPACE_MANIFEST_FILE);
        let content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        let parsed: toml::Table = crate::configuration::parse(&content, "workspace manifest")?;
        if parsed.get("workspace").is_some_and(toml::Value::is_table) {
            return Ok(directory.to_path_buf());
        }
    }
    Err(format!(
        "no repository automation workspace contains {}",
        start.display()
    ))
}

#[cfg(test)]
#[path = "../../tests/unit/workspace.rs"]
mod tests;
