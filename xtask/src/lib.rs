// SPDX-License-Identifier: Apache-2.0

//! Dependency-boundary checks for the Neutral workspace.
//!
//! This automation-only crate checks the resolved Cargo graph rather than
//! trusting package documentation. It is intentionally outside production
//! dependency graphs and does not implement Neutral language behavior.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, PathBuf},
    process::Command,
};

pub mod constants;

/// Runs an `xtask` subcommand.
///
/// # Errors
///
/// Returns an error when the command is invalid or the resolved dependency
/// graph violates the boundary policy.
pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();

    match arguments.as_slice() {
        [command] if command == "bootstrap" => bootstrap(),
        [command, action] if command == "environment" && action == "verify" => verify_environment(),
        [command, action] if command == "environment" && action == "manifest" => {
            print_environment_manifest()
        }
        [command, action] if command == "boundary" && action == "check" => check_boundaries(),
        [command] if command == "format" => run_cargo(&["fmt", "--all", "--", "--check"]),
        [command, action] if command == "format" && action == "--write" => {
            run_cargo(&["fmt", "--all"])
        }
        [command] if command == "lint" => run_cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]),
        [command, flag, profile] if command == "build" && flag == "--profile" => build(profile),
        [command, suite] if command == "test" => test_suite(suite),
        [command, suite, flag, profile]
            if command == "test" && suite == "performance" && flag == "--profile" =>
        {
            not_active(&format!("performance profile {profile}"))
        }
        [command, mode] if command == "fuzz" => not_active(&format!("fuzz mode {mode}")),
        [command] if command == "coverage" || command == "mutate" => not_active(command),
        [command, action] if command == "golden" || command == "quality" => {
            not_active(&format!("{command} {action}"))
        }
        [command, profile] if command == "ci" => ci(profile),
        [command] if command == "clean-results" => clean_results(),
        [command, action] if command == "boundary" && action == "help" => {
            println!("{} usage: cargo xtask boundary check", constants::INFO);
            Ok(())
        }
        _ => Err(
            "unsupported command; see portable/development/ENVIRONMENT-AUTOMATION.md".to_owned(),
        ),
    }
}

/// Creates ignored automation-result directories and records the local environment.
fn bootstrap() -> Result<(), String> {
    verify_environment()?;
    let result_directory = result_root()?.join("bootstrap");
    fs::create_dir_all(&result_directory)
        .map_err(|error| format!("could not create {}: {error}", result_directory.display()))?;
    fs::write(
        result_directory.join("environment.json"),
        environment_manifest()?,
    )
    .map_err(|error| format!("could not write bootstrap environment manifest: {error}"))?;
    println!("{} workspace bootstrap: pass", constants::INFO);
    Ok(())
}

/// Verifies the files and pinned Rust toolchain required by the Stage 1 workspace.
fn verify_environment() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    for required_path in [
        "Cargo.lock",
        "rust-toolchain.toml",
        "config/development-stage.toml",
        "config/host-policy.toml",
        "config/test-suites.toml",
        "conformance/manifest.toml",
    ] {
        if !workspace_root.join(required_path).is_file() {
            return Err(format!("missing required workspace file: {required_path}"));
        }
    }

    let rustc_version = command_output(constants::RUSTC_COMMAND, &["--version"])?;
    if !rustc_version.starts_with("rustc 1.97.1 ") {
        return Err(format!(
            "pinned Rust 1.97.1 is required; found {rustc_version}"
        ));
    }

    println!("{} environment verification: pass", constants::INFO);
    Ok(())
}

/// Prints the machine-readable environment manifest without writing tracked files.
fn print_environment_manifest() -> Result<(), String> {
    println!("{} {}", constants::MANIFEST, environment_manifest()?);
    Ok(())
}

/// Builds the machine-readable environment manifest used in generated evidence.
fn environment_manifest() -> Result<String, String> {
    let workspace_root = workspace_root()?;
    let rustc_version = command_output(constants::RUSTC_COMMAND, &["--version"])?;
    let cargo_version = command_output(constants::CARGO_COMMAND, &["--version"])?;
    let active_stage = active_stage()?;
    Ok(format!(
        concat!(
            "{{\n",
            "  \"workspace_root\": \"{}\",\n",
            "  \"rustc\": \"{}\",\n",
            "  \"cargo\": \"{}\",\n",
            "  \"active_stage\": {}\n",
            "}}"
        ),
        json_string(&workspace_root.display().to_string()),
        json_string(&rustc_version),
        json_string(&cargo_version),
        active_stage,
    ))
}

/// Reads the active implementation stage from the repository configuration.
fn active_stage() -> Result<u8, String> {
    let configuration_path = workspace_root()?.join("config/development-stage.toml");
    let configuration = fs::read_to_string(&configuration_path)
        .map_err(|error| format!("could not read {}: {error}", configuration_path.display()))?;
    let value = configuration
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("active_stage ="))
        .ok_or_else(|| "development-stage configuration has no active_stage".to_owned())?;
    value
        .trim()
        .parse::<u8>()
        .map_err(|error| format!("invalid active_stage value: {error}"))
}

/// Escapes a string for the limited JSON values emitted by automation evidence.
fn json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Runs the requested Cargo build profile without introducing source behavior.
fn build(profile: &str) -> Result<(), String> {
    match profile {
        "dev" => run_cargo(&["build", "--workspace"]),
        "test" => run_cargo(&["test", "--workspace", "--all-targets", "--no-run"]),
        "release" => run_cargo(&["build", "--workspace", "--release"]),
        _ => Err(format!("unknown build profile: {profile}")),
    }
}

/// Runs an active Stage 1 suite or rejects a future-stage suite selection.
fn test_suite(suite: &str) -> Result<(), String> {
    match suite {
        "all" | "unit" => {
            run_cargo(&["test", "--workspace", "--all-targets"])?;
            verify_active_test_counts()
        }
        "smoke" => run_shell_smoke(),
        "integration" | "system" | "conformance" | "property" | "security" => not_active(suite),
        _ => Err(format!("unknown or empty test suite: {suite}")),
    }
}

/// Runs the behavior-free command-shell checks active during Stage 1.
fn run_shell_smoke() -> Result<(), String> {
    run_cargo(&[
        "run",
        "--quiet",
        "--package",
        constants::NEUTRAL_CLI,
        "--",
        "--help",
    ])?;
    run_cargo(&[
        "run",
        "--quiet",
        "--package",
        constants::NEUTRAL_PROBE,
        "--",
        "--help",
    ])
}

/// Verifies that every active Stage 1 test category has its configured minimum.
fn verify_active_test_counts() -> Result<(), String> {
    let test_list = command_output(
        constants::CARGO_COMMAND,
        &["test", "--workspace", "--all-targets", "--", "--list"],
    )?;
    let minimums = stage1_test_minimums()?;
    let discovered = minimums
        .keys()
        .map(|category| {
            (
                category.clone(),
                test_list.matches(&format!("{category}_")).count(),
            )
        })
        .collect();
    validate_test_minimums(&minimums, &discovered)
}

/// Rejects an active test category whose discovered count is below its minimum.
fn validate_test_minimums(
    minimums: &BTreeMap<String, usize>,
    discovered: &BTreeMap<String, usize>,
) -> Result<(), String> {
    for (category, minimum) in minimums {
        let discovered = discovered.get(category).copied().unwrap_or_default();
        if discovered < *minimum {
            return Err(format!(
                "active Stage 1 suite {category} requires at least {minimum} tests; discovered {discovered}"
            ));
        }
    }
    Ok(())
}

/// Reads the simple Stage 1 test-minimum configuration owned by the workspace.
fn stage1_test_minimums() -> Result<BTreeMap<String, usize>, String> {
    let configuration_path = workspace_root()?.join("config/test-suites.toml");
    let configuration = fs::read_to_string(&configuration_path)
        .map_err(|error| format!("could not read {}: {error}", configuration_path.display()))?;
    let mut in_stage1_section = false;
    let mut minimums = BTreeMap::new();

    for line in configuration.lines().map(str::trim) {
        if line.starts_with('[') {
            in_stage1_section = line == "[stage1.minimum]";
            continue;
        }
        if !in_stage1_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!("invalid Stage 1 test-minimum entry: {line}"));
        };
        let minimum = value
            .trim()
            .parse::<usize>()
            .map_err(|error| format!("invalid test minimum for {}: {error}", name.trim()))?;
        minimums.insert(name.trim().to_owned(), minimum);
    }

    if minimums.is_empty() {
        Err("Stage 1 test-minimum configuration is empty".to_owned())
    } else {
        Ok(minimums)
    }
}

/// Runs the currently active checks for each declared CI profile.
fn ci(profile: &str) -> Result<(), String> {
    match profile {
        "stage1" | "pr" | "nightly" | "release" => run_stage1_ci(profile),
        _ => Err(format!("unknown CI profile: {profile}")),
    }
}

/// Runs the Stage 1 gate and writes a generated summary beneath the result root.
fn run_stage1_ci(profile: &str) -> Result<(), String> {
    verify_environment()?;
    run_cargo(&["metadata", "--format-version", "1", "--no-deps"])?;
    check_boundaries()?;
    run_cargo(&["fmt", "--all", "--", "--check"])?;
    run_cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])?;
    run_cargo(&["check", "--workspace", "--all-targets", "--locked"])?;
    test_suite("all")?;
    run_shell_smoke()?;
    run_cargo(&["build", "--locked", "--package", constants::NEUTRAL_PROBE])?;
    run_cargo(&["doc", "--workspace", "--no-deps"])?;

    let result_directory = unique_result_directory(profile)?;
    fs::write(
        result_directory.join("task-summary.json"),
        format!(
            "{{\"profile\":\"{}\",\"status\":\"pass\"}}\n",
            json_string(profile)
        ),
    )
    .map_err(|error| format!("could not write task summary: {error}"))?;
    println!("{} CI {profile}: pass", constants::INFO);
    Ok(())
}

/// Rejects a command whose evidence is intentionally inactive in Stage 1.
fn not_active(command: &str) -> Result<(), String> {
    Err(format!("{command} is not active during Stage 1"))
}

/// Runs Cargo with inherited standard streams and converts failures to task errors.
fn run_cargo(arguments: &[&str]) -> Result<(), String> {
    let status = Command::new(constants::CARGO_COMMAND)
        .current_dir(workspace_root()?)
        .args(arguments)
        .status()
        .map_err(|error| {
            format!(
                "could not run {} {}: {error}",
                constants::CARGO_COMMAND,
                arguments.join(" ")
            )
        })?;
    status.success().then_some(()).ok_or_else(|| {
        format!(
            "{} {} failed with {status}",
            constants::CARGO_COMMAND,
            arguments.join(" ")
        )
    })
}

/// Returns trimmed UTF-8 output from a successful command.
fn command_output(command: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run {command}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command} failed with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("{command} emitted non-UTF-8 output: {error}"))
}

/// Creates a unique generated-evidence directory beneath the relevant CI profile.
fn unique_result_directory(profile: &str) -> Result<PathBuf, String> {
    let profile_root = result_root()?.join("ci").join(profile);
    fs::create_dir_all(&profile_root).map_err(|error| {
        format!(
            "could not create CI evidence directory {}: {error}",
            profile_root.display()
        )
    })?;
    for suffix in 0_u16..1000 {
        let directory = profile_root.join(format!("run-{}-{suffix}", std::process::id()));
        match fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("could not create {}: {error}", directory.display())),
        }
    }
    Err("could not allocate a unique result directory".to_owned())
}

/// Returns the configured result root after rejecting unsafe paths.
fn result_root() -> Result<PathBuf, String> {
    let configured = env::var("NEUTRAL_TEST_RESULTS").unwrap_or_else(|_| "test-results".to_owned());
    let path = PathBuf::from(&configured);
    if !is_safe_result_path(&path) {
        return Err(
            "NEUTRAL_TEST_RESULTS must be a relative path beneath the workspace".to_owned(),
        );
    }
    Ok(workspace_root()?.join(path))
}

/// Returns whether a configured result path cannot name the workspace or escape it.
fn is_safe_result_path(path: &std::path::Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Removes only the configured generated-result root after validating its path.
fn clean_results() -> Result<(), String> {
    let root = result_root()?;
    if root.exists() {
        fs::remove_dir_all(&root)
            .map_err(|error| format!("could not remove {}: {error}", root.display()))?;
    }
    println!("{} generated results cleaned", constants::INFO);
    Ok(())
}

/// Checks all declared package and effect boundaries against Cargo's graph.
fn check_boundaries() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    let policy = direct_dependency_policy();

    for (package, allowed_dependencies) in &policy {
        let dependencies = direct_dependencies(&workspace_root, package)?;
        validate_direct_dependencies(package, &dependencies, allowed_dependencies)?;
    }

    let compiler_closure = package_names(&tree_output(
        &workspace_root,
        constants::NEUTRAL_COMPILER,
        "normal",
        None,
    )?);
    validate_allowed_packages(
        &format!("{} pure compilation closure", constants::NEUTRAL_COMPILER),
        &compiler_closure,
        &set([
            constants::NEUTRAL_COMPILER,
            constants::NEUTRAL_CORE,
            constants::NEUTRAL_IR,
            constants::NEUTRAL_VOCABULARY,
            "block-buffer",
            "cfg-if",
            "cpufeatures",
            "crypto-common",
            "digest",
            "generic-array",
            "sha2",
            "typenum",
            "version_check",
        ]),
    )?;

    let probe_closure = package_names(&tree_output(
        &workspace_root,
        constants::NEUTRAL_PROBE,
        "all",
        None,
    )?);
    validate_allowed_packages(
        "neutral-probe dependency tree",
        &probe_closure,
        &set([
            constants::NEUTRAL_PROBE,
            constants::NEUTRAL_CORE,
            constants::NEUTRAL_IR,
            constants::NEUTRAL_READER,
            constants::NEUTRAL_VOCABULARY,
            "block-buffer",
            "cfg-if",
            "cpufeatures",
            "crypto-common",
            "digest",
            "generic-array",
            "sha2",
            "typenum",
            "version_check",
        ]),
    )?;

    println!("{} dependency boundaries: pass", constants::INFO);
    Ok(())
}

/// Returns the workspace root derived from the `xtask` package location.
fn workspace_root() -> Result<PathBuf, String> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "xtask manifest has no workspace-root parent".to_owned())
}

/// Returns the exact normal-dependency policy for every workspace package.
fn direct_dependency_policy() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        (constants::NEUTRAL_BENCH, set([])),
        (
            constants::NEUTRAL_CLI,
            set([
                constants::NEUTRAL_COMPILER,
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_READER,
            ]),
        ),
        (
            constants::NEUTRAL_COMPILER,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_CORE, set(["sha2"])),
        (constants::NEUTRAL_IR, set([constants::NEUTRAL_CORE])),
        (
            constants::NEUTRAL_PROBE,
            set([constants::NEUTRAL_CORE, constants::NEUTRAL_READER]),
        ),
        (
            constants::NEUTRAL_READER,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_TEST_SUITE, set([])),
        (constants::NEUTRAL_TEST_SUPPORT, set([])),
        (
            constants::NEUTRAL_VOCABULARY,
            set([constants::NEUTRAL_CORE, constants::NEUTRAL_IR]),
        ),
        (constants::XTASK, set([])),
    ])
}

/// Resolves the direct normal dependencies of one workspace package.
fn direct_dependencies(
    workspace_root: &PathBuf,
    package: &str,
) -> Result<BTreeSet<String>, String> {
    let output = tree_output(workspace_root, package, "normal", Some(1))?;
    let mut packages = package_names(&output);
    packages.remove(package);
    Ok(packages)
}

/// Runs `cargo tree` and returns its UTF-8 output for one package.
fn tree_output(
    workspace_root: &PathBuf,
    package: &str,
    edges: &str,
    depth: Option<u8>,
) -> Result<String, String> {
    let mut command = Command::new(constants::CARGO_COMMAND);
    command.current_dir(workspace_root).args([
        "tree",
        "--locked",
        "--package",
        package,
        "--edges",
        edges,
        "--prefix",
        "none",
    ]);

    if let Some(depth) = depth {
        command.args(["--depth", &depth.to_string()]);
    }

    let output = command
        .output()
        .map_err(|error| format!("could not run cargo tree for {package}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree failed for {package}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo tree emitted non-UTF-8 output for {package}: {error}"))
}

/// Extracts package names from Cargo's no-prefix tree output.
fn package_names(tree: &str) -> BTreeSet<String> {
    tree.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version_or_kind = words.next()?;
            version_or_kind.starts_with('v').then(|| name.to_owned())
        })
        .collect()
}

/// Rejects direct dependencies that differ from the package's exact policy.
fn validate_direct_dependencies(
    package: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let allowed = allowed.iter().map(|name| (*name).to_owned()).collect();
    validate_exact_packages(&format!("{package} direct dependencies"), actual, &allowed)
}

/// Compares an observed package set with an exact expected package set.
fn validate_exact_packages(
    boundary: &str,
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }

    let unexpected = actual.difference(expected).collect::<Vec<_>>();
    let missing = expected.difference(actual).collect::<Vec<_>>();
    Err(format!(
        "{boundary} violates the policy; unexpected: {unexpected:?}; missing: {missing:?}"
    ))
}

/// Rejects packages outside an allowlist for a named dependency boundary.
fn validate_allowed_packages(
    boundary: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let unexpected = actual
        .iter()
        .filter(|package| !allowed.contains(package.as_str()))
        .collect::<Vec<_>>();

    if unexpected.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{boundary} contains forbidden packages: {unexpected:?}"
        ))
    }
}

/// Builds a sorted package-name set from an array literal.
fn set<const N: usize>(values: [&'static str; N]) -> BTreeSet<&'static str> {
    values.into_iter().collect()
}

#[cfg(test)]
/// Tests for dependency-boundary policy failures.
mod tests {
    use super::{constants, set, validate_allowed_packages, validate_direct_dependencies};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    /// Verifies that invalid automation command syntax is rejected.
    fn automation_rejects_an_invalid_command() {
        assert!(super::run(["unknown".to_owned()]).is_err());
    }

    #[test]
    /// Verifies that the environment manifest identifies the pinned toolchain.
    fn environment_manifest_identifies_the_pinned_toolchain() {
        let manifest =
            super::environment_manifest().expect("environment manifest should be available");
        assert!(manifest.contains("rustc 1.97.1"));
        assert!(manifest.contains("\"active_stage\": 2"));
    }

    #[test]
    /// Verifies that a compiler-to-CLI edge violates the workspace policy.
    fn workspace_rejects_a_forbidden_direct_compiler_dependency() {
        let actual = BTreeSet::from([
            constants::NEUTRAL_CLI.to_owned(),
            constants::NEUTRAL_CORE.to_owned(),
            constants::NEUTRAL_IR.to_owned(),
            constants::NEUTRAL_VOCABULARY.to_owned(),
        ]);

        assert!(
            validate_direct_dependencies(
                constants::NEUTRAL_COMPILER,
                &actual,
                &set([
                    constants::NEUTRAL_CORE,
                    constants::NEUTRAL_IR,
                    constants::NEUTRAL_VOCABULARY
                ]),
            )
            .is_err()
        );
    }

    #[test]
    /// Verifies that core may depend only on the reviewed SHA-256 value utility.
    fn core_allows_only_the_reviewed_sha256_dependency() {
        let actual = BTreeSet::from(["sha2".to_owned()]);
        assert!(
            validate_direct_dependencies(constants::NEUTRAL_CORE, &actual, &set(["sha2"])).is_ok()
        );
    }

    #[test]
    /// Verifies that a compiler package in the probe closure is rejected.
    fn probe_allowlist_rejects_a_compiler_dependency_in_the_probe_closure() {
        let actual = BTreeSet::from([
            constants::NEUTRAL_COMPILER.to_owned(),
            constants::NEUTRAL_CORE.to_owned(),
            constants::NEUTRAL_PROBE.to_owned(),
            constants::NEUTRAL_READER.to_owned(),
        ]);

        assert!(
            validate_allowed_packages(
                "neutral-probe dependency tree",
                &actual,
                &set([
                    constants::NEUTRAL_PROBE,
                    constants::NEUTRAL_CORE,
                    constants::NEUTRAL_IR,
                    constants::NEUTRAL_READER,
                    constants::NEUTRAL_VOCABULARY,
                ]),
            )
            .is_err()
        );
    }

    #[test]
    /// Verifies that automation cleanup cannot target a parent or workspace path.
    fn automation_rejects_unsafe_result_paths() {
        assert!(super::is_safe_result_path(std::path::Path::new(
            "test-results"
        )));
        assert!(!super::is_safe_result_path(std::path::Path::new(".")));
        assert!(!super::is_safe_result_path(std::path::Path::new(
            "../test-results"
        )));
    }

    #[test]
    /// Verifies that active-suite discovery fails when a required category is empty.
    fn automation_rejects_a_zero_active_suite() {
        let minimums = BTreeMap::from([("automation".to_owned(), 1)]);
        let discovered = BTreeMap::from([("automation".to_owned(), 0)]);

        assert!(super::validate_test_minimums(&minimums, &discovered).is_err());
    }
}
