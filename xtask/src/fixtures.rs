// SPDX-License-Identifier: Apache-2.0

//! Automated synchronization and verification of test fixtures, oracles, and freeze hashes.
//!
//! This module eliminates the manual burden of calculating SHA-256 digests across
//! `portable/conformance/manifest.toml` and `portable/specs/contracts/freeze.toml`.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

/// Summary of synchronization operations performed across manifests and freeze contracts.
#[derive(Default)]
struct SyncSummary {
    /// Number of fixture cases verified.
    fixture_count: usize,
    /// Number of oracle cases verified.
    oracle_count: usize,
    /// Number of digest changes made to the conformance manifest.
    manifest_changes: usize,
    /// Number of digest changes made to the contract freeze manifest.
    freeze_changes: usize,
    /// Mismatch errors encountered during check mode.
    mismatch_errors: Vec<String>,
}

/// Synchronizes or verifies all test fixtures, oracles, and freeze hashes in the repository.
pub(crate) fn sync_fixtures(workspace_root: &Path, check_only: bool) -> Result<(), String> {
    let mode = if check_only {
        "verifying"
    } else {
        "synchronizing"
    };
    println!("[info] fixtures: {mode} fixture hashes and contract freeze...");

    let manifest_rel = "portable/conformance/manifest.toml";
    let freeze_rel = "portable/specs/contracts/freeze.toml";

    let manifest_path = workspace_root.join(manifest_rel);
    let freeze_path = workspace_root.join(freeze_rel);

    if !manifest_path.is_file() {
        return Err(format!(
            "manifest file not found: {}",
            manifest_path.display()
        ));
    }
    if !freeze_path.is_file() {
        return Err(format!("freeze file not found: {}", freeze_path.display()));
    }

    let mut summary = SyncSummary::default();

    let registered_fixtures =
        sync_manifest(workspace_root, &manifest_path, check_only, &mut summary)?;
    sync_freeze(workspace_root, &freeze_path, check_only, &mut summary)?;
    discover_untracked_fixtures(workspace_root, &registered_fixtures)?;

    if check_only && !summary.mismatch_errors.is_empty() {
        return Err(format!(
            "fixture verification failed with {} mismatch(es):\n{}",
            summary.mismatch_errors.len(),
            summary.mismatch_errors.join("\n\n")
        ));
    }

    println!(
        "[info] fixtures: verified {} fixtures, {} oracles. (manifest updates: {}, freeze updates: {})",
        summary.fixture_count,
        summary.oracle_count,
        summary.manifest_changes,
        summary.freeze_changes
    );

    Ok(())
}

/// Synchronizes the conformance manifest and returns the set of registered fixture paths.
fn sync_manifest(
    workspace_root: &Path,
    manifest_path: &Path,
    check_only: bool,
    summary: &mut SyncSummary,
) -> Result<BTreeSet<String>, String> {
    let manifest_raw = fs::read_to_string(manifest_path)
        .map_err(|e| format!("could not read {}: {e}", manifest_path.display()))?;

    let mut registered_fixtures = BTreeSet::new();
    let mut updated_lines = Vec::new();
    let mut current_fixture: Option<String> = None;
    let mut current_oracle: Option<String> = None;

    for line in manifest_raw.lines() {
        let trimmed = line.trim();

        if trimmed == "[[case]]" || trimmed == "[[suite]]" {
            current_fixture = None;
            current_oracle = None;
            updated_lines.push(line.to_string());
            continue;
        }

        if let Some(path) = trimmed
            .strip_prefix("fixture =")
            .and_then(|s| extract_string_value(s.trim()))
        {
            registered_fixtures.insert(path.clone());
            current_fixture = Some(path);
            updated_lines.push(line.to_string());
            continue;
        }

        if let Some(path) = trimmed
            .strip_prefix("oracle =")
            .and_then(|s| extract_string_value(s.trim()))
        {
            current_oracle = Some(path);
            updated_lines.push(line.to_string());
            continue;
        }

        if trimmed.starts_with("fixture_sha256 =")
            && let Some(ref rel_path) = current_fixture
        {
            let new_line = process_digest_line(
                workspace_root,
                line,
                trimmed,
                rel_path,
                "fixture",
                check_only,
                &mut summary.fixture_count,
                &mut summary.manifest_changes,
                &mut summary.mismatch_errors,
            )?;
            updated_lines.push(new_line);
            continue;
        }

        if trimmed.starts_with("oracle_sha256 =")
            && let Some(ref rel_path) = current_oracle
        {
            let new_line = process_digest_line(
                workspace_root,
                line,
                trimmed,
                rel_path,
                "oracle",
                check_only,
                &mut summary.oracle_count,
                &mut summary.manifest_changes,
                &mut summary.mismatch_errors,
            )?;
            updated_lines.push(new_line);
            continue;
        }

        updated_lines.push(line.to_string());
    }

    if !check_only && summary.manifest_changes > 0 {
        let mut new_content = updated_lines.join("\n");
        if manifest_raw.ends_with('\n') {
            new_content.push('\n');
        }
        fs::write(manifest_path, new_content)
            .map_err(|e| format!("could not write {}: {e}", manifest_path.display()))?;
        println!(
            "[info] fixtures: updated {} digests in {}",
            summary.manifest_changes,
            manifest_path.display()
        );
    }

    Ok(registered_fixtures)
}

/// Processes a single digest line, computing its hash and checking for mismatches.
#[allow(clippy::too_many_arguments)]
fn process_digest_line(
    workspace_root: &Path,
    original_line: &str,
    trimmed: &str,
    rel_path: &str,
    kind: &str,
    check_only: bool,
    counter: &mut usize,
    change_counter: &mut usize,
    mismatches: &mut Vec<String>,
) -> Result<String, String> {
    let full_path = workspace_root.join(rel_path);
    let actual_digest = hash_file(&full_path)?;
    let existing_digest = extract_string_value(trimmed).unwrap_or_default();
    *counter += 1;

    if actual_digest != existing_digest {
        *change_counter += 1;
        if check_only {
            mismatches.push(format!(
                "{kind} `{rel_path}` digest mismatch:\n  expected: {existing_digest}\n  actual:   {actual_digest}"
            ));
        }
    }

    let key = if kind == "fixture" {
        "fixture_sha256"
    } else {
        "oracle_sha256"
    };
    let indent = original_line
        .find(key.chars().next().unwrap_or(' '))
        .unwrap_or(0);
    Ok(format!("{:indent$}{key} = \"{actual_digest}\"", ""))
}

/// Synchronizes the contract freeze manifest digests.
fn sync_freeze(
    workspace_root: &Path,
    freeze_path: &Path,
    check_only: bool,
    summary: &mut SyncSummary,
) -> Result<(), String> {
    let freeze_raw = fs::read_to_string(freeze_path)
        .map_err(|e| format!("could not read {}: {e}", freeze_path.display()))?;

    let mut updated_lines = Vec::new();
    let mut last_path_entry: Option<(String, String)> = None;

    for line in freeze_raw.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
            if trimmed.starts_with('[') {
                last_path_entry = None;
            }
            updated_lines.push(line.to_string());
            continue;
        }

        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"');

            if key.ends_with("_path") {
                let prefix = key.strip_suffix("_path").unwrap_or(key).to_string();
                last_path_entry = Some((prefix, val.to_string()));
                updated_lines.push(line.to_string());
                continue;
            }

            if key.ends_with("_sha256")
                && let Some((ref last_prefix, ref rel_path)) = last_path_entry
                && last_prefix == key.strip_suffix("_sha256").unwrap_or(key)
            {
                let full_path = workspace_root.join(rel_path);
                let actual_digest = hash_file(&full_path)?;

                if actual_digest != val {
                    summary.freeze_changes += 1;
                    if check_only {
                        summary.mismatch_errors.push(format!(
                            "freeze entry `{key}` ({rel_path}) mismatch:\n  expected: {val}\n  actual:   {actual_digest}"
                        ));
                    }
                }

                let indent = line.find(key.chars().next().unwrap_or(' ')).unwrap_or(0);
                updated_lines.push(format!("{:indent$}{key} = \"{actual_digest}\"", ""));
                continue;
            }
        }

        updated_lines.push(line.to_string());
    }

    if !check_only && summary.freeze_changes > 0 {
        let mut new_content = updated_lines.join("\n");
        if freeze_raw.ends_with('\n') {
            new_content.push('\n');
        }
        fs::write(freeze_path, new_content)
            .map_err(|e| format!("could not write {}: {e}", freeze_path.display()))?;
        println!(
            "[info] fixtures: updated {} contract hashes in {}",
            summary.freeze_changes,
            freeze_path.display()
        );
    }

    Ok(())
}

/// Discovers fixture files on disk that are not registered in the manifest.
fn discover_untracked_fixtures(
    workspace_root: &Path,
    registered: &BTreeSet<String>,
) -> Result<(), String> {
    let fixtures_dir = workspace_root.join("portable/specs/fixtures");
    if !fixtures_dir.is_dir() {
        return Ok(());
    }

    let mut on_disk = Vec::new();
    collect_fixture_files(&fixtures_dir, &mut on_disk)?;

    for path in on_disk {
        if let Ok(rel) = path.strip_prefix(workspace_root) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !registered.contains(&rel_str) {
                println!("[warn] fixtures: file on disk not yet in manifest: {rel_str}");
            }
        }
    }

    Ok(())
}

/// Recursively collects fixture test files (.neu, .toml).
fn collect_fixture_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries =
        fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_fixture_files(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str())
            && (ext == "neu" || ext == "toml")
            && path.file_name().and_then(|s| s.to_str()) != Some("README.md")
        {
            files.push(path);
        }
    }
    Ok(())
}

/// Extracts the quoted string value from a line or token.
fn extract_string_value(token: &str) -> Option<String> {
    let raw = token.split_once('=').map_or(token, |(_, v)| v).trim();
    raw.strip_prefix('"')?.strip_suffix('"').map(str::to_owned)
}

/// Returns the lowercase hex SHA-256 digest of a file.
fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
