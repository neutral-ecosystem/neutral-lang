// SPDX-License-Identifier: Apache-2.0

//! Exclusive measurement ownership and isolated, completion-marked report runs.

use crate::{
    Path, PathBuf, fs,
    runtime::files::{ExclusiveFileLock, unique_generated_directory},
};

/// A live OS lock; the stable lock file must not be unlinked on release.
pub(crate) struct MeasurementStore {
    /// Closing the handle releases ownership, including after process termination.
    _lock: ExclusiveFileLock,
    /// Unique writable run, never reused by a later measurement.
    pub(crate) directory: PathBuf,
}

impl MeasurementStore {
    /// Claims a gate and selects an incomplete new run before touching earlier evidence.
    pub(crate) fn begin(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root)
            .map_err(|error| format!("could not create measurement store: {error}"))?;
        let lock = ExclusiveFileLock::claim(&root.join("measurement.lock"))?;
        let directory = unique_generated_directory(root)?;
        let name = directory
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or("invalid measurement run name")?;
        let pending = root.join("current.pending");
        fs::write(&pending, name)
            .map_err(|error| format!("could not stage measurement pointer: {error}"))?;
        fs::rename(&pending, root.join("current"))
            .map_err(|error| format!("could not publish measurement attempt: {error}"))?;
        Ok(Self {
            _lock: lock,
            directory,
        })
    }
}

/// Resolves only a direct regular run directory, or a flat retained snapshot.
pub(crate) fn resolve(root: &Path) -> Result<PathBuf, String> {
    let pointer = root.join("current");
    let name = match fs::read_to_string(&pointer) {
        Ok(name) => name,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(root.to_owned()),
        Err(error) => return Err(format!("could not read measurement pointer: {error}")),
    };
    if !name.starts_with("run-")
        || !crate::is_safe_relative_path(Path::new(&name))
        || Path::new(&name).components().count() != 1
    {
        return Err("unsafe measurement pointer".to_owned());
    }
    if fs::symlink_metadata(&pointer)
        .map_err(|error| error.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("measurement pointer must not be a symlink".to_owned());
    }
    let run = root.join(name);
    let metadata = fs::symlink_metadata(&run)
        .map_err(|error| format!("measurement run is missing: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("measurement run must be a regular directory".to_owned());
    }
    Ok(run)
}

#[cfg(test)]
#[path = "../../tests/unit/measurement_store.rs"]
mod tests;
