// SPDX-License-Identifier: Apache-2.0

//! Ignored generated-result root and safe local cleanup.

use super::{
    Command, PathBuf, automation_value, constants, env, fs, is_safe_relative_path, quality_value,
    workspace_root,
};

/// Returns the configured result root after rejecting unsafe paths.
pub(super) fn result_root() -> Result<PathBuf, String> {
    let configured = match env::var(constants::TEST_RESULTS_ENV) {
        Ok(path) => path,
        Err(env::VarError::NotPresent) => automation_value("output", "results_root")?,
        Err(error) => return Err(format!("invalid {}: {error}", constants::TEST_RESULTS_ENV)),
    };
    let path = PathBuf::from(&configured);
    if !is_safe_result_path(&path) {
        return Err(format!(
            "{} must be a relative path beneath the workspace",
            constants::TEST_RESULTS_ENV
        ));
    }
    let workspace = workspace_root()?;
    let mut current = workspace.clone();
    for component in path.components() {
        current.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&current)
            && metadata.file_type().is_symlink()
        {
            return Err(format!(
                "generated-results path contains a symlink: {}",
                current.display()
            ));
        }
    }
    let probe = path.join(".neutral-results-check");
    let ignored = Command::new(constants::GIT_COMMAND)
        .current_dir(&workspace)
        .args(["check-ignore", "--no-index", "-q", "--"])
        .arg(&probe)
        .status()
        .map_err(|error| format!("could not verify generated-results ignore policy: {error}"))?;
    if !ignored.success() {
        return Err(format!(
            "generated-results root {configured} is not ignored; add it to .gitignore first"
        ));
    }
    Ok(workspace.join(path))
}

/// Resolves one quality-configured generated file below the selected result root.
pub(super) fn quality_output_path(section: &str, key: &str) -> Result<PathBuf, String> {
    let relative = PathBuf::from(quality_value(section, key)?);
    if !is_safe_result_path(&relative) {
        return Err(format!(
            "quality [{section}] {key} must be a safe relative result path"
        ));
    }
    Ok(result_root()?.join(relative))
}

/// Returns whether a configured result path cannot name the workspace or escape it.
pub(super) fn is_safe_result_path(path: &std::path::Path) -> bool {
    is_safe_relative_path(path) && !path.starts_with("target")
}

/// Removes only the configured generated-result root after validating its path.
pub(super) fn clean_results() -> Result<(), String> {
    let root = result_root()?;
    if root.exists() {
        if !root.is_dir() {
            return Err(format!(
                "generated-results root is not a directory: {}",
                root.display()
            ));
        }
        fs::remove_dir_all(&root)
            .map_err(|error| format!("could not remove {}: {error}", root.display()))?;
    }
    crate::output::info("generated results cleaned");
    Ok(())
}
