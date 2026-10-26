// SPDX-License-Identifier: Apache-2.0

//! runtime / files responsibilities for repository automation.

use crate::{Command, Path, PathBuf, command_output, constants, fs, sha256_hex};

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

/// Checks a Markdown link in the checkout, then narrowly proves historical links
/// in unchanged portable development evidence at the record's last Git revision.
pub(crate) fn markdown_link_exists(
    root: &Path,
    canonical_root: &Path,
    source: &Path,
    target: &str,
) -> Result<bool, String> {
    let candidate = source.parent().unwrap_or(root).join(target);
    if fs::canonicalize(&candidate).is_ok_and(|path| path.starts_with(canonical_root)) {
        return Ok(true);
    }
    historical_evidence_link_exists(root, source, target)
}

/// Resolves a missing evidence link only when both record bytes and target blob
/// are proven at the record's most recent commit. No path escaping is accepted.
fn historical_evidence_link_exists(
    root: &Path,
    source: &Path,
    target: &str,
) -> Result<bool, String> {
    let Ok(source_relative) = source.strip_prefix(root) else {
        return Ok(false);
    };
    let source_relative = slash_path(source_relative);
    if !is_portable_development_evidence(&source_relative) {
        return Ok(false);
    }
    let Some(target_relative) = resolve_markdown_target(source_relative.as_str(), target) else {
        return Ok(false);
    };
    let target_relative = slash_path(&target_relative);

    let shallow = git_output(root, &["rev-parse", "--is-shallow-repository"])?;
    if shallow.trim_ascii() == b"true" {
        return Err(format!(
            "could not prove historical Markdown link in {source_relative}: checkout is shallow; fetch full history"
        ));
    }

    let commit = git_output(root, &["log", "-n1", "--format=%H", "--", &source_relative])?;
    let commit = String::from_utf8(commit)
        .map_err(|error| format!("Git emitted non-UTF-8 evidence revision: {error}"))?;
    let commit = commit.trim();
    if commit.is_empty() {
        return Err(format!(
            "could not prove historical Markdown link in {source_relative}: no Git revision is available (checkout may be shallow)"
        ));
    }

    let source_spec = format!("{commit}:{source_relative}");
    let committed_source = git_output(root, &["show", &source_spec]).map_err(|error| {
        format!("could not prove historical evidence record {source_relative}: {error}")
    })?;
    let current_source = fs::read(root.join(&source_relative))
        .map_err(|error| format!("could not read evidence record {source_relative}: {error}"))?;
    if committed_source != current_source {
        return Ok(false);
    }

    let entries = git_output(root, &["ls-tree", "-z", commit, "--", &target_relative])
        .map_err(|error| format!("could not prove historical target {target_relative}: {error}"))?;
    Ok(entries.split(|byte| *byte == 0).any(|entry| {
        let Some(separator) = entry.iter().position(|byte| *byte == b'\t') else {
            return false;
        };
        let metadata = &entry[..separator];
        let path = &entry[separator + 1..];
        if path != target_relative.as_bytes() {
            return false;
        }
        let mut fields = metadata.split(|byte| *byte == b' ');
        let mode = fields.next();
        let kind = fields.next();
        matches!(mode, Some(mode) if mode == b"100644" || mode == b"100755")
            && kind == Some(&b"blob"[..])
    }))
}

/// Restricts historical fallback to active or archived portable development evidence.
fn is_portable_development_evidence(path: &str) -> bool {
    let parts = path.split('/').collect::<Vec<_>>();
    let active = parts.len() >= 4
        && parts[0] == "portable"
        && parts[1] == "development"
        && parts[2] == "evidence";
    let archived = parts.len() >= 5
        && parts[0] == "portable"
        && matches!(parts[1], "archive" | "archived")
        && parts
            .windows(2)
            .any(|pair| pair == ["development", "evidence"]);
    active || archived
}

/// Resolves a workspace-relative Markdown target without allowing root escape.
fn resolve_markdown_target(source: &str, target: &str) -> Option<PathBuf> {
    let target = Path::new(target);
    if target.is_absolute() {
        return None;
    }
    let mut resolved = Path::new(source).parent()?.to_path_buf();
    for component in target.components() {
        match component {
            std::path::Component::Normal(value) => resolved.push(value),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !resolved.pop() {
                    return None;
                }
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }
    (!resolved.as_os_str().is_empty()).then_some(resolved)
}

/// Uses Git's repository-relative path spelling on every supported host.
fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Reads repository proof through explicit arguments without a shell.
fn git_output(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run git {}: {error}", arguments.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed with {}: {}",
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
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
    let branch = command_output(constants::GIT_COMMAND, &["branch", "--show-current"])?;
    if branch != constants::RELEASE_BRANCH {
        return Err(format!(
            "release qualification requires checked-out {}; found {branch:?}",
            constants::RELEASE_BRANCH,
        ));
    }
    let head = command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?;
    let status = command_output(constants::GIT_COMMAND, &["status", "--porcelain"])?;
    if !status.is_empty() {
        return Err(format!(
            "release qualification requires a clean {} worktree",
            constants::RELEASE_BRANCH
        ));
    }
    Ok(head)
}

#[cfg(test)]
#[path = "../../tests/unit/file_lock.rs"]
mod tests;
