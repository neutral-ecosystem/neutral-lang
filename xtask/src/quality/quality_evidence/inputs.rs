// SPDX-License-Identifier: Apache-2.0

//! Fingerprint the repository inputs a quality measurement actually observes.
//!
//! Git supplies tracked and nonignored new paths; ordered path/content framing
//! makes file additions and edits part of the digest. Only designated guide and
//! approval-record locations are excluded. This binds evidence to inputs, not
//! merely a commit label, and rejects nonregular files rather than following them.

use super::{BTreeSet, Command, Path, Sha256, constants};
use sha2::Digest as _;
use std::fs;

/// Hashes ordered code, tests, fixtures, locks, and configuration, including new files.
pub(crate) fn input_digest(root: &Path) -> Result<String, String> {
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .map_err(|error| format!("could not list gate inputs: {error}"))?;
    if !output.status.success() {
        return Err("could not list tracked gate inputs".to_owned());
    }
    let paths = String::from_utf8(output.stdout)
        .map_err(|error| format!("non-UTF-8 gate input: {error}"))?;
    let mut hash = Sha256::new();
    hash.update(b"neutral-quality-inputs/2\0");
    let paths = paths
        .split('\0')
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    for relative in paths {
        let path = Path::new(relative);
        // Only known guide locations are excluded: Markdown can contain executable fixtures.
        let guide = path.extension().and_then(std::ffi::OsStr::to_str) == Some("md")
            && (path.parent() == Some(Path::new("docs"))
                || path.file_name().and_then(std::ffi::OsStr::to_str) == Some("README.md")
                || path == Path::new(constants::QUALITY_STATUS_FILE));
        if guide
            || (path.starts_with(constants::QUALITY_EVIDENCE_DIRECTORY)
                && path.file_name().and_then(std::ffi::OsStr::to_str) == Some("record.toml"))
        {
            continue;
        }
        let metadata = match fs::symlink_metadata(root.join(path)) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("could not inspect gate input {relative}: {error}")),
        };
        if !metadata.is_file() {
            return Err(format!("gate input must be a regular file: {relative}"));
        }
        let bytes = fs::read(root.join(path))
            .map_err(|error| format!("could not read gate input {relative}: {error}"))?;
        hash.update((relative.len() as u64).to_be_bytes());
        hash.update(relative.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}
