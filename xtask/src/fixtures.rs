// SPDX-License-Identifier: Apache-2.0

//! Formatting-preserving synchronization of reviewed fixture/oracle and contract hashes.

use crate::{constants, manifest_updates};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item, TableLike};

/// Counts checked inputs and changed digests without treating synchronization as approval.
#[derive(Default)]
struct SyncSummary {
    /// Checked fixture records.
    fixture_count: usize,
    /// Checked oracle records.
    oracle_count: usize,
    /// Changed conformance hashes.
    manifest_changes: usize,
    /// Changed contract hashes.
    freeze_changes: usize,
    /// Check-mode mismatches.
    mismatch_errors: Vec<String>,
}

/// Preflights both documents and all their inputs before writing any changed manifest.
pub(crate) fn sync_fixtures(root: &Path, check: bool) -> Result<(), String> {
    println!(
        "{} fixtures: {} fixture and contract hashes",
        constants::INFO,
        if check { "verifying" } else { "synchronizing" }
    );
    let manifest_path = root.join(constants::PORTABLE_CONFORMANCE_MANIFEST_FILE);
    let freeze_path = root.join(constants::PORTABLE_CONTRACT_FREEZE_FILE);
    let manifest_raw =
        crate::read_workspace_text(root, constants::PORTABLE_CONFORMANCE_MANIFEST_FILE)?;
    let freeze_raw = crate::read_workspace_text(root, constants::PORTABLE_CONTRACT_FREEZE_FILE)?;
    let mut manifest = manifest_updates::parse(&manifest_raw, "fixture manifest")?;
    let mut freeze = manifest_updates::parse(&freeze_raw, "contract freeze")?;
    let mut summary = SyncSummary::default();
    let registered = sync_manifest(root, &mut manifest, check, &mut summary)?;
    let updated_manifest = manifest.to_string();
    sync_freeze_table(
        root,
        freeze.as_table_mut(),
        check,
        &mut summary,
        &manifest_path,
        updated_manifest.as_bytes(),
    )?;
    discover_untracked_fixtures(root, &registered)?;
    if check && !summary.mismatch_errors.is_empty() {
        return Err(format!(
            "fixture verification failed with {} mismatch(es):\n{}",
            summary.mismatch_errors.len(),
            summary.mismatch_errors.join("\n\n")
        ));
    }
    if !check {
        if summary.manifest_changes > 0 {
            fs::write(&manifest_path, updated_manifest)
                .map_err(|error| format!("could not write {}: {error}", manifest_path.display()))?;
        }
        if summary.freeze_changes > 0 {
            fs::write(&freeze_path, freeze.to_string())
                .map_err(|error| format!("could not write {}: {error}", freeze_path.display()))?;
        }
    }
    println!(
        "{} fixtures: verified {} fixtures, {} oracles (manifest updates: {}, freeze updates: {})",
        constants::INFO,
        summary.fixture_count,
        summary.oracle_count,
        summary.manifest_changes,
        summary.freeze_changes
    );
    Ok(())
}

/// Verifies fixture and oracle pairs regardless of field ordering, preserving unrelated extensions.
fn sync_manifest(
    root: &Path,
    document: &mut DocumentMut,
    check: bool,
    summary: &mut SyncSummary,
) -> Result<BTreeSet<String>, String> {
    let cases = document
        .get_mut("case")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or("fixture manifest has no [[case]] records")?;
    let mut registered = BTreeSet::new();
    for case in cases.iter_mut() {
        for kind in ["fixture", "oracle"] {
            let path = case
                .get(kind)
                .and_then(Item::as_str)
                .ok_or_else(|| format!("fixture case has no string {kind} path"))?
                .to_owned();
            let actual = hash_file(&registered_path(root, &path)?)?;
            let key = format!("{kind}_sha256");
            let item = case
                .get_mut(&key)
                .ok_or_else(|| format!("fixture case has no {key}"))?;
            if update_digest(item, &actual, &path, check, &mut summary.mismatch_errors)? {
                summary.manifest_changes += 1;
            }
            if kind == "fixture" {
                registered.insert(path);
                summary.fixture_count += 1;
            } else {
                summary.oracle_count += 1;
            }
        }
    }
    Ok(registered)
}

/// Resolves only regular workspace-owned files, rejecting traversal and symlink escapes.
fn registered_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !crate::configuration::is_safe_relative_path(Path::new(relative)) {
        return Err(format!("unsafe fixture/contract path: {relative}"));
    }
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("could not resolve {relative}: {error}"))?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("could not resolve workspace root: {error}"))?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(format!(
            "fixture/contract path is external or not a file: {relative}"
        ));
    }
    Ok(path)
}

/// Checks or edits a digest string while retaining the original comment and spacing.
fn update_digest(
    item: &mut Item,
    actual: &str,
    context: &str,
    check: bool,
    mismatches: &mut Vec<String>,
) -> Result<bool, String> {
    let expected = item
        .as_str()
        .ok_or_else(|| format!("{context} digest must be a string"))?
        .to_owned();
    if expected == actual {
        return Ok(false);
    }
    if check {
        mismatches.push(format!(
            "{context} digest mismatch:\n  expected: {expected}\n  actual:   {actual}"
        ));
    } else {
        manifest_updates::replace_string(item, &expected, actual, context)?;
    }
    Ok(true)
}

/// Visits contract path/hash pairs in their owning table, including nested and repeated tables.
fn sync_freeze_table(
    root: &Path,
    table: &mut dyn TableLike,
    check: bool,
    summary: &mut SyncSummary,
    manifest_path: &Path,
    manifest_bytes: &[u8],
) -> Result<(), String> {
    let pairs = table
        .iter()
        .filter_map(|(key, item)| {
            let prefix = key.strip_suffix("_sha256")?;
            table.get(&format!("{prefix}_path")).map(|path| {
                (
                    key.to_owned(),
                    path.as_str().map(str::to_owned),
                    item.as_str().is_some(),
                )
            })
        })
        .collect::<Vec<_>>();
    for (key, path, valid_digest) in pairs {
        let relative = path.ok_or_else(|| format!("{key} companion path must be a string"))?;
        if !valid_digest {
            return Err(format!("{key} digest must be a string"));
        }
        let path = registered_path(root, &relative)?;
        let actual = if path == manifest_path {
            crate::sha256_hex(manifest_bytes)
        } else {
            hash_file(&path)?
        };
        let item = table
            .get_mut(&key)
            .ok_or_else(|| format!("missing freeze digest {key}"))?;
        if update_digest(
            item,
            &actual,
            &relative,
            check,
            &mut summary.mismatch_errors,
        )? {
            summary.freeze_changes += 1;
        }
    }
    for (_, item) in table.iter_mut() {
        if let Some(tables) = item.as_array_of_tables_mut() {
            for child in tables.iter_mut() {
                sync_freeze_table(root, child, check, summary, manifest_path, manifest_bytes)?;
            }
        } else if let Some(child) = item.as_table_like_mut() {
            sync_freeze_table(root, child, check, summary, manifest_path, manifest_bytes)?;
        }
    }
    Ok(())
}

/// Warns about fixture files not registered in the reviewed manifest.
fn discover_untracked_fixtures(root: &Path, registered: &BTreeSet<String>) -> Result<(), String> {
    let mut files = Vec::new();
    collect_fixture_files(
        &root.join(constants::PORTABLE_FIXTURE_DIRECTORY),
        &mut files,
    )?;
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if !registered.contains(&relative) {
            println!(
                "{} fixtures: file on disk not yet in manifest: {relative}",
                constants::WARN
            );
        }
    }
    Ok(())
}

/// Collects test files without following directory symlinks or swallowing directory errors.
fn collect_fixture_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?
    {
        let entry =
            entry.map_err(|error| format!("could not read fixture directory entry: {error}"))?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("could not inspect fixture: {error}"))?;
        if kind.is_symlink() {
            return Err(format!(
                "fixture inventory cannot follow symlinks: {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            collect_fixture_files(&entry.path(), files)?;
        } else if kind.is_file()
            && matches!(
                entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("neu" | "toml")
            )
        {
            files.push(entry.path());
        }
    }
    Ok(())
}

/// Hashes the exact captured bytes, never a normalized text representation.
fn hash_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    Ok(crate::sha256_hex(&bytes))
}

#[cfg(test)]
#[path = "../tests/unit/fixtures.rs"]
mod tests;
