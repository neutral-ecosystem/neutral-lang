// SPDX-License-Identifier: Apache-2.0

//! checks / `test_layout` responsibilities for repository automation.

use crate::{collect_regular_files, constants, fs, workspace_package_manifests, workspace_root};

/// Rejects inline Rust test bodies and requires path-based crate-local test modules.
pub(crate) fn check_test_layout() -> Result<(), String> {
    let root = workspace_root()?;
    let mut source_files = Vec::new();
    for manifest in workspace_package_manifests(&root)? {
        let source = manifest
            .parent()
            .ok_or_else(|| format!("workspace manifest has no parent: {}", manifest.display()))?
            .join("src");
        if source.is_dir() {
            collect_regular_files(&source, &mut source_files)?;
        }
    }
    let mut violations = Vec::new();
    for path in source_files {
        if path.extension().and_then(|value| value.to_str()) != Some("rs")
            || !path
                .components()
                .any(|component| component.as_os_str() == "src")
        {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if source_has_non_path_test_configuration(&content) {
            violations.push(
                path.strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    if violations.is_empty() {
        crate::output::pass("crate-local test layout");
        Ok(())
    } else {
        Err(format!(
            "production source contains inline or non-path test modules: {}",
            violations.join(", ")
        ))
    }
}

/// Returns whether any test-only source declaration lacks an immediate path attribute.
pub(crate) fn source_has_non_path_test_configuration(content: &str) -> bool {
    let lines = content.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        line.trim() == constants::TEST_CONFIGURATION_MARKER
            && lines[index + 1..]
                .iter()
                .find(|candidate| !candidate.trim().is_empty())
                .is_none_or(|candidate| {
                    !candidate
                        .trim()
                        .starts_with(constants::TEST_PATH_ATTRIBUTE_MARKER)
                })
    })
}
