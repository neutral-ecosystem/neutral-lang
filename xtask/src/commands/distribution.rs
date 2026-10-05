// SPDX-License-Identifier: Apache-2.0

//! commands / distribution responsibilities for repository automation.

use crate::constants::flags;
use crate::release::release_metadata::{BuildProvenance, ReleaseArtifact, ReleaseManifest};
use crate::{
    BTreeMap, Command, Path, PathBuf, QualityProfile, ValidationTarget, cargo_target_directory,
    check_boundaries, command_output, constants, fs, html_spdx_marker, output, project_license,
    project_slug, quality, release, release_metadata, release_plan, require_main_head_checkout,
    result_root, run_cargo, run_program, run_recorded_workflow, rustc_command, sha256_hex,
    verify_release_approval, workspace_root,
};

/// Validates either built release binaries or one encoded artifact.
pub(crate) fn validate(target: ValidationTarget) -> Result<(), String> {
    match target {
        ValidationTarget::Binaries => validate_release_binaries(
            &cargo_target_directory()?.join("release"),
            &release_plan()?.binaries,
        ),
        ValidationTarget::Artifact(path) => {
            if !path.is_file() {
                return Err(format!("artifact does not exist: {}", path.display()));
            }
            let probe = release_binary_path(
                &cargo_target_directory()?.join("release"),
                constants::NEUTRAL_PROBE,
            );
            if !probe.is_file() {
                return Err(format!(
                    "release probe is missing: {}; run `cargo xtask build --profile release`",
                    probe.display()
                ));
            }
            run_program(&probe, &[path.as_os_str()])
        }
    }
}

/// Validates release-mode CLI and probe entry points plus the probe boundary.
pub(crate) fn validate_release_binaries(
    directory: &Path,
    binaries: &[String],
) -> Result<(), String> {
    for binary in binaries {
        let path = release_binary_path(directory, binary);
        if !path.is_file() {
            return Err(format!(
                "release binary is missing: {}; run `cargo xtask build --profile release`",
                path.display()
            ));
        }
        run_program(&path, &[std::ffi::OsStr::new("--help")])?;
    }
    check_boundaries()?;
    crate::output::pass("release binaries");
    Ok(())
}

/// Returns a platform-correct release binary path.
pub(crate) fn release_binary_path(directory: &Path, binary: &str) -> PathBuf {
    let extension = if cfg!(windows) { ".exe" } else { "" };
    directory.join(format!("{binary}{extension}"))
}

/// One immutable generated distribution file.
pub(crate) struct DistributionAsset {
    /// Plain filename beneath the release package directory.
    pub(crate) filename: String,
    /// Exact file bytes.
    pub(crate) bytes: Vec<u8>,
}

/// Shared provenance fields applied to every release-manifest artifact entry.
pub(crate) struct ReleaseEntryContext<'a> {
    /// Workspace package version that produced the artifact.
    pub(crate) version: &'a str,
    /// Exact source commit from which the artifact was produced.
    pub(crate) candidate_commit: &'a str,
    /// Project license expression inherited from the workspace manifest.
    pub(crate) license: &'a str,
}

/// Assembles the selected binary distribution into the ignored release root.
pub(crate) fn package() -> Result<(), String> {
    let plan = release_plan()?;
    let candidate_commit = require_main_head_checkout()?;
    if !plan
        .channels
        .contains(&release::DistributionChannel::GithubBinaries)
    {
        return Err("GitHub binary distribution is not selected".to_owned());
    }
    for binary in &plan.binaries {
        run_cargo(&["build", "--release", flags::LOCKED, flags::PACKAGE, binary])?;
    }
    let root = workspace_root()?;
    let host = rust_host()?;
    let source_directory = cargo_target_directory()?.join("release");
    validate_release_binaries(&source_directory, &plan.binaries)?;
    let output_directory = release_metadata::package_output_directory(
        &result_root()?,
        &plan.release_tag,
        &candidate_commit,
        &host,
    );
    let summary =
        release_metadata::package_summary_json(&plan.release_tag, &candidate_commit, &host)?;
    let assets = release_distribution_assets(
        &root,
        &source_directory,
        &plan,
        &candidate_commit,
        &host,
        &summary,
    )?;
    stage_binary_package(
        &root,
        &source_directory,
        &output_directory,
        &plan.binaries,
        &assets,
        &summary,
    )?;
    output::file("package assembled", &output_directory);
    Ok(())
}

/// Generates the source archive, SBOM, provenance, installation, manifest, and checksums.
pub(crate) fn release_distribution_assets(
    root: &Path,
    source_directory: &Path,
    plan: &release::ReleasePlan,
    candidate_commit: &str,
    host: &str,
    package_summary: &str,
) -> Result<Vec<DistributionAsset>, String> {
    let version = plan.release_tag.trim_start_matches('v');
    let license = project_license(root)?;
    let artifact_name = project_slug(root)?;
    let license_marker = html_spdx_marker(&license);
    let entry_context = ReleaseEntryContext {
        version,
        candidate_commit,
        license: &license,
    };
    let source_name = format!("{artifact_name}-{}-source.tar", plan.release_tag);
    let lock_bytes = fs::read(root.join(constants::CARGO_LOCK_FILE))
        .map_err(|error| format!("could not read release dependency lock: {error}"))?;
    let mut assets = vec![
        DistributionAsset {
            filename: source_name,
            bytes: source_archive(root, &artifact_name, version, candidate_commit)?,
        },
        DistributionAsset {
            filename: constants::RELEASE_SBOM_FILE.to_owned(),
            bytes: lock_bytes.clone(),
        },
        DistributionAsset {
            filename: constants::RELEASE_INSTALL_FILE.to_owned(),
            bytes: release_install_guide(
                &license_marker,
                &artifact_name,
                version,
                host,
                &plan.binaries,
            ),
        },
        DistributionAsset {
            filename: constants::RELEASE_PROVENANCE_FILE.to_owned(),
            bytes: release_provenance_bytes(plan, candidate_commit, host, &lock_bytes)?,
        },
    ];
    let mut entries = Vec::new();
    let mut checksums = Vec::new();
    append_binary_release_entries(
        &mut entries,
        &mut checksums,
        source_directory,
        &plan.binaries,
        &entry_context,
    )?;
    for asset in &assets {
        let channel = if asset.filename.ends_with("-source.tar") {
            "source-tag"
        } else {
            "github-release-metadata"
        };
        append_release_entry(
            &mut entries,
            &mut checksums,
            &asset.filename,
            &asset.bytes,
            channel,
            &entry_context,
        );
    }
    let summary_name = constants::RELEASE_PACKAGE_SUMMARY_FILE;
    for (filename, bytes) in [
        (
            constants::LICENSE_FILE,
            fs::read(root.join(constants::LICENSE_FILE))
                .map_err(|error| format!("could not hash LICENSE: {error}"))?,
        ),
        (
            constants::ROOT_README_FILE,
            fs::read(root.join(constants::ROOT_README_FILE))
                .map_err(|error| format!("could not hash README: {error}"))?,
        ),
        (summary_name, package_summary.as_bytes().to_vec()),
    ] {
        append_release_entry(
            &mut entries,
            &mut checksums,
            filename,
            &bytes,
            "github-release-metadata",
            &entry_context,
        );
    }
    let manifest = release_manifest_bytes(plan, &entry_context, host, &entries)?;
    checksums.push(format!(
        "{}  {}\n",
        sha256_hex(&manifest),
        constants::RELEASE_MANIFEST_FILE
    ));
    assets.push(DistributionAsset {
        filename: constants::RELEASE_MANIFEST_FILE.to_owned(),
        bytes: manifest,
    });
    checksums.sort();
    assets.push(DistributionAsset {
        filename: constants::RELEASE_CHECKSUM_FILE.to_owned(),
        bytes: checksums.concat().into_bytes(),
    });
    Ok(assets)
}

/// Serializes exact build inputs without concatenating user-supplied JSON fragments.
fn release_provenance_bytes(
    plan: &release::ReleasePlan,
    candidate_commit: &str,
    host: &str,
    lock_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    crate::runtime::json::pretty(&BuildProvenance {
        schema_version: release_metadata::RELEASE_METADATA_SCHEMA,
        builder: "cargo xtask package",
        candidate_ref: release_metadata::CANDIDATE_REF,
        candidate_commit,
        release_tag: &plan.release_tag,
        target: host,
        rustc: command_output(&rustc_command()?, &[flags::VERSION])?,
        cargo_lock_sha256: sha256_hex(lock_bytes),
        reproducible_command: "cargo xtask package",
    })
}

/// Hashes the selected built binaries into one release manifest and checksum set.
pub(crate) fn append_binary_release_entries(
    entries: &mut Vec<ReleaseArtifact>,
    checksums: &mut Vec<String>,
    source_directory: &Path,
    binaries: &[String],
    context: &ReleaseEntryContext<'_>,
) -> Result<(), String> {
    for binary in binaries {
        let filename = release_binary_path(Path::new(""), binary)
            .to_string_lossy()
            .into_owned();
        let bytes = fs::read(release_binary_path(source_directory, binary))
            .map_err(|error| format!("could not hash release binary {binary}: {error}"))?;
        append_release_entry(
            entries,
            checksums,
            &filename,
            &bytes,
            "github-binaries",
            context,
        );
    }
    Ok(())
}

/// Renders the installation instructions for the exact selected binary set.
pub(crate) fn release_install_guide(
    license_marker: &str,
    artifact_name: &str,
    version: &str,
    host: &str,
    binaries: &[String],
) -> Vec<u8> {
    let programs = binaries
        .iter()
        .map(|binary| format!("`{binary}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{license_marker}\n\n# Install {artifact_name} {version}\n\nSupported target: `{host}`. Verify the downloaded files with `sha256sum --check SHA256SUMS`, install {programs} into a directory on `PATH`, then run each installed binary with `--help`.\n"
    )
    .into_bytes()
}

/// Renders metadata from the selected channels and assembled artifact records.
pub(crate) fn release_manifest_bytes(
    plan: &release::ReleasePlan,
    context: &ReleaseEntryContext<'_>,
    host: &str,
    entries: &[ReleaseArtifact],
) -> Result<Vec<u8>, String> {
    let crates_io_selected = plan
        .channels
        .contains(&release::DistributionChannel::CratesIo);
    let mut deferred = vec!["additional host targets"];
    if !crates_io_selected {
        deferred.push("crates.io publication");
    }
    crate::runtime::json::pretty(&ReleaseManifest {
        schema_version: release_metadata::RELEASE_METADATA_SCHEMA,
        release_tag: &plan.release_tag,
        candidate_ref: release_metadata::CANDIDATE_REF,
        candidate_commit: context.candidate_commit,
        license: context.license,
        supported_targets: [host],
        crates_io_selected,
        known_limitations: [
            "single-host binary package",
            "no runtime or application semantics",
        ],
        deferred,
        artifacts: entries,
    })
}

/// Produces deterministic tracked source bytes for one exact candidate commit.
pub(crate) fn source_archive(
    root: &Path,
    artifact_name: &str,
    version: &str,
    candidate_commit: &str,
) -> Result<Vec<u8>, String> {
    let archive = Command::new("git")
        .current_dir(root)
        .args([
            "archive",
            "--format=tar",
            &format!("--prefix={artifact_name}-{version}/"),
            candidate_commit,
        ])
        .output()
        .map_err(|error| format!("could not create source archive: {error}"))?;
    if archive.status.success() {
        Ok(archive.stdout)
    } else {
        Err(format!(
            "git archive failed: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        ))
    }
}

/// Adds one selected file to the release manifest and checksum list.
pub(crate) fn append_release_entry(
    entries: &mut Vec<ReleaseArtifact>,
    checksums: &mut Vec<String>,
    filename: &str,
    bytes: &[u8],
    channel: &str,
    context: &ReleaseEntryContext<'_>,
) {
    let digest = sha256_hex(bytes);
    checksums.push(format!("{digest}  {filename}\n"));
    entries.push(ReleaseArtifact {
        filename: filename.to_owned(),
        sha256: digest,
        license: context.license.to_owned(),
        producer_version: context.version.to_owned(),
        source_commit: context.candidate_commit.to_owned(),
        channel: channel.to_owned(),
    });
}

/// Atomically stages selected binaries, license material, and package metadata.
pub(crate) fn stage_binary_package(
    root: &Path,
    source_directory: &Path,
    output_directory: &Path,
    binaries: &[String],
    assets: &[DistributionAsset],
    summary: &str,
) -> Result<(), String> {
    if output_directory.exists() {
        return verify_existing_binary_package(
            root,
            source_directory,
            output_directory,
            binaries,
            assets,
            summary,
        );
    }
    let parent = output_directory
        .parent()
        .ok_or_else(|| "package output has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let partial_directory = parent.join(format!(".partial-{}", std::process::id()));
    if partial_directory.exists() {
        return Err(format!(
            "partial package output already exists: {}; run `cargo xtask clean`",
            partial_directory.display()
        ));
    }
    fs::create_dir(&partial_directory)
        .map_err(|error| format!("could not create {}: {error}", partial_directory.display()))?;
    for binary in binaries {
        let source = release_binary_path(source_directory, binary);
        let destination = release_binary_path(&partial_directory, binary);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    for file in [constants::LICENSE_FILE, constants::ROOT_README_FILE] {
        fs::copy(root.join(file), partial_directory.join(file))
            .map_err(|error| format!("could not stage {file}: {error}"))?;
    }
    for asset in assets {
        fs::write(partial_directory.join(&asset.filename), &asset.bytes)
            .map_err(|error| format!("could not stage {}: {error}", asset.filename))?;
    }
    fs::write(
        partial_directory.join(constants::RELEASE_PACKAGE_SUMMARY_FILE),
        summary,
    )
    .map_err(|error| format!("could not write package summary: {error}"))?;
    fs::rename(&partial_directory, output_directory).map_err(|error| {
        format!(
            "could not publish staged package {} as {}: {error}",
            partial_directory.display(),
            output_directory.display()
        )
    })?;
    Ok(())
}

/// Reuses an existing package only when its exact file set and bytes still match.
pub(crate) fn verify_existing_binary_package(
    root: &Path,
    source_directory: &Path,
    output_directory: &Path,
    binaries: &[String],
    assets: &[DistributionAsset],
    summary: &str,
) -> Result<(), String> {
    let output_type = fs::symlink_metadata(output_directory)
        .map_err(|error| format!("could not inspect {}: {error}", output_directory.display()))?
        .file_type();
    if !output_type.is_dir() {
        return Err(format!(
            "existing package output is not a regular directory: {}",
            output_directory.display()
        ));
    }
    let mut expected = BTreeMap::new();
    for binary in binaries {
        let filename = release_binary_path(Path::new(""), binary)
            .to_string_lossy()
            .into_owned();
        let bytes = fs::read(release_binary_path(source_directory, binary))
            .map_err(|error| format!("could not read release binary {binary}: {error}"))?;
        if expected.insert(filename, bytes).is_some() {
            return Err(format!("duplicate release binary: {binary}"));
        }
    }
    for filename in [constants::LICENSE_FILE, constants::ROOT_README_FILE] {
        let bytes = fs::read(root.join(filename))
            .map_err(|error| format!("could not read release {filename}: {error}"))?;
        expected.insert(filename.to_owned(), bytes);
    }
    for asset in assets {
        if expected
            .insert(asset.filename.clone(), asset.bytes.clone())
            .is_some()
        {
            return Err(format!("duplicate release asset: {}", asset.filename));
        }
    }
    expected.insert(
        constants::RELEASE_PACKAGE_SUMMARY_FILE.to_owned(),
        summary.as_bytes().to_vec(),
    );

    for entry in fs::read_dir(output_directory)
        .map_err(|error| format!("could not inspect {}: {error}", output_directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect package entry: {error}"))?;
        let filename = entry.file_name().to_string_lossy().into_owned();
        if !entry
            .file_type()
            .map_err(|error| format!("could not inspect package {filename}: {error}"))?
            .is_file()
        {
            return Err(format!("package entry is not a regular file: {filename}"));
        }
        let expected_bytes = expected
            .remove(&filename)
            .ok_or_else(|| format!("unexpected package file: {filename}"))?;
        let actual_bytes = fs::read(entry.path())
            .map_err(|error| format!("could not read package file {filename}: {error}"))?;
        if actual_bytes != expected_bytes {
            return Err(format!(
                "existing package file differs from current build: {filename}"
            ));
        }
    }
    if !expected.is_empty() {
        return Err(format!(
            "existing package is incomplete: {}",
            expected.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    crate::output::pass("existing package verified");
    crate::output::file("package", output_directory);
    Ok(())
}

/// Runs release checks and assembles artifacts without tagging or publishing.
pub(crate) fn release_qualify() -> Result<(), String> {
    run_recorded_workflow(
        "release",
        "qualify",
        vec![
            ("release-plan", Box::new(|| release_plan().map(drop))),
            (
                "main-head",
                Box::new(|| require_main_head_checkout().map(drop)),
            ),
            ("approval", Box::new(verify_release_approval)),
            (
                "measurements",
                Box::new(crate::release::lifecycle::ensure_measurements),
            ),
            ("quality", Box::new(|| quality(QualityProfile::Release))),
            ("package", Box::new(package)),
        ],
    )?;
    let plan = release_plan()?;
    crate::output::pass(format!(
        "release {} qualified and packaged; no publish action was performed",
        plan.release_tag
    ));
    Ok(())
}

/// Returns the host triple reported by the selected Rust compiler.
pub(crate) fn rust_host() -> Result<String, String> {
    let verbose = command_output(&rustc_command()?, &["-vV"])?;
    verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -vV did not report a host triple".to_owned())
}

#[cfg(test)]
#[path = "../../tests/unit/distribution.rs"]
mod tests;
