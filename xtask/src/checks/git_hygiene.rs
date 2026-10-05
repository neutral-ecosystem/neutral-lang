// SPDX-License-Identifier: Apache-2.0

//! Git-backed tracking checks reuse ignore policy rather than duplicating glob matching.

use crate::constants;
use std::{path::Path, process::Command};

/// Rejects force-added generated/local files even when their paths are new to repository tooling.
pub(crate) fn verify_no_ignored_tracked_files(root: &Path) -> Result<(), String> {
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "ls-files",
            "--cached",
            "--ignored",
            "--exclude-standard",
            "-z",
        ])
        .output()
        .map_err(|error| format!("could not inspect ignored tracked files: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not inspect ignored tracked files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let paths = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git emitted non-UTF-8 paths: {error}"))?;
    let paths = paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "ignored files are tracked: {paths:?}; remove generated/local files from Git tracking, or explicitly document and unignore intentional inputs"
        ))
    }
}

#[cfg(test)]
#[path = "../../tests/unit/git_hygiene.rs"]
mod tests;
