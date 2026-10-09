// SPDX-License-Identifier: Apache-2.0

//! runtime / files responsibilities for repository automation.

use crate::{Path, PathBuf, command_output, constants, fs, sha256_hex};

/// Scoped ownership of a stable OS lock file; the inode is never replaced or deleted.
pub(crate) struct ExclusiveFileLock {
    /// The descriptor owning the platform lock.
    file: fs::File,
}

impl ExclusiveFileLock {
    /// Claims an exclusive lock without truncation or following an existing symlink.
    pub(crate) fn claim(path: &Path) -> Result<Self, String> {
        if fs::symlink_metadata(path).is_ok_and(|metadata| !metadata.is_file()) {
            return Err(format!("lock must be a regular file: {}", path.display()));
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.try_lock().map_err(|error| {
            format!(
                "already running or lock unavailable at {}: {error}",
                path.display()
            )
        })?;
        Ok(Self { file })
    }
}

impl Drop for ExclusiveFileLock {
    /// Explicit unlock also releases transient descriptor copies inherited by forked children.
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Returns the SHA-256 digest of one exact file.
pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("could not read {}: {error}", path.display()))
}

/// Returns whether text is one lowercase SHA-256 hexadecimal digest.
pub(crate) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

/// Extracts local, anchor-free Markdown link paths from document text.
pub(crate) fn markdown_link_targets(content: &str) -> Vec<String> {
    content
        .split("](")
        .skip(1)
        .filter_map(|tail| tail.split_once(')').map(|(target, _)| target))
        .map(str::trim)
        .map(|target| target.trim_matches(['<', '>']))
        .filter(|target| {
            !target.is_empty()
                && !target.starts_with('#')
                && !target.contains("://")
                && !target.starts_with("mailto:")
        })
        .map(|target| target.split('#').next().unwrap_or(target))
        .filter(|target| !target.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Reads one required UTF-8 workspace file with a path-safe error.
pub(crate) fn read_workspace_text(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("could not read {relative}: {error}"))
}

/// Recursively collects regular files without following directory symlinks.
pub(crate) fn collect_regular_files(
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("could not inspect directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        if file_type.is_dir() {
            collect_regular_files(&entry.path(), files)?;
        } else if file_type.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

/// Creates a process-unique directory below one approved generated-output root.
pub(crate) fn unique_generated_directory(parent: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "could not create generated evidence directory {}: {error}",
            parent.display()
        )
    })?;
    for suffix in 0_u16..1000 {
        let directory = parent.join(format!("run-{}-{suffix}", std::process::id()));
        match fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("could not create {}: {error}", directory.display())),
        }
    }
    Err("could not allocate a unique result directory".to_owned())
}

/// Requires a clean checkout and returns its exact current Git commit.
pub(crate) fn require_clean_checkout() -> Result<String, String> {
    let commit = command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?;
    let status = command_output(constants::GIT_COMMAND, &["status", "--porcelain"])?;
    if !status.is_empty() {
        return Err("quality evaluation requires a clean worktree".to_owned());
    }
    Ok(commit)
}

/// Requires a clean checked-out `main` and returns its exact current candidate commit.
pub(crate) fn require_main_head_checkout() -> Result<String, String> {
    let branch = command_output("git", &["branch", "--show-current"])?;
    if branch != "main" {
        return Err(format!(
            "release qualification requires checked-out main; found {branch:?}"
        ));
    }
    let head = command_output("git", &["rev-parse", "HEAD"])?;
    let status = command_output("git", &["status", "--porcelain"])?;
    if !status.is_empty() {
        return Err("release qualification requires a clean main worktree".to_owned());
    }
    Ok(head)
}

#[cfg(test)]
#[path = "../../tests/unit/file_lock.rs"]
mod tests;
