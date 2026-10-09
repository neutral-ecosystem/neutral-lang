// SPDX-License-Identifier: Apache-2.0

//! Recoverable writes limited to owned release-version metadata.

use crate::{Path, constants, fs, runtime::files::ExclusiveFileLock};
use serde::{Deserialize, Serialize};
use std::io::Write as _;

/// Stable ignored location, independent of output-directory environment overrides.
const STORE: &str = ".neutral-version-update";
/// Durable metadata transition; completion is marked before reporting success.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    /// Supported recovery representation.
    schema_version: u32,
    /// Whether every replacement was durably applied.
    complete: bool,
    /// Owned relative paths and exact old/new contents.
    entries: Vec<Replacement>,
}
/// One recoverable release metadata replacement.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Replacement {
    /// Restricted workspace-relative metadata path.
    path: String,
    /// Absence is retained distinctly from an empty file.
    old: Option<Vec<u8>>,
    /// Prepared replacement contents.
    new: Vec<u8>,
}

/// Rejects traversal, symlinks, and destinations not owned by version preparation.
fn target(root: &Path, path: &str) -> Result<std::path::PathBuf, String> {
    let relative = Path::new(path);
    let evidence = relative
        .strip_prefix(constants::QUALITY_EVIDENCE_DIRECTORY)
        .ok();
    let evidence_readme = evidence.is_some_and(|suffix| {
        let parts = suffix.iter().collect::<Vec<_>>();
        parts.len() == 2
            && parts[1] == "README.md"
            && parts[0]
                .to_str()
                .and_then(|version| version.strip_prefix('v'))
                .is_some_and(|version| crate::validate_semver(version).is_ok())
    });
    if !crate::is_safe_relative_path(relative)
        || !(path == constants::WORKSPACE_MANIFEST_FILE
            || path == constants::CARGO_LOCK_FILE
            || evidence_readme)
    {
        return Err(format!("unsafe version recovery target: {path}"));
    }
    let mut checked = root.to_owned();
    for part in relative.components() {
        checked.push(part);
        if let Ok(metadata) = fs::symlink_metadata(&checked)
            && metadata.file_type().is_symlink()
        {
            return Err(format!(
                "version recovery target contains a symlink: {path}"
            ));
        }
    }
    Ok(checked)
}

/// Claims the stable recovery store without deleting or replacing its lock inode.
fn lock(root: &Path) -> Result<ExclusiveFileLock, String> {
    let directory = root.join(STORE);
    if let Ok(metadata) = fs::symlink_metadata(&directory)
        && metadata.file_type().is_symlink()
    {
        return Err("version recovery directory must not be a symlink".to_owned());
    }
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    sync_directory(root)?;
    ExclusiveFileLock::claim(&directory.join("lock"))
}

/// Flushes directory entries where the platform exposes directory synchronization.
fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Stages complete bytes in the ignored store and replaces one destination.
fn replace(root: &Path, destination: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or("version destination has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let staging = root.join(STORE).join("replacement");
    if fs::symlink_metadata(&staging).is_ok_and(|metadata| !metadata.is_file()) {
        return Err("version recovery staging must be a regular file".to_owned());
    }
    let mut file = fs::File::create(&staging).map_err(|error| error.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| error.to_string())?;
    if let Ok(metadata) = fs::metadata(destination) {
        fs::set_permissions(&staging, metadata.permissions()).map_err(|error| error.to_string())?;
    }
    drop(file);
    fs::rename(staging, destination).map_err(|error| error.to_string())?;
    sync_directory(parent)
}

/// Publishes a complete journal before any destination is modified.
fn write_journal(root: &Path, journal: &Journal) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(journal).map_err(|error| error.to_string())?;
    replace(root, &root.join(STORE).join("journal.json"), &bytes)
}

/// Reads regular files only, retaining absence for a newly created scaffold.
fn contents(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
        Ok(metadata) if metadata.is_file() => {
            fs::read(path).map(Some).map_err(|error| error.to_string())
        }
        Ok(_) => Err(format!(
            "version metadata must be a regular file: {}",
            path.display()
        )),
    }
}

/// Validates every current state before recovery can touch any destination.
fn validate(root: &Path, journal: &Journal) -> Result<(), String> {
    if journal.schema_version != 1 || journal.entries.is_empty() || journal.entries.len() > 3 {
        return Err("unsupported or empty version recovery journal".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &journal.entries {
        let destination = target(root, &entry.path)?;
        let current = contents(&destination)?;
        if !seen.insert(&entry.path)
            || (current.as_deref() != Some(entry.new.as_slice())
                && (journal.complete || current != entry.old))
        {
            return Err(format!(
                "version recovery refuses modified or duplicate target: {}",
                entry.path
            ));
        }
    }
    Ok(())
}

/// Restores an interrupted update only when all destinations have expected bytes.
fn rollback(root: &Path, journal: &Journal) -> Result<(), String> {
    validate(root, journal)?;
    for entry in journal.entries.iter().rev() {
        let destination = target(root, &entry.path)?;
        match &entry.old {
            Some(bytes) => replace(root, &destination, bytes)?,
            None if destination.exists() => {
                fs::remove_file(&destination).map_err(|error| error.to_string())?;
                sync_directory(
                    destination
                        .parent()
                        .ok_or("version destination has no parent")?,
                )?;
            }
            None => {}
        }
    }
    fs::remove_file(root.join(STORE).join("journal.json")).map_err(|error| error.to_string())?;
    sync_directory(&root.join(STORE))
}

/// Recovers interruption, or returns completed owned paths for safe release-commit retry.
pub(crate) fn recover(root: &Path) -> Result<Option<Vec<String>>, String> {
    let _lock = lock(root)?;
    let Some(bytes) = contents(&root.join(STORE).join("journal.json"))? else {
        return Ok(None);
    };
    let journal: Journal = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid version recovery journal: {error}"))?;
    validate(root, &journal)?;
    if journal.complete {
        return Ok(Some(
            journal
                .entries
                .into_iter()
                .map(|entry| entry.path)
                .collect(),
        ));
    }
    rollback(root, &journal)?;
    Ok(None)
}

/// Acknowledges a complete metadata update after the caller has safely consumed it.
pub(crate) fn acknowledge(root: &Path) -> Result<(), String> {
    let _lock = lock(root)?;
    let path = root.join(STORE).join("journal.json");
    let Some(bytes) = contents(&path)? else {
        return Ok(());
    };
    let journal: Journal = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    validate(root, &journal)?;
    if !journal.complete {
        return Err("version update still needs recovery".to_owned());
    }
    fs::remove_file(path).map_err(|error| error.to_string())?;
    sync_directory(&root.join(STORE))
}

/// Applies the prepared metadata bytes to their workspace-relative destinations.
pub(crate) fn apply(root: &Path, changes: &[(String, Vec<u8>)]) -> Result<(), String> {
    apply_with(root, changes, replace)
}

/// Keeps installation failures injectable while recovery always uses the real replacement path.
fn apply_with(
    root: &Path,
    changes: &[(String, Vec<u8>)],
    mut install: impl FnMut(&Path, &Path, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let _lock = lock(root)?;
    if root.join(STORE).join("journal.json").exists() {
        return Err("version update needs recovery first".to_owned());
    }
    let mut journal = Journal {
        schema_version: 1,
        complete: false,
        entries: Vec::new(),
    };
    for (path, bytes) in changes {
        let destination = target(root, path)?;
        journal.entries.push(Replacement {
            path: path.clone(),
            old: contents(&destination)?,
            new: bytes.clone(),
        });
    }
    validate(root, &journal)?;
    write_journal(root, &journal)?;
    let result = journal
        .entries
        .iter()
        .try_for_each(|entry| install(root, &root.join(&entry.path), &entry.new));
    if let Err(error) = result {
        return match rollback(root, &journal) {
            Ok(()) => Err(format!("version update rolled back: {error}")),
            Err(recovery) => Err(format!(
                "version update failed: {error}; rerun preparation to recover: {recovery}"
            )),
        };
    }
    journal.complete = true;
    write_journal(root, &journal)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/version_update.rs"]
mod tests;
