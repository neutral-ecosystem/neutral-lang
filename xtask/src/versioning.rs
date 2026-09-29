// SPDX-License-Identifier: Apache-2.0

//! Package release versioning, dependency lock, and inherited contract checks.

use super::*;

/// Executes the stable version command family.
pub(super) fn version(action: VersionAction) -> Result<(), String> {
    match action {
        VersionAction::Show => show_versions(),
        VersionAction::Check => check_versions(),
        VersionAction::Prepare(requested) => prepare_version(&requested),
    }
}

/// Displays the package version separately from every frozen contract version.
pub(super) fn show_versions() -> Result<(), String> {
    let root = workspace_root()?;
    let package = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    println!("{} package-release {package}", constants::INFO);
    let freeze = read_workspace_text(
        &root,
        &ReleasedBundle::load(&root)?.member("specs/contracts/freeze.toml"),
    )?;
    for (name, value) in configuration_section(&freeze, "contract_versions")? {
        println!("{} contract {name}={value}", constants::INFO);
    }
    Ok(())
}

/// Checks that package versions inherit the one workspace release version.
pub(super) fn check_versions() -> Result<(), String> {
    let root = workspace_root()?;
    let package_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    for manifest in workspace_package_manifests(&root)? {
        let content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        if !content
            .lines()
            .any(|line| line.trim() == "version.workspace = true")
        {
            return Err(format!(
                "workspace package must inherit version.workspace: {}",
                manifest.display()
            ));
        }
        for inherited in ["license.workspace = true", "repository.workspace = true"] {
            if !content.lines().any(|line| line.trim() == inherited) {
                return Err(format!(
                    "workspace package must inherit {inherited}: {}",
                    manifest.display()
                ));
            }
        }
    }
    verify_dependency_lock(&root, &package_version)?;
    let freeze = read_workspace_text(
        &root,
        &ReleasedBundle::load(&root)?.member("specs/contracts/freeze.toml"),
    )?;
    if configuration_value(&freeze, "status").as_deref() != Some("approved")
        || configuration_section(&freeze, "contract_versions")?.is_empty()
    {
        return Err("contract freeze must remain approved and version-complete".to_owned());
    }
    println!("{} centralized package versions: pass", constants::INFO);
    Ok(())
}

/// Rejects ordinary version work mixed with unreviewed normative changes.
pub(super) fn ensure_no_unreviewed_contract_changes(root: &Path) -> Result<(), String> {
    let bundle = ReleasedBundle::load(root)?;
    let freeze = bundle.member("specs/contracts/freeze.toml");
    let specs = bundle.member("specs");
    let manifest = bundle.member("conformance/manifest.toml");
    let oracles = bundle.member("conformance/oracles");
    let review = bundle.member("conformance/fixture-oracle-review.toml");
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "diff",
            "--name-only",
            "HEAD",
            "--",
            constants::CONFORMANCE_CONFIG_FILE,
            &freeze,
            &specs,
            &manifest,
            &oracles,
            &review,
            "config/ir-encoding.toml",
        ])
        .output()
        .map_err(|error| format!("could not inspect normative changes: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not inspect normative changes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let changed = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git emitted non-UTF-8 paths: {error}"))?;
    if changed.trim().is_empty() {
        Ok(())
    } else {
        Err(format!(
            "ordinary package-version work includes normative changes requiring contract-freeze review: {}",
            changed.lines().collect::<Vec<_>>().join(", ")
        ))
    }
}

/// Propagates one reviewed package-release version without tagging or publishing.
pub(super) fn prepare_version(requested: &str) -> Result<(), String> {
    validate_semver(requested)?;
    let root = workspace_root()?;
    let manifest_path = root.join(constants::WORKSPACE_MANIFEST_FILE);
    let manifest = read_workspace_text(&root, constants::WORKSPACE_MANIFEST_FILE)?;
    let current = workspace_package_version(&manifest)?;
    validate_version_transition(&current, requested)?;
    ensure_no_unreviewed_contract_changes(&root)?;
    check_versions()?;
    require_clean_checkout()?;

    let updated_manifest = replace_workspace_package_version(&manifest, &current, requested)?;
    let lock_path = root.join(constants::CARGO_LOCK_FILE);
    let lock = read_workspace_text(&root, constants::CARGO_LOCK_FILE)?;
    let package_names = workspace_package_names(&root)?;
    let updated_lock = replace_workspace_lock_versions(&lock, &package_names, &current, requested)?;
    let evidence_directory = root
        .join(constants::QUALITY_EVIDENCE_DIRECTORY)
        .join(format!("v{requested}"));
    let evidence_readme = evidence_directory.join("README.md");
    if evidence_readme.exists() {
        return Err(format!(
            "release evidence is already prepared: {}",
            evidence_readme.display()
        ));
    }

    let freeze_bytes =
        fs::read(root.join(ReleasedBundle::load(&root)?.member("specs/contracts/freeze.toml")))
            .map_err(|error| format!("could not read contract freeze: {error}"))?;
    let freeze_digest = sha256_hex(&freeze_bytes);

    fs::create_dir_all(&evidence_directory).map_err(|error| {
        format!(
            "could not create release evidence directory {}: {error}",
            evidence_directory.display()
        )
    })?;
    fs::write(&manifest_path, updated_manifest)
        .map_err(|error| format!("could not update {}: {error}", manifest_path.display()))?;
    fs::write(&lock_path, updated_lock)
        .map_err(|error| format!("could not update {}: {error}", lock_path.display()))?;
    fs::write(
        &evidence_readme,
        release_evidence_readme(requested, &project_license(&root)?),
    )
    .map_err(|error| format!("could not write {}: {error}", evidence_readme.display()))?;

    let directory = result_root()?.join(constants::VERSION_RESULT_DIRECTORY);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let output = directory.join(format!("prepare-{requested}.json"));
    fs::write(
        &output,
        format!(
            "{{\"schema_version\":1,\"current\":\"{}\",\"requested\":\"{}\",\"derived_updates\":[],\"contract_freeze_sha256\":\"{}\",\"frozen_contracts_changed\":false,\"actions\":[\"edit-workspace-package-version\",\"run-version-check\",\"review\"],\"status\":\"review-required\"}}\n",
            json_string(&current),
            json_string(requested),
            freeze_digest
        ),
    )
    .map_err(|error| format!("could not write {}: {error}", output.display()))?;
    println!(
        "{} package release prepared: {current} -> {requested}",
        constants::INFO
    );
    println!(
        "{} updated: {}, {}, {}",
        constants::INFO,
        constants::WORKSPACE_MANIFEST_FILE,
        constants::CARGO_LOCK_FILE,
        evidence_readme
            .strip_prefix(&root)
            .unwrap_or(&evidence_readme)
            .display()
    );
    println!("{} version plan: {}", constants::INFO, output.display());
    Ok(())
}

/// Replaces only the workspace package version in the root manifest.
pub(super) fn replace_workspace_package_version(
    manifest: &str,
    current: &str,
    requested: &str,
) -> Result<String, String> {
    let mut selected = false;
    let mut replaced = false;
    let expected = format!("version = \"{current}\"");
    let replacement = format!("version = \"{requested}\"");
    let mut lines = Vec::new();

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            selected = trimmed == "[workspace.package]";
        }
        if selected && trimmed == expected {
            let indentation = line.len() - line.trim_start().len();
            lines.push(format!("{}{}", &line[..indentation], replacement));
            replaced = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        return Err("root Cargo.toml has no matching workspace package version".to_owned());
    }
    Ok(format!("{}\n", lines.join("\n")))
}

/// Replaces each workspace package version in Cargo's lockfile without touching dependencies.
pub(super) fn replace_workspace_lock_versions(
    lock: &str,
    package_names: &BTreeSet<String>,
    current: &str,
    requested: &str,
) -> Result<String, String> {
    let mut updated = lock.to_owned();
    for package_name in package_names {
        let expected = format!("name = \"{package_name}\"\nversion = \"{current}\"");
        let replacement = format!("name = \"{package_name}\"\nversion = \"{requested}\"");
        if !updated.contains(&expected) {
            return Err(format!(
                "Cargo.lock is stale for workspace package {package_name} {current}"
            ));
        }
        updated = updated.replacen(&expected, &replacement, 1);
    }
    Ok(updated)
}

/// Renders the durable release-evidence scaffold required before quality approval.
pub(super) fn release_evidence_readme(version: &str, license: &str) -> String {
    format!(
        "{}\n\n# Neutral v{version} quality evidence\n\nThis directory is prepared by `cargo xtask version prepare {version}`. Keep the\nrelease-quality approval record immutable once `cargo xtask quality approve --release\n{version}` succeeds.\n",
        html_spdx_marker(license)
    )
}

/// Checks the root release lock and its declared dependency-source policy.
pub(super) fn verify_dependency_lock(root: &Path, package_version: &str) -> Result<(), String> {
    let policy = read_workspace_text(root, constants::DEPENDENCY_SOURCES_FILE)?;
    if configuration_value(&policy, "lockfile").as_deref() != Some(constants::CARGO_LOCK_FILE)
        || configuration_value(&policy, "allow_crates_io_registry").as_deref() != Some("true")
        || configuration_value(&policy, "allow_git_sources").as_deref() != Some("false")
        || configuration_value(&policy, "allow_external_paths").as_deref() != Some("false")
    {
        return Err("dependency-source policy is incomplete or unsafe".to_owned());
    }
    let review_path = configuration_value(&policy, "review")
        .ok_or_else(|| "dependency-source policy has no review path".to_owned())?;
    if !is_safe_relative_path(Path::new(&review_path)) {
        return Err("dependency-source review path must stay within the workspace".to_owned());
    }
    let review = read_workspace_text(root, &review_path)?;
    if !review.contains("Result: pass for the current lockfile") || !review.contains("cargo audit")
    {
        return Err("dependency and advisory review is absent or not passing".to_owned());
    }
    let lock = read_workspace_text(root, constants::CARGO_LOCK_FILE)?;
    for package in lock.split("[[package]]").skip(1) {
        if package.contains("source = \"git+") {
            return Err("Cargo.lock contains a forbidden Git dependency".to_owned());
        }
        if package.contains("source = \"registry+") && !package.contains("checksum = \"") {
            return Err("Cargo.lock contains a registry package without a checksum".to_owned());
        }
    }
    for manifest in workspace_package_manifests(root)? {
        verify_manifest_dependency_paths(root, &manifest)?;
        let package_content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        let package_name = configuration::quality_value_from(&package_content, "package", "name")
            .ok_or_else(|| {
            format!(
                "package manifest has no package name: {}",
                manifest.display()
            )
        })?;
        let expected = format!("name = \"{package_name}\"\nversion = \"{package_version}\"");
        if !lock.contains(&expected) {
            return Err(format!(
                "Cargo.lock is stale for workspace package {package_name} {package_version}"
            ));
        }
    }
    let output = Command::new(cargo_command()?)
        .current_dir(root)
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ])
        .output()
        .map_err(|error| format!("could not validate Cargo.lock: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Cargo.lock is stale or unavailable offline: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.lines().any(|line| line.contains("warning:")) {
        return Err(format!(
            "Cargo metadata emitted a release-relevant warning: {}",
            stderr.trim()
        ));
    }
    Ok(())
}

/// Requires every manifest path dependency to resolve inside the workspace.
pub(super) fn verify_manifest_dependency_paths(root: &Path, manifest: &Path) -> Result<(), String> {
    let content = fs::read_to_string(manifest)
        .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("could not canonicalize workspace root: {error}"))?;
    for tail in content.split("path = \"").skip(1) {
        let dependency = tail
            .split_once('"')
            .map(|(value, _)| value)
            .ok_or_else(|| format!("malformed path dependency in {}", manifest.display()))?;
        let path = manifest.parent().unwrap_or(root).join(dependency);
        let canonical = fs::canonicalize(&path).map_err(|error| {
            format!(
                "could not resolve path dependency {}: {error}",
                path.display()
            )
        })?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "manifest path dependency escapes the workspace: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Reads the root workspace package version from its exact TOML section.
pub(super) fn workspace_package_version(manifest: &str) -> Result<String, String> {
    let version = workspace_package_value(manifest, "version")?;
    validate_semver(&version)?;
    Ok(version)
}

/// Reads the root workspace package license identifier.
pub(super) fn workspace_package_license(manifest: &str) -> Result<String, String> {
    let license = workspace_package_value(manifest, "license")?;
    if license.is_empty() {
        return Err("root Cargo.toml has an empty [workspace.package] license".to_owned());
    }
    Ok(license)
}

/// Reads one quoted value from the root workspace package section.
pub(super) fn workspace_package_value(manifest: &str, key: &str) -> Result<String, String> {
    let mut selected = false;
    let prefix = format!("{key} = \"");
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            selected = line == "[workspace.package]";
            continue;
        }
        if selected
            && let Some(value) = line
                .strip_prefix(&prefix)
                .and_then(|line| line.strip_suffix('"'))
        {
            return Ok(value.to_owned());
        }
    }
    Err(format!("root Cargo.toml has no [workspace.package] {key}"))
}

/// Reads the project license identifier from the root workspace manifest.
pub(super) fn project_license(root: &Path) -> Result<String, String> {
    workspace_package_license(&read_workspace_text(
        root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)
}

/// Derives a safe distribution name from the workspace repository URL.
pub(super) fn project_slug(root: &Path) -> Result<String, String> {
    let manifest = read_workspace_text(root, constants::WORKSPACE_MANIFEST_FILE)?;
    let repository = workspace_package_value(&manifest, "repository")?;
    let slug = repository
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim_end_matches(".git");
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("workspace repository has no safe distribution name".to_owned());
    }
    Ok(slug.to_owned())
}

/// Returns the line-comment SPDX marker for one license identifier.
pub(super) fn line_spdx_marker(license: &str) -> String {
    format!("# SPDX-License-Identifier: {license}")
}

/// Returns the HTML-comment SPDX marker for one license identifier.
pub(super) fn html_spdx_marker(license: &str) -> String {
    format!("<!-- SPDX-License-Identifier: {license} -->")
}

/// Returns every non-root workspace package manifest.
pub(super) fn workspace_package_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let workspace = read_workspace_text(root, constants::WORKSPACE_MANIFEST_FILE)?;
    let members = configuration_array_from(&workspace, "workspace", "members")?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("could not canonicalize workspace root: {error}"))?;
    let mut manifests = BTreeSet::new();
    for member in members {
        let relative = Path::new(&member);
        if !is_safe_relative_path(relative) {
            return Err(format!("workspace member path is unsafe: {member}"));
        }
        let manifest = root.join(relative).join("Cargo.toml");
        let canonical = fs::canonicalize(&manifest)
            .map_err(|error| format!("could not resolve workspace member {member}: {error}"))?;
        if !canonical.starts_with(&canonical_root) || !manifests.insert(canonical) {
            return Err(format!(
                "workspace member is external or repeated: {member}"
            ));
        }
    }
    Ok(manifests.into_iter().collect())
}

/// Returns package names declared by the selected workspace member manifests.
pub(super) fn workspace_package_names(root: &Path) -> Result<BTreeSet<String>, String> {
    workspace_package_manifests(root)?
        .into_iter()
        .map(|manifest| {
            let content = fs::read_to_string(&manifest)
                .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
            configuration::quality_value_from(&content, "package", "name").ok_or_else(|| {
                format!(
                    "package manifest has no package name: {}",
                    manifest.display()
                )
            })
        })
        .collect()
}

/// Resolves a workspace package directory by its declared Cargo package name.
pub(super) fn workspace_package_directory(root: &Path, name: &str) -> Result<PathBuf, String> {
    for manifest in workspace_package_manifests(root)? {
        let content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        if configuration::quality_value_from(&content, "package", "name").as_deref() == Some(name) {
            return manifest
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| format!("workspace package {name} has no directory"));
        }
    }
    Err(format!("workspace has no package named {name}"))
}

/// Collects regular files with one exact filename below a directory.
pub(super) fn collect_named_files(
    directory: &Path,
    name: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect directory entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_named_files(&path, name, files)?;
        } else if path.file_name().and_then(std::ffi::OsStr::to_str) == Some(name) {
            files.push(path);
        }
    }
    Ok(())
}

/// Validates the supported numeric `SemVer` core with an optional prerelease suffix.
pub(super) fn validate_semver(value: &str) -> Result<(), String> {
    let (core, suffix) = value
        .split_once('-')
        .map_or((value, None), |(core, suffix)| (core, Some(suffix)));
    let components = core.split('.').collect::<Vec<_>>();
    let numeric = components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty()
                && component.bytes().all(|byte| byte.is_ascii_digit())
                && (component == &"0" || !component.starts_with('0'))
        });
    let valid_suffix = suffix.is_none_or(|suffix| {
        !suffix.is_empty()
            && suffix.split('.').all(|identifier| {
                !identifier.is_empty()
                    && identifier
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    && (!identifier.bytes().all(|byte| byte.is_ascii_digit())
                        || identifier == "0"
                        || !identifier.starts_with('0'))
            })
    });
    if numeric && valid_suffix {
        Ok(())
    } else {
        Err(format!("invalid package SemVer: {value}"))
    }
}

/// Rejects package-version downgrades and invalid prerelease transitions.
pub(super) fn validate_version_transition(current: &str, requested: &str) -> Result<(), String> {
    let current = parsed_semver(current)?;
    let requested = parsed_semver(requested)?;
    if compare_semver(&requested, &current) != std::cmp::Ordering::Greater {
        return Err(
            "requested package version must be greater than the current version".to_owned(),
        );
    }
    Ok(())
}

/// Parses supported `SemVer` into numeric core and prerelease identifiers.
pub(super) fn parsed_semver(value: &str) -> Result<(u64, u64, u64, Vec<String>), String> {
    validate_semver(value)?;
    let (core, prerelease) = value
        .split_once('-')
        .map_or((value, ""), |(core, prerelease)| (core, prerelease));
    let mut numbers = core.split('.').map(|value| {
        value
            .parse::<u64>()
            .map_err(|error| format!("invalid package SemVer component: {error}"))
    });
    let parsed = (
        numbers.next().transpose()?.unwrap_or_default(),
        numbers.next().transpose()?.unwrap_or_default(),
        numbers.next().transpose()?.unwrap_or_default(),
        prerelease
            .split('.')
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
    );
    Ok(parsed)
}

/// Compares two parsed `SemVer` values using numeric prerelease precedence.
pub(super) fn compare_semver(
    left: &(u64, u64, u64, Vec<String>),
    right: &(u64, u64, u64, Vec<String>),
) -> std::cmp::Ordering {
    let core = (left.0, left.1, left.2).cmp(&(right.0, right.1, right.2));
    if core != std::cmp::Ordering::Equal {
        return core;
    }
    match (left.3.is_empty(), right.3.is_empty()) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (true, true) => std::cmp::Ordering::Equal,
        (false, false) => compare_prerelease(&left.3, &right.3),
    }
}

/// Compares `SemVer` prerelease identifier sequences.
pub(super) fn compare_prerelease(left: &[String], right: &[String]) -> std::cmp::Ordering {
    for (left, right) in left.iter().zip(right) {
        let ordering = match (left.parse::<u64>(), right.parse::<u64>()) {
            (Ok(left), Ok(right)) => left.cmp(&right),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => left.cmp(right),
        };
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

/// Returns the lowercase SHA-256 digest of exact bytes.
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}
