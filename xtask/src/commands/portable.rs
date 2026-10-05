// SPDX-License-Identifier: Apache-2.0

//! commands / portable responsibilities for repository automation.

use crate::{
    BTreeMap, Component, Path, PathBuf, PortableAction, check_portable_traceability_at,
    collect_regular_files, configuration_section, configuration_value, constants,
    ensure_no_portable_archive_dependencies, fs, is_sha256, json_string, line_spdx_marker,
    markdown_link_targets, project_license, read_workspace_text, result_root, sha256_hex,
    unique_generated_directory, workspace_root,
};
use std::fmt::Write as _;

/// Executes the active portable lifecycle command family.
pub(crate) fn portable(action: PortableAction) -> Result<(), String> {
    match action {
        PortableAction::Install(source) => install_portable(&source),
        PortableAction::Verify => verify_portable(),
        PortableAction::Snapshot => snapshot_portable(),
    }
}

/// Atomically installs and verifies one reviewed active portable package.
pub(crate) fn install_portable(source: &Path) -> Result<(), String> {
    let root = workspace_root()?;
    let destination = root.join(constants::PORTABLE_DIRECTORY);
    if destination.exists() {
        return Err(format!(
            "active portable destination already exists: {}",
            destination.display()
        ));
    }
    let source = fs::canonicalize(source).map_err(|error| {
        format!(
            "could not resolve portable source {}: {error}",
            source.display()
        )
    })?;
    if !source.is_dir() {
        return Err(format!(
            "portable source is not a directory: {}",
            source.display()
        ));
    }
    let staging = root.join(format!(".portable.install-{}", std::process::id()));
    if staging.exists() {
        return Err(format!(
            "portable installation staging path already exists: {}",
            staging.display()
        ));
    }
    if let Err(error) = copy_portable_tree(&source, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    fs::rename(&staging, &destination).map_err(|error| {
        format!(
            "could not publish portable package {} as {}: {error}",
            staging.display(),
            destination.display()
        )
    })?;
    if let Err(error) = verify_portable() {
        let rejected_root = result_root()?.join("portable/rejected");
        let rejected_run = match unique_generated_directory(&rejected_root) {
            Ok(directory) => directory,
            Err(move_error) => {
                fs::rename(&destination, &staging).map_err(|restore_error| {
                    format!(
                        "{error}; could not allocate rejected-package storage: {move_error}; could not retain the package at {}: {restore_error}",
                        staging.display()
                    )
                })?;
                return Err(format!(
                    "{error}; could not allocate rejected-package storage: {move_error}; rejected package retained at {}",
                    staging.display()
                ));
            }
        };
        let rejected = rejected_run.join(constants::PORTABLE_DIRECTORY);
        fs::rename(&destination, &rejected).map_err(|move_error| {
            format!(
                "{error}; could not move rejected package {} to {}: {move_error}",
                destination.display(),
                rejected.display()
            )
        })?;
        return Err(format!(
            "{error}; rejected package retained at {}",
            rejected.display()
        ));
    }
    crate::output::pass("active portable installed");
    crate::output::file("portable source", &source);
    Ok(())
}

/// Copies a portable directory without following symbolic links or special files.
pub(crate) fn copy_portable_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination)
        .map_err(|error| format!("could not create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("could not inspect {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect portable entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_portable_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| {
                format!(
                    "could not copy {} to {}: {error}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        } else {
            return Err(format!(
                "portable source contains a symbolic link or special file: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

/// Verifies the active portable package's required roots and traceability.
pub(crate) fn verify_portable() -> Result<(), String> {
    let root = workspace_root()?;
    verify_portable_layout(&root)?;
    let lifecycle = read_workspace_text(&root, constants::PORTABLE_LIFECYCLE_FILE)?;
    let active_series = configuration_value(&lifecycle, "active_series")
        .ok_or_else(|| "active portable lifecycle has no active_series".to_owned())?;
    let status = configuration_value(&lifecycle, "status")
        .ok_or_else(|| "active portable lifecycle has no status".to_owned())?;
    if status != "active" {
        return Err(format!(
            "installed portable status must be active; found {status:?}"
        ));
    }
    if !is_portable_series(&active_series) {
        return Err(format!(
            "active portable series must be a numeric v-prefixed identifier; found {active_series:?}"
        ));
    }
    check_portable_traceability_at(&root)?;
    verify_frozen_input_digests(&root, constants::PORTABLE_CONTRACT_FREEZE_FILE)?;
    verify_portable_links(&root)?;
    ensure_no_portable_archive_dependencies(&root)?;
    verify_existing_portable_snapshots(&root)?;
    crate::output::pass("active portable package");
    Ok(())
}

/// Verifies the version-independent files and directories required from every portable package.
pub(crate) fn verify_portable_layout(root: &Path) -> Result<(), String> {
    for relative in [
        constants::PORTABLE_PLAN_FILE,
        constants::PORTABLE_LIFECYCLE_FILE,
        constants::PORTABLE_REQUIREMENTS_FILE,
        constants::PORTABLE_TRACEABILITY_FILE,
        constants::PORTABLE_CONTRACT_FREEZE_FILE,
        constants::PORTABLE_CONFORMANCE_MANIFEST_FILE,
    ] {
        if !root.join(relative).is_file() {
            return Err(format!("active portable file is missing: {relative}"));
        }
        if read_workspace_text(root, relative)?.trim().is_empty() {
            return Err(format!("active portable file is empty: {relative}"));
        }
    }
    for relative in [
        constants::PORTABLE_CONTRACT_DIRECTORY,
        constants::PORTABLE_FIXTURE_DIRECTORY,
        constants::PORTABLE_ORACLE_DIRECTORY,
    ] {
        if !root.join(relative).is_dir() {
            return Err(format!("active portable directory is missing: {relative}"));
        }
    }
    Ok(())
}

/// Returns whether a portable series is a nonempty numeric `v` identifier.
pub(crate) fn is_portable_series(value: &str) -> bool {
    value.strip_prefix('v').is_some_and(|digits| {
        !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
    })
}

/// Verifies an active portable package when one is installed for development.
pub(crate) fn verify_optional_portable() -> Result<(), String> {
    let root = workspace_root()?;
    if root.join(constants::PORTABLE_DIRECTORY).exists() {
        verify_portable()
    } else {
        crate::output::info(
            "no active portable package installed; release conformance remains available",
        );
        Ok(())
    }
}

/// Verifies every locally retained digest-addressed portable snapshot.
pub(crate) fn verify_existing_portable_snapshots(root: &Path) -> Result<(), String> {
    let snapshot_root = result_root()?.join(constants::PORTABLE_SNAPSHOT_DIRECTORY);
    if !snapshot_root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&snapshot_root)
        .map_err(|error| format!("could not inspect {}: {error}", snapshot_root.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect snapshot entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?
            .is_dir()
            || entry.file_name().to_string_lossy().starts_with('.')
        {
            continue;
        }
        verify_portable_snapshot_directory(root, &entry.path())?;
    }
    Ok(())
}

/// Verifies one snapshot manifest, directory digest, and copied file set.
pub(crate) fn verify_portable_snapshot_directory(
    root: &Path,
    directory: &Path,
) -> Result<(), String> {
    let manifest = fs::read_to_string(directory.join("manifest.sha256"))
        .map_err(|error| format!("could not read snapshot manifest: {error}"))?;
    let expected_license_marker = line_spdx_marker(&project_license(root)?);
    if !manifest.starts_with(&expected_license_marker) {
        return Err("portable snapshot manifest has the wrong license marker".to_owned());
    }
    let mut records = String::new();
    for line in manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        writeln!(&mut records, "{line}").expect("writing to a String cannot fail");
    }
    let expected_tree = directory
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| "portable snapshot directory has no UTF-8 digest name".to_owned())?;
    if sha256_hex(records.as_bytes()) != expected_tree {
        return Err(format!(
            "portable snapshot directory {expected_tree} does not match its manifest"
        ));
    }
    for record in records.lines() {
        let mut fields = record.splitn(3, "  ");
        let expected_digest = fields
            .next()
            .ok_or_else(|| format!("invalid portable snapshot record: {record}"))?;
        let expected_size = fields
            .next()
            .ok_or_else(|| format!("invalid portable snapshot record: {record}"))?
            .parse::<usize>()
            .map_err(|error| format!("invalid portable snapshot size: {error}"))?;
        let relative = fields
            .next()
            .ok_or_else(|| format!("invalid portable snapshot record: {record}"))?;
        let path = directory.join("content").join(relative);
        let bytes = fs::read(&path)
            .map_err(|error| format!("could not read snapshot file {}: {error}", path.display()))?;
        if bytes.len() != expected_size || sha256_hex(&bytes) != expected_digest {
            return Err(format!(
                "portable snapshot file differs from manifest: {}",
                path.strip_prefix(root).unwrap_or(&path).display()
            ));
        }
    }
    Ok(())
}

/// Verifies every frozen input path and any digest retained beside it.
///
/// Active single-maintainer plans may track the input by path alone. Released
/// freezes can retain a matching `*_sha256` field for immutable archival
/// verification.
pub(crate) fn verify_frozen_input_digests(root: &Path, freeze_file: &str) -> Result<(), String> {
    let freeze = read_workspace_text(root, freeze_file)?;
    let values = configuration_section(&freeze, "fixture_corpus")?
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let frozen_paths = values
        .iter()
        .filter_map(|(key, path)| {
            key.strip_suffix("_path")
                .map(|name| (name.to_owned(), path.to_owned()))
        })
        .collect::<Vec<_>>();
    if frozen_paths.is_empty() {
        return Err("contract freeze has no frozen input paths".to_owned());
    }
    for (name, path) in frozen_paths {
        let digest_key = format!("{name}_sha256");
        let relative = Path::new(&path);
        if relative.as_os_str().is_empty()
            || !relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "frozen input path must be a safe workspace-relative path: {path}"
            ));
        }
        let bytes = fs::read(root.join(relative))
            .map_err(|error| format!("could not read frozen input {path}: {error}"))?;
        if let Some(expected) = values.get(&digest_key) {
            if !is_sha256(expected) {
                return Err(format!("contract freeze has an invalid {digest_key}"));
            }
            let actual = sha256_hex(&bytes);
            if actual != expected.as_str() {
                return Err(format!(
                    "frozen input {path} has SHA-256 {actual}, expected {expected}; contract review is required"
                ));
            }
        }
    }
    Ok(())
}

/// Verifies repository-local links in every active portable Markdown file.
pub(crate) fn verify_portable_links(root: &Path) -> Result<(), String> {
    let portable_root = root.join(constants::PORTABLE_DIRECTORY);
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("could not canonicalize workspace root: {error}"))?;
    let mut files = Vec::new();
    collect_regular_files(&portable_root, &mut files)?;
    let mut missing = Vec::new();
    for file in files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
    {
        let content = fs::read_to_string(file)
            .map_err(|error| format!("could not read {}: {error}", file.display()))?;
        for target in markdown_link_targets(&content) {
            let candidate = file.parent().unwrap_or(&portable_root).join(&target);
            let valid = fs::canonicalize(&candidate)
                .is_ok_and(|candidate| candidate.starts_with(&canonical_root));
            if !valid {
                missing.push(format!(
                    "{} -> {target}",
                    file.strip_prefix(root).unwrap_or(file).display()
                ));
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "active portable links are missing: {}",
            missing.join(", ")
        ))
    }
}

/// Creates an atomic digest-addressed copy of the active portable package.
pub(crate) fn snapshot_portable() -> Result<(), String> {
    verify_portable()?;
    let root = workspace_root()?;
    let portable_root = root.join(constants::PORTABLE_DIRECTORY);
    let mut files = Vec::new();
    collect_regular_files(&portable_root, &mut files)?;
    files.sort();
    let records = portable_digest_records(&root, &files)?;
    let tree_digest = sha256_hex(records.as_bytes());
    let parent = result_root()?.join(constants::PORTABLE_SNAPSHOT_DIRECTORY);
    let output = parent.join(&tree_digest);
    let license_marker = line_spdx_marker(&project_license(&root)?);
    let manifest =
        format!("{license_marker}\n# sha256  bytes  repository-relative-path\n{records}");
    if output.exists() {
        let existing = fs::read_to_string(output.join("manifest.sha256"))
            .map_err(|error| format!("could not read existing snapshot manifest: {error}"))?;
        if existing != manifest {
            return Err(format!(
                "portable snapshot collision at {}",
                output.display()
            ));
        }
        crate::output::file("portable snapshot", &output);
        return Ok(());
    }
    fs::create_dir_all(&parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let partial = parent.join(format!(".{tree_digest}.partial-{}", std::process::id()));
    fs::create_dir(&partial)
        .map_err(|error| format!("could not create {}: {error}", partial.display()))?;
    let content_root = partial.join("content").join(constants::PORTABLE_DIRECTORY);
    for source in &files {
        let relative = source
            .strip_prefix(&portable_root)
            .map_err(|error| format!("portable path escaped its root: {error}"))?;
        let destination = content_root.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        fs::copy(source, &destination).map_err(|error| {
            format!(
                "could not copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    fs::write(partial.join("manifest.sha256"), &manifest)
        .map_err(|error| format!("could not write portable snapshot manifest: {error}"))?;
    let lifecycle = read_workspace_text(&root, constants::PORTABLE_LIFECYCLE_FILE)?;
    let report = format!(
        "{{\n  \"schema_version\": 1,\n  \"tree_sha256\": \"{tree_digest}\",\n  \"file_count\": {},\n  \"next_series\": \"{}\",\n  \"status\": \"review-required\"\n}}\n",
        files.len(),
        json_string(&configuration_value(&lifecycle, "next_series").unwrap_or_default())
    );
    fs::write(partial.join("migration-report.json"), report)
        .map_err(|error| format!("could not write portable migration report: {error}"))?;
    fs::rename(&partial, &output).map_err(|error| {
        format!(
            "could not publish portable snapshot {} as {}: {error}",
            partial.display(),
            output.display()
        )
    })?;
    crate::output::file("portable snapshot", &output);
    Ok(())
}

/// Builds sorted exact-byte digest records for active portable files.
pub(crate) fn portable_digest_records(root: &Path, files: &[PathBuf]) -> Result<String, String> {
    let mut records = String::new();
    for path in files {
        let bytes = fs::read(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("portable path escaped workspace: {error}"))?
            .to_string_lossy()
            .replace('\\', "/");
        writeln!(
            &mut records,
            "{}  {}  {relative}",
            sha256_hex(&bytes),
            bytes.len()
        )
        .expect("writing to a String cannot fail");
    }
    Ok(records)
}
