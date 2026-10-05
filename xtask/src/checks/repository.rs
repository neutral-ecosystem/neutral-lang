// SPDX-License-Identifier: Apache-2.0

//! checks / repository responsibilities for repository automation.

use crate::{
    BTreeSet, Command, Path, ReleasedBundle, automation_value, collect_regular_files,
    configuration, configuration_value, constants, fs, git_hygiene, html_spdx_marker, interface,
    is_safe_relative_path, markdown_link_targets, project_license, quality_array,
    quality_output_path, quality_value, read_workspace_text, repository_directories,
    workspace_package_manifests, workspace_root,
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
            let candidate = file.parent().unwrap_or(&root).join(&target);
            let valid = fs::canonicalize(&candidate)
                .is_ok_and(|resolved| resolved.starts_with(&canonical_root));
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
    let retained_workflows = format!("{}/workflows", automation_value("output", "results_root")?);
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
    if !release.contains("cargo xtask release prepare") {
        return Err("release.yml does not delegate to `cargo xtask release prepare`".to_owned());
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
    let result_directory = automation_value("output", "results_root")?;
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
    verify_coverage_policy()?;
    verify_fuzz_ownership(&root)?;
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
    let results_ignore = format!("{}/", automation_value("output", "results_root")?);
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

/// Verifies coverage environment, outputs, thresholds, and exclusion policy.
pub(crate) fn verify_coverage_policy() -> Result<(), String> {
    let toolchain = quality_value("coverage", "toolchain")?;
    if toolchain != "nightly-only" && toolchain != "stable" {
        return Err(format!(
            "unsupported coverage toolchain policy: {toolchain}"
        ));
    }
    let html = quality_output_path("coverage", "html_output")?;
    if !html.ends_with("html/index.html") {
        return Err("coverage HTML output must end in html/index.html".to_owned());
    }
    let json = quality_output_path("coverage", "json_output")?;
    if json.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
        return Err("coverage JSON output must have a .json extension".to_owned());
    }
    for minimum_key in [
        "minimum_line_percent",
        "minimum_function_percent",
        "minimum_region_percent",
    ] {
        let minimum = quality_value("coverage", minimum_key)?
            .parse::<f64>()
            .map_err(|error| format!("invalid coverage {minimum_key}: {error}"))?;
        if !(minimum > 0.0 && minimum <= 100.0) {
            return Err(format!("coverage {minimum_key} must be within (0, 100]"));
        }
    }
    if quality_value("coverage", "exclusion_regex")?.is_empty() {
        return Err("coverage exclusions must be explicit".to_owned());
    }
    let exclusions = quality_value("coverage", "exclusion_policy")?;
    if !exclusions.starts_with("reviewed:") {
        return Err("coverage exclusions require an explicit reviewed rationale".to_owned());
    }
    Ok(())
}

/// Verifies every durable test purpose has one documented stable command.
pub(crate) fn verify_test_level_inventory(root: &Path) -> Result<(), String> {
    let inventory = read_workspace_text(root, constants::TEST_LEVELS_FILE)?;
    for (name, command) in [
        ("unit", "cargo xtask test unit"),
        ("smoke", "cargo xtask test smoke"),
        ("integration", "cargo xtask test integration"),
        ("system", "cargo xtask test system"),
        ("conformance", "cargo xtask test conformance"),
        ("property", "cargo xtask test property"),
        ("security", "cargo xtask test security"),
        ("fuzz-regression", "cargo xtask fuzz smoke"),
        ("performance", "cargo xtask test performance --profile pr"),
        ("all", "cargo xtask test all"),
    ] {
        if !inventory.contains(&format!("name = \"{name}\""))
            || !inventory.contains(&format!("command = \"{command}\""))
        {
            return Err(format!("test-level inventory is missing {name}: {command}"));
        }
    }
    Ok(())
}

/// Verifies configured fuzz targets have harnesses, owners, and state policy.
pub(crate) fn verify_fuzz_ownership(root: &Path) -> Result<(), String> {
    let quality = read_workspace_text(root, constants::QUALITY_GATES_FILE)?;
    for target in quality_array("fuzz", "targets")? {
        if !root
            .join("fuzz/fuzz_targets")
            .join(format!("{target}.rs"))
            .is_file()
            || !quality.contains(&format!("{target}_owner = \""))
        {
            return Err(format!(
                "fuzz target {target} has no harness or subsystem owner"
            ));
        }
    }
    for required in ["finding_policy = \"", "mutable_state = \""] {
        if !quality.contains(required) {
            return Err(format!("fuzz policy is missing {required}"));
        }
    }
    Ok(())
}

/// Rejects tracked generated products while retaining documented empty roots.
pub(crate) fn verify_generated_file_hygiene(root: &Path) -> Result<(), String> {
    git_hygiene::verify_no_ignored_tracked_files(root)?;
    let result_directory = automation_value("output", "results_root")?;
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
