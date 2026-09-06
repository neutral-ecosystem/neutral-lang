// SPDX-License-Identifier: Apache-2.0

//! Dependency-boundary checks for the Neutral workspace.
//!
//! This automation-only crate checks the resolved Cargo graph rather than
//! trusting package documentation. It is intentionally outside production
//! dependency graphs and does not implement Neutral language behavior.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

pub mod constants;

/// Source template for the generated workspace rustdoc landing page.
const RUSTDOC_INDEX_TEMPLATE: &str = include_str!("rustdoc-index.html");
/// Shared HTML fragment injected into every generated rustdoc page.
const RUSTDOC_HEADER_TEMPLATE: &str = include_str!("rustdoc-header.html");

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
        [command, action] if command == "test-layout" && action == "check" => check_test_layout(),
        [command, action] if command == "traceability" && action == "check" => check_traceability(),
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
        [command] if command == "docs" => documentation(),
        [command, flag, profile] if command == "build" && flag == "--profile" => build(profile),
        [command, suite] if command == "test" => test_suite(suite),
        [command, suite, flag, profile]
            if command == "test" && suite == "performance" && flag == "--profile" =>
        {
            performance(profile)
        }
        [command, mode] if command == "fuzz" => fuzz(mode),
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
            "unsupported command; see portable/development/00-ENVIRONMENT-AUTOMATION.md".to_owned(),
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

/// Verifies the files and selected Rust toolchain channel required by the workspace.
fn verify_environment() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    for required_path in [
        "Cargo.lock",
        "rust-toolchain.toml",
        "config/development-stage.toml",
        "config/host-policy.toml",
        "config/ir-encoding.toml",
        "config/quality-gates.toml",
        "config/test-suites.toml",
        "portable/conformance/manifest.toml",
    ] {
        if !workspace_root.join(required_path).is_file() {
            return Err(format!("missing required workspace file: {required_path}"));
        }
    }

    let rustc_version = command_output(constants::RUSTC_COMMAND, &["--version"])?;
    let rust_channel = rust_channel()?;
    if !rust_version_matches_channel(&rustc_version, &rust_channel) {
        return Err(format!(
            "Rust channel {rust_channel} is required; found {rustc_version}"
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
    let rust_channel = rust_channel()?;
    let active_stage = active_stage()?;
    Ok(format!(
        concat!(
            "{{\n",
            "  \"workspace_root\": \"{}\",\n",
            "  \"rust_channel\": \"{}\",\n",
            "  \"rustc\": \"{}\",\n",
            "  \"cargo\": \"{}\",\n",
            "  \"active_stage\": {}\n",
            "}}"
        ),
        json_string(&workspace_root.display().to_string()),
        json_string(&rust_channel),
        json_string(&rustc_version),
        json_string(&cargo_version),
        active_stage,
    ))
}

/// Reads the selected Rust channel from the repository toolchain manifest.
fn rust_channel() -> Result<String, String> {
    let path = workspace_root()?.join("rust-toolchain.toml");
    let manifest = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    manifest
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("channel = \"")?.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| "rust-toolchain.toml has no quoted channel".to_owned())
}

/// Returns whether one compiler version belongs to the selected toolchain channel.
fn rust_version_matches_channel(rustc_version: &str, channel: &str) -> bool {
    if channel == "stable" {
        rustc_version.starts_with("rustc ")
            && !["-nightly", "-beta", "-dev"]
                .iter()
                .any(|marker| rustc_version.contains(marker))
    } else {
        rustc_version.starts_with(&format!("rustc {channel} "))
    }
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

/// Builds every workspace crate's API documentation and its metadata-driven index.
fn documentation() -> Result<(), String> {
    run_rustdoc()?;
    let metadata = command_output(
        constants::CARGO_COMMAND,
        &["metadata", "--format-version", "1", "--no-deps"],
    )?;
    let index = render_rustdoc_index(&metadata)?;
    let output_directory = workspace_root()?.join(constants::RUSTDOC_OUTPUT_DIRECTORY);
    fs::create_dir_all(&output_directory).map_err(|error| {
        format!(
            "could not create rustdoc directory {}: {error}",
            output_directory.display()
        )
    })?;
    let output_path = output_directory.join(constants::RUSTDOC_INDEX_FILE);
    fs::write(&output_path, index)
        .map_err(|error| format!("could not write {}: {error}", output_path.display()))?;
    println!(
        "{} workspace documentation: {}",
        constants::INFO,
        output_path.display()
    );
    Ok(())
}

/// Runs workspace rustdoc with the shared navigation header on every HTML page.
fn run_rustdoc() -> Result<(), String> {
    let header_path = workspace_root()?.join(constants::RUSTDOC_HEADER_FILE);
    if !header_path.is_file() {
        return Err(format!(
            "rustdoc navigation header is missing: {}",
            header_path.display()
        ));
    }
    let header_path = header_path
        .to_str()
        .ok_or_else(|| "rustdoc navigation header path is not valid UTF-8".to_owned())?;
    let mut rustdoc_flags = env::var(constants::CARGO_ENCODED_RUSTDOCFLAGS).unwrap_or_default();
    if !rustdoc_flags.is_empty() {
        rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    }
    rustdoc_flags.push_str(constants::RUSTDOC_HTML_HEADER_FLAG);
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str(header_path);
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("--cfg");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str(&rustdoc_header_configuration());

    let arguments = ["doc", "--workspace", "--no-deps"];
    let status = Command::new(constants::CARGO_COMMAND)
        .current_dir(workspace_root()?)
        .env(constants::CARGO_ENCODED_RUSTDOCFLAGS, rustdoc_flags)
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

/// Derives a stable non-security cache token from the shared rustdoc header.
fn rustdoc_header_configuration() -> String {
    let mut hash = constants::RUSTDOC_HEADER_HASH_OFFSET;
    for byte in RUSTDOC_HEADER_TEMPLATE.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(constants::RUSTDOC_HEADER_HASH_PRIME);
    }
    format!("{}_{hash:016x}", constants::RUSTDOC_HEADER_CFG_PREFIX)
}

/// Embeds Cargo metadata safely into the workspace rustdoc index template.
fn render_rustdoc_index(metadata: &str) -> Result<String, String> {
    if !RUSTDOC_INDEX_TEMPLATE.contains(constants::CARGO_METADATA_PLACEHOLDER) {
        return Err("rustdoc index template has no Cargo metadata placeholder".to_owned());
    }
    let safe_metadata = metadata.replace('<', "\\u003c");
    Ok(RUSTDOC_INDEX_TEMPLATE.replacen(constants::CARGO_METADATA_PLACEHOLDER, &safe_metadata, 1))
}

/// Runs an active Stage 1 suite or rejects a future-stage suite selection.
fn test_suite(suite: &str) -> Result<(), String> {
    match suite {
        "all" | "unit" => {
            run_cargo(&["test", "--workspace", "--all-targets"])?;
            verify_test_counts(&active_test_profile()?)
        }
        "smoke" => run_shell_smoke(),
        "integration" | "system" | "conformance" | "property" | "security"
            if active_stage()? >= 2 =>
        {
            run_active_test_filter(suite)
        }
        "integration" | "system" | "conformance" | "property" | "security" => not_active(suite),
        _ => Err(format!("unknown or empty test suite: {suite}")),
    }
}

/// Runs one active cross-package suite by stable test-name prefix.
fn run_active_test_filter(suite: &str) -> Result<(), String> {
    run_cargo(&[
        "test",
        "--package",
        constants::NEUTRAL_TEST_SUITE,
        "--",
        &format!("{suite}_"),
    ])
}

/// Runs an active bounded fuzz selection or rejects future campaigns.
fn fuzz(mode: &str) -> Result<(), String> {
    match mode {
        "smoke" if active_stage()? >= 2 => run_active_test_filter("fuzz_smoke"),
        "campaign" if active_stage()? >= 9 => run_active_test_filter("fuzz"),
        "campaign" if active_stage()? >= 7 => run_active_test_filter("fuzz_decoder"),
        _ => not_active(&format!("fuzz mode {mode}")),
    }
}

/// Runs one controlled Stage 9 benchmark, stress, or soak profile.
fn performance(profile: &str) -> Result<(), String> {
    if active_stage()? < 9 {
        return not_active(&format!("performance profile {profile}"));
    }
    match profile {
        "pr" | "release" | "soak" => run_cargo(&[
            "bench",
            "--package",
            constants::NEUTRAL_BENCH,
            "--bench",
            constants::STAGE9_BENCHMARK,
            "--",
            profile,
        ]),
        _ => Err(format!("unknown performance profile: {profile}")),
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
fn verify_test_counts(profile: &str) -> Result<(), String> {
    let test_list = command_output(
        constants::CARGO_COMMAND,
        &["test", "--workspace", "--all-targets", "--", "--list"],
    )?;
    let minimums = test_minimums(profile)?;
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
                "active suite {category} requires at least {minimum} tests; discovered {discovered}"
            ));
        }
    }
    Ok(())
}

/// Returns the test-minimum profile matching the configured active stage.
fn active_test_profile() -> Result<String, String> {
    Ok(format!("stage{}", active_stage()?))
}

/// Reads the simple Stage 1 test-minimum configuration owned by the workspace.
fn test_minimums(profile: &str) -> Result<BTreeMap<String, usize>, String> {
    let configuration_path = workspace_root()?.join("config/test-suites.toml");
    let configuration = fs::read_to_string(&configuration_path)
        .map_err(|error| format!("could not read {}: {error}", configuration_path.display()))?;
    let section = format!("[{profile}.minimum]");
    let mut in_requested_section = false;
    let mut minimums = BTreeMap::new();

    for line in configuration.lines().map(str::trim) {
        if line.starts_with('[') {
            in_requested_section = line == section;
            continue;
        }
        if !in_requested_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!("invalid {profile} test-minimum entry: {line}"));
        };
        let minimum = value
            .trim()
            .parse::<usize>()
            .map_err(|error| format!("invalid test minimum for {}: {error}", name.trim()))?;
        minimums.insert(name.trim().to_owned(), minimum);
    }

    if minimums.is_empty() {
        Err(format!("{profile} test-minimum configuration is empty"))
    } else {
        Ok(minimums)
    }
}

/// Runs the currently active checks for each declared CI profile.
fn ci(profile: &str) -> Result<(), String> {
    match profile {
        "stage1" => run_ci_gate(profile, false),
        "pr" | "nightly" | "release" => run_ci_gate(profile, true),
        _ => Err(format!("unknown CI profile: {profile}")),
    }
}

/// Runs the Stage 1 gate and writes a generated summary beneath the result root.
fn run_ci_gate(profile: &str, include_behavior: bool) -> Result<(), String> {
    verify_environment()?;
    run_cargo(&["metadata", "--format-version", "1", "--no-deps"])?;
    check_boundaries()?;
    check_test_layout()?;
    check_traceability()?;
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
    run_cargo(&["test", "--workspace", "--all-targets"])?;
    let test_profile = if include_behavior {
        active_test_profile()?
    } else {
        "stage1".to_owned()
    };
    verify_test_counts(&test_profile)?;
    run_shell_smoke()?;
    if include_behavior {
        for suite in [
            "integration",
            "system",
            "conformance",
            "property",
            "security",
        ] {
            run_active_test_filter(suite)?;
        }
        fuzz("smoke")?;
    }
    run_cargo(&["build", "--locked", "--package", constants::NEUTRAL_PROBE])?;
    documentation()?;

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
    let stage = active_stage().unwrap_or_default();
    Err(format!(
        "{command} is not active during configured Stage {stage}"
    ))
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
            constants::NEUTRAL_ENCODING,
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

/// Checks accepted IDs, completed syntax items, and fixture/oracle inventories.
fn check_traceability() -> Result<(), String> {
    let root = workspace_root()?;
    let requirements = read_workspace_text(&root, constants::REQUIREMENTS_FILE)?;
    let syntax = read_workspace_text(&root, constants::SYNTAX_CONTRACT_FILE)?;
    let checklist = read_workspace_text(&root, constants::SYNTAX_CHECKLIST_FILE)?;
    let traceability = read_workspace_text(&root, constants::TRACEABILITY_FILE)?;
    let manifest = read_workspace_text(&root, constants::CONFORMANCE_MANIFEST_FILE)?;

    let requirement_ids = contract_ids(&requirements, "NL-");
    let syntax_ids = contract_ids(&syntax, "SYN-");
    let checklist_ids = contract_ids(&checklist, "SYN-");
    ensure_ids_covered("requirements", &requirement_ids, &traceability)?;
    ensure_ids_covered("syntax", &syntax_ids, &traceability)?;
    if syntax_ids != checklist_ids {
        return Err("master syntax and implementation checklist IDs differ".to_owned());
    }
    ensure_syntax_complete(constants::SYNTAX_CONTRACT_FILE, &syntax)?;
    ensure_syntax_complete(constants::SYNTAX_CHECKLIST_FILE, &checklist)?;
    ensure_inventory_registered(&root, constants::FIXTURE_DIRECTORY, &manifest)?;
    ensure_inventory_registered(&root, constants::ORACLE_DIRECTORY, &manifest)?;
    ensure_registered_paths_exist(&root, &manifest)?;

    println!("{} traceability coherence: pass", constants::INFO);
    Ok(())
}

/// Reads one required UTF-8 workspace file with a path-safe error.
fn read_workspace_text(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("could not read {relative}: {error}"))
}

/// Extracts unique contract identifiers beginning with `prefix`.
fn contract_ids(content: &str, prefix: &str) -> BTreeSet<String> {
    content
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        .filter(|token| token.starts_with(prefix))
        .map(str::to_owned)
        .collect()
}

/// Requires every accepted identifier to occur in the evidence index.
fn ensure_ids_covered(
    category: &str,
    identifiers: &BTreeSet<String>,
    traceability: &str,
) -> Result<(), String> {
    if identifiers.is_empty() {
        return Err(format!("{category} contains no contract identifiers"));
    }
    let missing = identifiers
        .iter()
        .filter(|identifier| !traceability.contains(identifier.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "traceability is missing {category} IDs: {}",
            missing.join(", ")
        ))
    }
}

/// Rejects an unchecked master syntax item after Stage 8 traceability closure.
fn ensure_syntax_complete(path: &str, content: &str) -> Result<(), String> {
    let unchecked = content
        .lines()
        .filter(|line| line.trim_start().starts_with("- [ ]") && line.contains("SYN-"))
        .collect::<Vec<_>>();
    if unchecked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{path} contains {} unchecked syntax items",
            unchecked.len()
        ))
    }
}

/// Requires every normative file beneath `directory` to occur in the manifest.
fn ensure_inventory_registered(root: &Path, directory: &str, manifest: &str) -> Result<(), String> {
    let mut files = Vec::new();
    collect_regular_files(&root.join(directory), &mut files)?;
    let missing = files
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| {
            matches!(
                Path::new(path).extension().and_then(|value| value.to_str()),
                Some("neu" | "json" | "toml")
            )
        })
        .filter(|path| !manifest.contains(path))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "conformance manifest omits normative files: {}",
            missing.join(", ")
        ))
    }
}

/// Recursively collects regular files without following directory symlinks.
fn collect_regular_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
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

/// Rejects inline Rust test bodies and requires path-based crate-local test modules.
fn check_test_layout() -> Result<(), String> {
    let root = workspace_root()?;
    let mut source_files = Vec::new();
    collect_regular_files(&root.join("crates"), &mut source_files)?;
    collect_regular_files(&root.join("xtask/src"), &mut source_files)?;
    let mut violations = Vec::new();
    for path in source_files {
        if path.extension().and_then(|value| value.to_str()) != Some("rs")
            || !path
                .components()
                .any(|component| component.as_os_str() == "src")
        {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if source_has_non_path_test_configuration(&content) {
            violations.push(
                path.strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    if violations.is_empty() {
        println!("{} crate-local test layout: pass", constants::INFO);
        Ok(())
    } else {
        Err(format!(
            "production source contains inline or non-path test modules: {}",
            violations.join(", ")
        ))
    }
}

/// Returns whether any test-only source declaration lacks an immediate path attribute.
fn source_has_non_path_test_configuration(content: &str) -> bool {
    let lines = content.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        line.trim() == constants::TEST_CONFIGURATION_MARKER
            && lines[index + 1..]
                .iter()
                .find(|candidate| !candidate.trim().is_empty())
                .is_none_or(|candidate| {
                    !candidate
                        .trim()
                        .starts_with(constants::TEST_PATH_ATTRIBUTE_MARKER)
                })
    })
}

/// Requires every workspace-relative path registered by the manifest to exist.
fn ensure_registered_paths_exist(root: &Path, manifest: &str) -> Result<(), String> {
    let missing = manifest
        .split('"')
        .filter(|value| value.starts_with("portable/"))
        .filter(|value| !root.join(value).is_file())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "manifest paths do not exist: {}",
            missing.join(", ")
        ))
    }
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
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
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
        (
            constants::NEUTRAL_ENCODING,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_IR, set([constants::NEUTRAL_CORE])),
        (
            constants::NEUTRAL_PROBE,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_READER,
            ]),
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
#[path = "../tests/unit/mod.rs"]
mod tests;
