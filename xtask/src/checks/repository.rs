// SPDX-License-Identifier: Apache-2.0

//! checks / repository responsibilities for repository automation.

use crate::{
    BTreeSet, Command, Path, ReleasedBundle, collect_regular_files, configuration,
    configuration_value, constants, fs, git_hygiene, html_spdx_marker, interface,
    is_safe_relative_path, markdown_link_exists, markdown_link_targets, project_license,
    read_workspace_text, repository_directories, workspace_package_manifests, workspace_root,
};

/// Verifies every tracked repository-local Markdown link stays within the workspace.
pub(crate) fn verify_repository_markdown_links() -> Result<(), String> {
    let root = workspace_root()?;
    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("could not canonicalize workspace root: {error}"))?;
    let mut files = vec![root.join(constants::ROOT_README_FILE)];
    for directory in repository_directories(&root)? {
        collect_regular_files(&root.join(directory.path), &mut files)?;
    }
    let mut failures = Vec::new();
    for file in files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
    {
        let content = fs::read_to_string(file)
            .map_err(|error| format!("could not read {}: {error}", file.display()))?;
        for target in markdown_link_targets(&content) {
            let valid = markdown_link_exists(&root, &canonical_root, file, &target)?;
            if !valid {
                failures.push(format!(
                    "{} -> {target}",
                    file.strip_prefix(&root).unwrap_or(file).display()
                ));
            }
        }
    }
    if failures.is_empty() {
        crate::output::pass("repository Markdown links");
        Ok(())
    } else {
        failures.sort();
        failures.dedup();
        Err(format!(
            "repository Markdown links are missing or escape the workspace: {}",
            failures.join(", ")
        ))
    }
}

/// Verifies command documentation, platform adapters, and CI stay synchronized.
pub(crate) fn check_workflow_contract() -> Result<(), String> {
    let root = workspace_root()?;
    for relative in ["README.md"] {
        let content = read_workspace_text(&root, relative)?;
        for command in interface::DOCUMENTED_COMMANDS {
            if !content.contains(command) {
                return Err(format!("{relative} does not document `{command}`"));
            }
        }
    }

    let ci = read_workspace_text(&root, ".github/workflows/ci.yml")?;
    if !ci.contains("cargo xtask ci pr") {
        return Err("ci.yml does not delegate to `cargo xtask ci pr`".to_owned());
    }
    let retained_workflows = format!(
        "{}/workflows",
        crate::configuration::automation_at(&root)?
            .output
            .results_root
    );
    for retention_requirement in [
        "if: always()",
        "actions/upload-artifact@",
        &retained_workflows,
    ] {
        if !ci.contains(retention_requirement) {
            return Err(format!(
                "ci.yml does not retain workflow evidence with `{retention_requirement}`"
            ));
        }
    }
    let release = read_workspace_text(&root, ".github/workflows/release.yml")?;
    if !release.contains("cargo xtask release qualify") {
        return Err("release.yml does not delegate to `cargo xtask release qualify`".to_owned());
    }
    for requirement in [
        "tags: ['v*']",
        "github.event_name == 'push' && github.ref || 'main'",
        "Verify tag identifies main HEAD",
        "git switch -C main",
        "cargo xtask release tag",
        "sha256sum --check SHA256SUMS",
        "contents: write",
        "if: github.event_name == 'push'",
        "gh release create",
        "--verify-tag",
        "--draft",
    ] {
        if !release.contains(requirement) {
            return Err(format!(
                "release.yml does not enforce publication requirement `{requirement}`"
            ));
        }
    }
    if ci.contains("pull_request:") || ci.contains("contents: write") {
        return Err("push CI must not expose release credentials or write permission".to_owned());
    }
    if ci.contains("cargo xtask docs") || ci.contains("cargo xtask quality") {
        return Err("ci.yml must use the single logged CI composition".to_owned());
    }

    for (relative, command) in [
        ("scripts/linux/bootstrap.sh", "xtask bootstrap"),
        ("scripts/linux/environment.sh", "xtask environment"),
        ("scripts/linux/release.sh", "xtask release prepare"),
        ("scripts/win/bootstrap.ps1", "xtask bootstrap"),
        ("scripts/win/environment.ps1", "xtask environment"),
        ("scripts/win/release.ps1", "xtask release prepare"),
    ] {
        if !read_workspace_text(&root, relative)?.contains(command) {
            return Err(format!("{relative} does not delegate to `{command}`"));
        }
    }
    crate::output::pass("stable workflow contract");
    Ok(())
}

/// Rejects production or executable-test references to archived portable inputs.
pub(crate) fn ensure_no_portable_archive_dependencies(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    let mut production_roots = Vec::new();
    for manifest in workspace_package_manifests(root)? {
        let package_root = manifest
            .parent()
            .ok_or_else(|| format!("workspace manifest has no parent: {}", manifest.display()))?;
        collect_regular_files(package_root, &mut files)?;
        let content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        if configuration::quality_value_from(&content, "package", "name").as_deref()
            != Some(constants::XTASK)
        {
            production_roots.push(package_root.to_path_buf());
        }
    }
    let forbidden = [
        concat!("portable", "/archive/"),
        concat!("portable", "/archived/"),
    ];
    let violations = files
        .iter()
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("rs" | "toml")
            )
        })
        .filter_map(|path| {
            let content = fs::read_to_string(path).ok()?;
            let uses_active_portable = production_roots
                .iter()
                .any(|package_root| path.starts_with(package_root))
                && content.contains("portable/");
            (uses_active_portable || forbidden.iter().any(|value| content.contains(value))).then(
                || {
                    path.strip_prefix(root)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                },
            )
        })
        .collect::<Vec<_>>();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "code depends on archived portable material: {}",
            violations.join(", ")
        ))
    }
}

/// Verifies generated-output ownership and ignored tracking policy.
pub(crate) fn check_generated_outputs() -> Result<(), String> {
    let root = workspace_root()?;
    let inventory = read_workspace_text(&root, constants::GENERATED_OUTPUTS_FILE)?;
    let result_directory = crate::configuration::automation_at(&root)?
        .output
        .results_root;
    if !is_safe_relative_path(Path::new(&result_directory)) {
        return Err("configured generated-results root is not a safe relative path".to_owned());
    }
    for name in [
        "rustdoc",
        "coverage",
        "quality-measurements",
        "package",
        "workflow-runs",
        "portable-snapshot",
        "portable-rejected",
        "quality-evaluation",
    ] {
        if !inventory.contains(&format!("name = \"{name}\"")) {
            return Err(format!("generated-output inventory is missing {name}"));
        }
    }
    for entry in inventory.split("[[output]]").skip(1) {
        let path = configuration_value(entry, "path")
            .ok_or_else(|| "generated output has no path".to_owned())?;
        let tracking = configuration_value(entry, "tracking");
        let ephemeral_output = (path.starts_with("target/")
            || path.starts_with(&format!("{result_directory}/")))
            && tracking.as_deref() == Some("ignored");
        if !ephemeral_output
            || configuration_value(entry, "owner").is_none()
            || configuration_value(entry, "generate").is_none()
            || configuration_value(entry, "validate").is_none()
        {
            return Err(format!(
                "generated output {path} has an unsafe or incomplete policy"
            ));
        }
    }
    let ignore = read_workspace_text(&root, ".gitignore")?;
    if !ignore.lines().any(|line| line.trim() == "target")
        || !ignore
            .lines()
            .any(|line| line.trim() == format!("{result_directory}/"))
    {
        return Err("Cargo target and configured results must remain ignored".to_owned());
    }
    crate::output::pass("generated-output ownership");
    Ok(())
}

/// Verifies the categorized inventory of durable quality documents.
pub(crate) fn check_quality_inventory() -> Result<(), String> {
    let root = workspace_root()?;
    let expected_license_marker = html_spdx_marker(&project_license(&root)?);
    let manifest = read_workspace_text(&root, constants::QUALITY_MANIFEST_FILE)?;
    let mut registered = BTreeSet::new();
    for entry in manifest.split("[[document]]").skip(1) {
        let path = configuration_value(entry, "path")
            .ok_or_else(|| "quality document has no path".to_owned())?;
        let kind = configuration_value(entry, "kind")
            .ok_or_else(|| format!("quality document {path} has no kind"))?;
        let owner = configuration_value(entry, "owner")
            .ok_or_else(|| format!("quality document {path} has no owner"))?;
        let status = configuration_value(entry, "status")
            .ok_or_else(|| format!("quality document {path} has no status"))?;
        let expected_prefix = match kind.as_str() {
            "policy" => "quality/policy/",
            "review" => "quality/reviews/",
            "release-evidence" => "quality/evidence/",
            _ => return Err(format!("quality document {path} has unknown kind {kind:?}")),
        };
        if owner.is_empty() || status.is_empty() || !path.starts_with(expected_prefix) {
            return Err(format!(
                "quality document {path} has incomplete or inconsistent metadata"
            ));
        }
        if !registered.insert(path.clone()) {
            return Err(format!("quality document is registered twice: {path}"));
        }
        let content = read_workspace_text(&root, &path)?;
        if !content.starts_with(&expected_license_marker) {
            return Err(format!("quality document lacks its license marker: {path}"));
        }
    }

    let mut files = Vec::new();
    collect_regular_files(&root.join("quality"), &mut files)?;
    let actual = files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .filter(|path| path.file_name().and_then(|value| value.to_str()) != Some("README.md"))
        .filter(|path| path.as_path() != root.join(constants::QUALITY_STATUS_FILE))
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<BTreeSet<_>>();
    if actual != registered {
        let unregistered = actual.difference(&registered).collect::<Vec<_>>();
        let missing = registered.difference(&actual).collect::<Vec<_>>();
        return Err(format!(
            "quality inventory differs from durable documents; unregistered: {unregistered:?}; missing: {missing:?}"
        ));
    }
    crate::output::pass("quality document inventory");
    Ok(())
}

/// Verifies directory ownership, durable test levels, fuzz ownership, and hygiene.
pub(crate) fn check_repository_structure() -> Result<(), String> {
    let root = workspace_root()?;
    let expected_license_marker = html_spdx_marker(&project_license(&root)?);
    let mut configured = BTreeSet::new();
    for directory in repository_directories(&root)? {
        let path = directory.path;
        let readme = directory.readme;
        if !is_safe_relative_path(Path::new(&path))
            || Path::new(&path).components().count() != 1
            || !is_safe_relative_path(Path::new(&readme))
            || !readme.starts_with(&format!("{path}/"))
            || directory.owner.is_empty()
            || directory.lifecycle.is_empty()
            || !root.join(&path).is_dir()
            || !root.join(&readme).is_file()
        {
            return Err(format!(
                "repository directory {path} has incomplete ownership"
            ));
        }
        let readme_content = read_workspace_text(&root, &readme)?;
        if !readme_content.starts_with(&expected_license_marker) {
            return Err(format!(
                "repository README lacks its license marker: {readme}"
            ));
        }
        if !configured.insert(path.clone()) {
            return Err(format!("repository directory is declared twice: {path}"));
        }
    }
    let tracked = Command::new(constants::GIT_COMMAND)
        .current_dir(&root)
        .args(["ls-files", "--cached", "-z"])
        .output()
        .map_err(|error| format!("could not enumerate tracked repository roots: {error}"))?;
    if !tracked.status.success() {
        return Err("could not enumerate tracked repository roots".to_owned());
    }
    let mut expected = BTreeSet::new();
    for file in tracked
        .stdout
        .split(|byte| *byte == 0)
        .filter(|file| !file.is_empty())
    {
        let name = std::str::from_utf8(file)
            .map_err(|error| format!("tracked repository path is not UTF-8: {error}"))?;
        if let Some((directory, _)) = name.split_once('/')
            && directory != constants::PORTABLE_DIRECTORY
        {
            expected.insert(directory.to_owned());
        }
    }
    if configured != expected {
        return Err(format!(
            "repository layout differs from required tracked roots: expected {expected:?}, found {configured:?}"
        ));
    }
    for manifest in workspace_package_manifests(&root)? {
        let readme = manifest.parent().unwrap_or(&root).join("README.md");
        if !readme.is_file() {
            return Err(format!(
                "workspace package has no responsibility README: {}",
                manifest.display()
            ));
        }
    }
    for readme in ["scripts/linux/README.md", "scripts/win/README.md"] {
        if !root.join(readme).is_file() {
            return Err(format!(
                "script directory has no responsibility README: {readme}"
            ));
        }
    }
    verify_test_level_inventory(&root)?;
    let policy = crate::config::quality_settings::QualityPolicy::parse(&read_workspace_text(
        &root,
        constants::QUALITY_GATES_FILE,
    )?)?;
    verify_fuzz_ownership(&root, &policy.fuzz)?;
    verify_generated_file_hygiene(&root)?;
    verify_ignore_policy(&root)?;
    crate::output::pass("repository ownership and hygiene");
    Ok(())
}

/// Verifies generated-state ignores without allowing source-of-truth inputs.
pub(crate) fn verify_ignore_policy(root: &Path) -> Result<(), String> {
    let ignore = read_workspace_text(root, ".gitignore")?;
    for required in [
        "target",
        "**/mutants.out*/",
        "fuzz/artifacts/",
        "fuzz/corpus/*",
        "!fuzz/corpus/README.md",
        "fuzz/coverage/",
        "/quality/evidence/*/gates/",
        "massif.out.*",
        "callgrind.out.*",
        "perf.data*",
        "*.profraw",
        "/dist/",
        "/release/",
        ".idea/",
        ".vscode/",
        "*.swp",
    ] {
        if !ignore.lines().any(|line| line.trim() == required) {
            return Err(format!("generated-state ignore is missing {required}"));
        }
    }
    let results_ignore = format!(
        "{}/",
        crate::configuration::automation_at(root)?
            .output
            .results_root
    );
    if !ignore.lines().any(|line| line.trim() == results_ignore) {
        return Err(format!(
            "generated-state ignore is missing {results_ignore}"
        ));
    }
    let bundle = ReleasedBundle::load(root)?;
    let freeze = bundle.member(crate::constants::BUNDLE_CONTRACT_FREEZE_MEMBER);
    let manifest = bundle.member("conformance/manifest.toml");
    for required in [
        crate::constants::CARGO_LOCK_FILE,
        "rust-toolchain.toml",
        constants::AUTOMATION_CONFIG_FILE,
        constants::CONFORMANCE_CONFIG_FILE,
        "config/release.toml",
        &freeze,
        &manifest,
        "scripts/linux/bootstrap.sh",
    ] {
        let output = Command::new(constants::GIT_COMMAND)
            .current_dir(root)
            .args(["ls-files", "--error-unmatch", required])
            .output()
            .map_err(|error| format!("could not inspect tracked input {required}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "required source-of-truth input is not tracked: {required}"
            ));
        }
    }
    Ok(())
}

/// Verifies every durable test purpose has one documented stable command.
pub(crate) fn verify_test_level_inventory(root: &Path) -> Result<(), String> {
    let inventory: crate::configuration_models::TestInventory =
        crate::configuration::read(root, constants::TEST_LEVELS_FILE)?;
    crate::configuration::require_schema(inventory.schema_version, "test inventory")?;
    let mut required = BTreeSet::from([
        "unit",
        "smoke",
        "integration",
        "system",
        "conformance",
        "property",
        "security",
        "fuzz-regression",
        "performance",
        "all",
    ]);
    for entry in inventory.level {
        if !required.remove(entry.name.as_str()) || entry.owner.trim().is_empty() {
            return Err(format!(
                "unknown, duplicate, or unowned test purpose: {}",
                entry.name
            ));
        }
        let expected = match entry.name.as_str() {
            "fuzz-regression" => "fuzz smoke".to_owned(),
            "performance" => "test performance --profile pr".to_owned(),
            name => format!("test {name}"),
        };
        let words = entry.command.split_whitespace().collect::<Vec<_>>();
        if !words.starts_with(&["cargo", "xtask"]) {
            return Err(format!("invalid test command for {}", entry.name));
        }
        let arguments = words[2..]
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>();
        let expected = expected
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if crate::interface::parse(&arguments)? != crate::interface::parse(&expected)? {
            return Err(format!(
                "test command does not match purpose {}",
                entry.name
            ));
        }
    }
    if !required.is_empty() {
        return Err(format!("test inventory is missing purposes: {required:?}"));
    }
    Ok(())
}

/// Verifies configured fuzz targets have harnesses, owners, and state policy.
fn verify_fuzz_ownership(
    root: &Path,
    settings: &crate::config::quality_settings::FuzzSettings,
) -> Result<(), String> {
    for target in &settings.targets {
        if !root
            .join("fuzz/fuzz_targets")
            .join(format!("{target}.rs"))
            .is_file()
        {
            return Err(format!(
                "fuzz target {target} has no harness or subsystem owner"
            ));
        }
    }
    Ok(())
}

/// Rejects tracked generated products while retaining documented empty roots.
pub(crate) fn verify_generated_file_hygiene(root: &Path) -> Result<(), String> {
    git_hygiene::verify_no_ignored_tracked_files(root)?;
    let result_directory = crate::configuration::automation_at(root)?
        .output
        .results_root;
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "ls-files",
            "--",
            "target",
            "mutants.out",
            "mutants.out.old",
            "fuzz/artifacts",
            "fuzz/corpus",
            "portable/archive",
            "portable/archived",
        ])
        .arg(result_directory)
        .output()
        .map_err(|error| format!("could not inspect tracked generated files: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not inspect tracked generated files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let tracked = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git emitted non-UTF-8 paths: {error}"))?
        .lines()
        .filter(|path| *path != "fuzz/corpus/README.md")
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if tracked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "generated or archived products are tracked: {}",
            tracked.join(", ")
        ))
    }
}
