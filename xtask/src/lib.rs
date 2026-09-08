// SPDX-License-Identifier: Apache-2.0

//! Dependency-boundary checks for the Neutral workspace.
//!
//! This automation-only crate checks the resolved Cargo graph rather than
//! trusting package documentation. It is intentionally outside production
//! dependency graphs and does not implement Neutral language behavior.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

pub mod constants;
mod interface;
mod release;

use interface::{
    BuildProfile, CiProfile, FuzzMode, PerformanceProfile, PortableAction, QualityProfile, Task,
    TestLevel, ValidationTarget, VersionAction,
};

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
    execute(interface::parse(&arguments)?)
}

/// Executes one parsed stable automation task.
fn execute(task: Task) -> Result<(), String> {
    match task {
        Task::Help => {
            for line in interface::HELP.lines() {
                println!("{} {line}", constants::INFO);
            }
            Ok(())
        }
        Task::Bootstrap => bootstrap(),
        Task::EnvironmentVerify => verify_complete_environment(),
        Task::EnvironmentManifest => print_environment_manifest(),
        Task::Format { write } => format_workspace(write),
        Task::Lint => lint(),
        Task::Check => check(),
        Task::Build(profile) => build(profile),
        Task::Docs => documentation(),
        Task::Test(level) => test_suite(level),
        Task::Performance(profile) => performance(profile),
        Task::Fuzz(mode) => fuzz(mode),
        Task::Coverage => coverage(),
        Task::Mutate => mutate(),
        Task::Quality(profile) => quality(profile),
        Task::Validate(target) => validate(target),
        Task::Package => package(),
        Task::ReleasePrepare => release_prepare(),
        Task::Version(action) => version(action),
        Task::Portable(action) => portable(action),
        Task::Clean => clean_results(),
        Task::Ci(profile) => ci(profile),
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
        "config/dependency-sources.toml",
        "config/generated-outputs.toml",
        "config/host-policy.toml",
        "config/ir-encoding.toml",
        "config/quality-gates.toml",
        "config/release.toml",
        "config/repository-layout.toml",
        "config/test-levels.toml",
        "config/test-suites.toml",
        "portable/conformance/manifest.toml",
    ] {
        if !workspace_root.join(required_path).is_file() {
            return Err(format!("missing required workspace file: {required_path}"));
        }
    }

    let rustc_version = command_output(constants::RUSTC_COMMAND, &["--version"])?;
    let _cargo_version = command_output(constants::CARGO_COMMAND, &["--version"])?;
    let rust_channel = rust_channel()?;
    if !rust_version_matches_channel(&rustc_version, &rust_channel) {
        return Err(format!(
            "Rust channel {rust_channel} is required; found {rustc_version}"
        ));
    }
    let verbose_rustc = command_output(constants::RUSTC_COMMAND, &["-vV"])?;
    let host = verbose_rustc
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| "rustc -vV did not report a host target".to_owned())?;
    let host_policy = read_workspace_text(&workspace_root, "config/host-policy.toml")?;
    if !host_policy.contains(&format!("supported = [\"{host}\"]")) {
        return Err(format!(
            "unsupported release host {host}; use a host listed in config/host-policy.toml"
        ));
    }

    println!("{} environment verification: pass", constants::INFO);
    Ok(())
}

/// Verifies the complete stable, analysis, and release workstation tool set.
fn verify_complete_environment() -> Result<(), String> {
    verify_environment()?;
    let mut failures = Vec::new();
    for tool in required_tool_specs() {
        if let Err(error) = tool_version(&tool) {
            failures.push(error);
        }
    }
    if failures.is_empty() {
        println!("{} complete environment tool set: pass", constants::INFO);
        Ok(())
    } else {
        Err(format!(
            "complete environment verification failed:\n{}",
            failures.join("\n")
        ))
    }
}

/// Describes one executable and an actionable installation hint.
struct ToolSpec {
    /// Stable machine-readable manifest key.
    key: &'static str,
    /// Human-readable tool name used in diagnostics.
    label: &'static str,
    /// Executable resolved from the selected environment.
    command: &'static str,
    /// Arguments that print a bounded identity or version.
    arguments: &'static [&'static str],
    /// Action the operator can take when verification fails.
    install_hint: &'static str,
}

/// Returns the complete release-workstation tool inventory.
fn required_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            key: "rustfmt",
            label: "Rustfmt",
            command: constants::RUSTFMT_COMMAND,
            arguments: &["--version"],
            install_hint: "run `rustup component add rustfmt`",
        },
        ToolSpec {
            key: "clippy",
            label: "Clippy",
            command: constants::CARGO_COMMAND,
            arguments: &["clippy", "--version"],
            install_hint: "run `rustup component add clippy`",
        },
        ToolSpec {
            key: "rustup_active_toolchain",
            label: "Rustup",
            command: constants::RUSTUP_COMMAND,
            arguments: &["show", "active-toolchain"],
            install_hint: "install Rustup from https://rustup.rs",
        },
        ToolSpec {
            key: "nightly_rustc",
            label: "isolated nightly Rust",
            command: constants::RUSTUP_COMMAND,
            arguments: &["run", "nightly", "rustc", "--version"],
            install_hint: "run `rustup toolchain install nightly --profile minimal`",
        },
        ToolSpec {
            key: "cargo_llvm_cov",
            label: "LLVM coverage tools",
            command: constants::CARGO_COMMAND,
            arguments: &["llvm-cov", "--version"],
            install_hint: "run `rustup component add --toolchain nightly llvm-tools-preview` and `cargo install cargo-llvm-cov`",
        },
        ToolSpec {
            key: "cargo_fuzz",
            label: "coverage-guided fuzzing tools",
            command: constants::CARGO_COMMAND,
            arguments: &["fuzz", "--version"],
            install_hint: "run `cargo install cargo-fuzz`",
        },
        ToolSpec {
            key: "cargo_mutants",
            label: "mutation testing tools",
            command: constants::CARGO_COMMAND,
            arguments: &["mutants", "--version"],
            install_hint: "run `cargo install cargo-mutants`",
        },
        ToolSpec {
            key: "valgrind",
            label: "Valgrind",
            command: constants::VALGRIND_COMMAND,
            arguments: &["--version"],
            install_hint: "install the distribution `valgrind` package",
        },
        ToolSpec {
            key: "git",
            label: "Git",
            command: constants::GIT_COMMAND,
            arguments: &["--version"],
            install_hint: "install the distribution `git` package",
        },
        ToolSpec {
            key: "sh",
            label: "POSIX shell",
            command: constants::POSIX_SHELL_COMMAND,
            arguments: &["-c", "printf 'POSIX shell'"],
            install_hint: "install a POSIX-compatible `sh`",
        },
        ToolSpec {
            key: "tar",
            label: "tar",
            command: constants::TAR_COMMAND,
            arguments: &["--version"],
            install_hint: "install the distribution `tar` package",
        },
        ToolSpec {
            key: "curl",
            label: "curl",
            command: constants::CURL_COMMAND,
            arguments: &["--version"],
            install_hint: "install TLS-enabled `curl` with system certificates",
        },
        ToolSpec {
            key: "sha256sum",
            label: "SHA-256 checksum utility",
            command: constants::SHA256_COMMAND,
            arguments: &["--version"],
            install_hint: "install the distribution `coreutils` package",
        },
    ]
}

/// Returns a verified one-line tool version or an actionable error.
fn tool_version(tool: &ToolSpec) -> Result<String, String> {
    command_output(tool.command, tool.arguments)
        .map(|output| output.lines().next().unwrap_or_default().to_owned())
        .map_err(|error| {
            format!(
                "{} is unavailable ({error}); {}",
                tool.label, tool.install_hint
            )
        })
}

/// Prints the machine-readable environment manifest without writing tracked files.
fn print_environment_manifest() -> Result<(), String> {
    println!("{} {}", constants::MANIFEST, environment_manifest()?);
    Ok(())
}

/// Builds the machine-readable environment manifest used in generated evidence.
fn environment_manifest() -> Result<String, String> {
    let rustc_version = command_output(constants::RUSTC_COMMAND, &["--version"])?;
    let cargo_version = command_output(constants::CARGO_COMMAND, &["--version"])?;
    let rust_channel = rust_channel()?;
    let active_stage = active_stage()?;
    let host_image = host_image();
    let kernel = command_output(constants::UNAME_COMMAND, &["-srmo"])
        .unwrap_or_else(|error| format!("unavailable: {error}"));
    let tools = environment_tools_json();
    Ok(format!(
        concat!(
            "{{\n",
            "  \"host_image\": \"{}\",\n",
            "  \"kernel\": \"{}\",\n",
            "  \"rust_channel\": \"{}\",\n",
            "  \"rustc\": \"{}\",\n",
            "  \"cargo\": \"{}\",\n",
            "  \"active_stage\": {},\n",
            "  \"tools\": {{\n{}\n  }}\n",
            "}}"
        ),
        json_string(&host_image),
        json_string(&kernel),
        json_string(&rust_channel),
        json_string(&rustc_version),
        json_string(&cargo_version),
        active_stage,
        tools,
    ))
}

/// Returns the host operating-system identity without a user-specific path.
fn host_image() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_owned())
            })
        })
        .unwrap_or_else(|| "unknown operating system".to_owned())
}

/// Renders all specialized tool versions, retaining unavailable diagnostics.
fn environment_tools_json() -> String {
    required_tool_specs()
        .into_iter()
        .map(|tool| {
            let version =
                tool_version(&tool).unwrap_or_else(|error| format!("unavailable: {error}"));
            format!("    \"{}\": \"{}\"", tool.key, json_string(&version))
        })
        .collect::<Vec<_>>()
        .join(",\n")
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

/// Checks or applies Rust formatting across the complete workspace.
fn format_workspace(write: bool) -> Result<(), String> {
    if write {
        run_cargo(&["fmt", "--all"])
    } else {
        run_cargo(&["fmt", "--all", "--", "--check"])
    }
}

/// Runs warning-free Clippy across every workspace target and feature.
fn lint() -> Result<(), String> {
    run_cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])
}

/// Runs locked compilation plus repository boundary and coherence checks.
fn check() -> Result<(), String> {
    run_cargo(&["check", "--workspace", "--all-targets", "--locked"])?;
    check_boundaries()?;
    check_test_layout()?;
    check_traceability()?;
    check_versions()?;
    verify_portable()?;
    check_generated_outputs()?;
    check_repository_structure()?;
    check_workflow_contract()
}

/// Verifies command documentation, platform adapters, and CI stay synchronized.
fn check_workflow_contract() -> Result<(), String> {
    let root = workspace_root()?;
    for relative in [
        "README.md",
        "portable/development/00-ENVIRONMENT-AUTOMATION.md",
    ] {
        let content = read_workspace_text(&root, relative)?;
        for command in interface::DOCUMENTED_COMMANDS {
            if !content.contains(command) {
                return Err(format!("{relative} does not document `{command}`"));
            }
        }
    }

    let ci = read_workspace_text(&root, ".github/workflows/ci.yml")?;
    for command in ["cargo xtask quality --profile pr", "cargo xtask docs"] {
        if !ci.contains(command) {
            return Err(format!("ci.yml does not delegate to `{command}`"));
        }
    }
    let release = read_workspace_text(&root, ".github/workflows/release.yml")?;
    if !release.contains("cargo xtask release prepare") {
        return Err("release.yml does not delegate to `cargo xtask release prepare`".to_owned());
    }
    if ci.contains("cargo xtask ci ") || release.contains("cargo xtask ci ") {
        return Err("workflow YAML must call stable commands, not internal CI aliases".to_owned());
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
    println!("{} stable workflow contract: pass", constants::INFO);
    Ok(())
}

/// Runs the requested durable Cargo build profile.
fn build(profile: BuildProfile) -> Result<(), String> {
    match profile {
        BuildProfile::Dev => run_cargo(&["build", "--workspace", "--locked"]),
        BuildProfile::Release => run_cargo(&["build", "--workspace", "--release", "--locked"]),
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
    rustdoc_flags.push_str("-D");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("warnings");
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

/// Runs one independently selectable, stage-free test level.
fn test_suite(level: TestLevel) -> Result<(), String> {
    match level {
        TestLevel::All => {
            run_cargo(&["test", "--workspace", "--lib", "--bins", "--tests"])?;
            verify_test_counts(active_test_profile())
        }
        TestLevel::Unit => run_cargo(&["test", "--workspace", "--lib", "--bins"]),
        TestLevel::Smoke => run_shell_smoke(),
        TestLevel::Integration => run_active_test_filter("integration"),
        TestLevel::System => run_active_test_filter("system"),
        TestLevel::Conformance => run_active_test_filter("conformance"),
        TestLevel::Property => run_active_test_filter("property"),
        TestLevel::Security => run_active_test_filter("security"),
    }
}

/// Runs one active cross-package suite by stable test-name prefix.
fn run_active_test_filter(suite: &str) -> Result<(), String> {
    run_cargo(&[
        "test",
        "--workspace",
        "--lib",
        "--bins",
        "--tests",
        "--",
        &format!("{suite}_"),
    ])
}

/// Runs the selected durable fuzz mode.
fn fuzz(mode: FuzzMode) -> Result<(), String> {
    match mode {
        FuzzMode::Smoke => run_active_test_filter("fuzz_smoke"),
        FuzzMode::Campaign => coverage_guided_fuzz_campaign(),
    }
}

/// Runs every configured coverage-guided fuzz target for its approved budget.
fn coverage_guided_fuzz_campaign() -> Result<(), String> {
    let targets = quality_array("fuzz", "targets")?;
    let seconds = quality_value("fuzz", "minimum_seconds_per_target")?;
    for target in targets {
        run_cargo(&[
            "fuzz",
            "run",
            &target,
            "--",
            &format!("-max_total_time={seconds}"),
        ])?;
    }
    Ok(())
}

/// Runs workspace coverage and enforces every configured percentage threshold.
fn coverage() -> Result<(), String> {
    let lines = quality_value("coverage", "minimum_line_percent")?;
    let functions = quality_value("coverage", "minimum_function_percent")?;
    let regions = quality_value("coverage", "minimum_region_percent")?;
    let exclusions = quality_value("coverage", "exclusion_regex")?;
    let root = result_root()?;
    let html = root.join(constants::COVERAGE_HTML_DIRECTORY);
    let html_index = html.join("html/index.html");
    let json = root.join(constants::COVERAGE_JSON_FILE);
    fs::create_dir_all(
        json.parent()
            .ok_or_else(|| "coverage JSON output has no parent".to_owned())?,
    )
    .map_err(|error| format!("could not create coverage result directory: {error}"))?;
    let html = html
        .to_str()
        .ok_or_else(|| "coverage HTML path is not valid UTF-8".to_owned())?;
    let json = json
        .to_str()
        .ok_or_else(|| "coverage JSON path is not valid UTF-8".to_owned())?;
    run_cargo(&["llvm-cov", "--workspace", "--all-targets", "--no-report"])?;
    run_cargo(&[
        "llvm-cov",
        "report",
        "--html",
        "--output-dir",
        html,
        "--ignore-filename-regex",
        &exclusions,
        "--fail-under-lines",
        &lines,
        "--fail-under-functions",
        &functions,
        "--fail-under-regions",
        &regions,
    ])?;
    run_cargo(&[
        "llvm-cov",
        "report",
        "--json",
        "--summary-only",
        "--output-path",
        json,
        "--ignore-filename-regex",
        &exclusions,
        "--fail-under-lines",
        &lines,
        "--fail-under-functions",
        &functions,
        "--fail-under-regions",
        &regions,
    ])?;
    println!(
        "{} coverage HTML: {}",
        constants::INFO,
        html_index.display()
    );
    println!("{} coverage JSON: {json}", constants::INFO);
    Ok(())
}

/// Runs mutation analysis for the configured critical production target.
fn mutate() -> Result<(), String> {
    let target = quality_value("mutation", "critical_target")?;
    run_cargo(&["mutants", "--file", &target])
}

/// Runs the documented aggregate quality composition for one durable profile.
fn quality(profile: QualityProfile) -> Result<(), String> {
    format_workspace(false)?;
    lint()?;
    check()?;
    test_suite(TestLevel::All)?;
    test_suite(TestLevel::Smoke)?;
    fuzz(FuzzMode::Smoke)?;
    run_cargo(&["build", "--locked", "--package", constants::NEUTRAL_PROBE])?;
    if profile == QualityProfile::Release {
        verify_recorded_quality_gates()?;
        build(BuildProfile::Release)?;
        validate(ValidationTarget::Binaries)?;
    }
    let profile = match profile {
        QualityProfile::Pr => "pr",
        QualityProfile::Release => "release",
    };
    let result_directory = unique_generated_directory(
        &result_root()?
            .join(constants::QUALITY_REPORT_DIRECTORY)
            .join(profile),
    )?;
    write_task_summary(&result_directory, "quality", profile)?;
    println!("{} quality {profile}: pass", constants::INFO);
    Ok(())
}

/// Verifies that every configured expensive quality gate has retained pass evidence.
fn verify_recorded_quality_gates() -> Result<(), String> {
    for (section, accepted) in [
        ("coverage", "pass"),
        ("mutation", "pass"),
        ("fuzz", "full-campaign-pass"),
        ("performance", "pass-local-profiled-runner"),
    ] {
        let actual = quality_value(section, "status")?;
        if actual != accepted {
            return Err(format!(
                "quality [{section}] status must be {accepted}; found {actual}"
            ));
        }
    }
    Ok(())
}

/// Validates either built release binaries or one encoded artifact.
fn validate(target: ValidationTarget) -> Result<(), String> {
    match target {
        ValidationTarget::Binaries => {
            validate_release_binaries(&workspace_root()?.join(constants::CARGO_RELEASE_DIRECTORY))
        }
        ValidationTarget::Artifact(path) => {
            if !path.is_file() {
                return Err(format!("artifact does not exist: {}", path.display()));
            }
            let probe = release_binary_path(
                &workspace_root()?.join(constants::CARGO_RELEASE_DIRECTORY),
                constants::NEUTRAL_PROBE_BINARY,
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
fn validate_release_binaries(directory: &Path) -> Result<(), String> {
    for binary in [
        constants::NEUTRAL_CLI_BINARY,
        constants::NEUTRAL_PROBE_BINARY,
    ] {
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
    println!("{} release binaries: valid", constants::INFO);
    Ok(())
}

/// Returns a platform-correct release binary path.
fn release_binary_path(directory: &Path, binary: &str) -> PathBuf {
    let extension = if cfg!(windows) { ".exe" } else { "" };
    directory.join(format!("{binary}{extension}"))
}

/// Runs one already-resolved executable with inherited standard streams.
fn run_program(program: &Path, arguments: &[&std::ffi::OsStr]) -> Result<(), String> {
    let status = Command::new(program)
        .current_dir(workspace_root()?)
        .args(arguments)
        .status()
        .map_err(|error| format!("could not run {}: {error}", program.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} failed with {status}", program.display()))
}

/// Reads and verifies the explicit release-authority selection for a `main`-head candidate.
fn release_plan() -> Result<release::ReleasePlan, String> {
    let root = workspace_root()?;
    let package_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    let plan =
        release::ReleasePlan::read(&root.join(constants::RELEASE_CONFIG_FILE), &package_version)?;
    let residual_risks = read_workspace_text(&root, constants::RESIDUAL_RISKS_FILE)?;
    if !residual_risks.contains("Approval state: approved") {
        return Err("Stage 9 residual-risk record is not approved".to_owned());
    }
    if plan
        .channels
        .contains(&release::DistributionChannel::GithubBinaries)
    {
        let expected = BTreeSet::from([
            constants::NEUTRAL_CLI.to_owned(),
            constants::NEUTRAL_PROBE.to_owned(),
        ]);
        let selected = plan.binaries.iter().cloned().collect::<BTreeSet<_>>();
        if selected != expected {
            return Err(format!(
                "GitHub binary scope must select exactly {expected:?}; found {selected:?}"
            ));
        }
    }
    Ok(plan)
}

/// Requires a clean checked-out `main` and returns its exact current candidate commit.
fn require_main_head_checkout() -> Result<String, String> {
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

/// One immutable generated distribution file.
struct DistributionAsset {
    /// Plain filename beneath the release package directory.
    filename: String,
    /// Exact file bytes.
    bytes: Vec<u8>,
}

/// Assembles the selected binary distribution into the ignored release root.
fn package() -> Result<(), String> {
    let plan = release_plan()?;
    let candidate_commit = require_main_head_checkout()?;
    if !plan
        .channels
        .contains(&release::DistributionChannel::GithubBinaries)
    {
        return Err("GitHub binary distribution is not selected".to_owned());
    }
    for binary in &plan.binaries {
        run_cargo(&["build", "--release", "--locked", "--package", binary])?;
    }
    let root = workspace_root()?;
    let host = rust_host()?;
    let source_directory = root.join(constants::CARGO_RELEASE_DIRECTORY);
    validate_release_binaries(&source_directory)?;
    let output_directory = result_root()?
        .join(constants::RELEASE_RESULT_DIRECTORY)
        .join("package")
        .join(&plan.release_tag)
        .join(&candidate_commit)
        .join(&host);
    let assets =
        release_distribution_assets(&root, &source_directory, &plan, &candidate_commit, &host)?;
    let summary = format!(
        "{{\"release_tag\":\"{}\",\"candidate_ref\":\"main\",\"candidate_commit\":\"{}\",\"host\":\"{}\",\"channel\":\"github-binaries\",\"status\":\"assembled\"}}\n",
        json_string(&plan.release_tag),
        json_string(&candidate_commit),
        json_string(&host)
    );
    stage_binary_package(
        &root,
        &source_directory,
        &output_directory,
        &plan.binaries,
        &assets,
        &summary,
    )?;
    println!(
        "{} package assembled: {}",
        constants::INFO,
        output_directory.display()
    );
    Ok(())
}

/// Generates the source archive, SBOM, provenance, installation, manifest, and checksums.
fn release_distribution_assets(
    root: &Path,
    source_directory: &Path,
    plan: &release::ReleasePlan,
    candidate_commit: &str,
    host: &str,
) -> Result<Vec<DistributionAsset>, String> {
    let version = plan.release_tag.trim_start_matches('v');
    let source_name = format!("neutral-lang-{}-source.tar", plan.release_tag);
    let archive = Command::new("git")
        .current_dir(root)
        .args([
            "archive",
            "--format=tar",
            &format!("--prefix=neutral-lang-{version}/"),
            candidate_commit,
        ])
        .output()
        .map_err(|error| format!("could not create source archive: {error}"))?;
    if !archive.status.success() {
        return Err(format!(
            "git archive failed: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        ));
    }
    let lock_bytes = fs::read(root.join(constants::CARGO_LOCK_FILE))
        .map_err(|error| format!("could not read release dependency lock: {error}"))?;
    let mut assets = vec![
        DistributionAsset {
            filename: source_name,
            bytes: archive.stdout,
        },
        DistributionAsset {
            filename: constants::RELEASE_SBOM_FILE.to_owned(),
            bytes: lock_bytes.clone(),
        },
        DistributionAsset {
            filename: constants::RELEASE_INSTALL_FILE.to_owned(),
            bytes: format!("<!-- SPDX-License-Identifier: Apache-2.0 -->\n\n# Install Neutral {version}\n\nSupported target: `{host}`. Verify the downloaded files with `sha256sum --check SHA256SUMS`, install `neutral-cli` and `neutral-probe` into a directory on `PATH`, then run `neutral-cli --version` and `neutral-probe --help`.\n").into_bytes(),
        },
        DistributionAsset {
            filename: constants::RELEASE_PROVENANCE_FILE.to_owned(),
            bytes: format!("{{\"schema_version\":1,\"builder\":\"cargo xtask package\",\"candidate_ref\":\"main\",\"candidate_commit\":\"{}\",\"release_tag\":\"{}\",\"target\":\"{}\",\"rustc\":\"{}\",\"cargo_lock_sha256\":\"{}\",\"reproducible_command\":\"cargo xtask package\"}}\n", json_string(candidate_commit), json_string(&plan.release_tag), json_string(host), json_string(&command_output(constants::RUSTC_COMMAND, &["--version"])?), sha256_hex(&lock_bytes)).into_bytes(),
        },
    ];
    let mut entries = Vec::new();
    let mut checksums = Vec::new();
    for binary in &plan.binaries {
        let filename = release_binary_path(Path::new(""), binary)
            .to_string_lossy()
            .into_owned();
        let bytes = fs::read(release_binary_path(source_directory, binary))
            .map_err(|error| format!("could not hash release binary {binary}: {error}"))?;
        append_release_entry(
            &mut entries,
            &mut checksums,
            &filename,
            &bytes,
            "github-binaries",
            version,
            candidate_commit,
        );
    }
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
            version,
            candidate_commit,
        );
    }
    checksums.sort();
    assets.push(DistributionAsset {
        filename: constants::RELEASE_CHECKSUM_FILE.to_owned(),
        bytes: checksums.concat().into_bytes(),
    });
    assets.push(DistributionAsset {
        filename: constants::RELEASE_MANIFEST_FILE.to_owned(),
        bytes: format!("{{\"schema_version\":1,\"release_tag\":\"{}\",\"candidate_ref\":\"main\",\"candidate_commit\":\"{}\",\"license\":\"Apache-2.0\",\"supported_targets\":[\"{}\"],\"crates_io_selected\":false,\"known_limitations\":[\"single supported Linux x86_64 target\",\"no runtime or application semantics\"],\"deferred\":[\"additional host targets\",\"crates.io publication\"],\"artifacts\":[{}]}}\n", json_string(&plan.release_tag), json_string(candidate_commit), json_string(host), entries.join(",")).into_bytes(),
    });
    Ok(assets)
}

/// Adds one selected file to the release manifest and checksum list.
fn append_release_entry(
    entries: &mut Vec<String>,
    checksums: &mut Vec<String>,
    filename: &str,
    bytes: &[u8],
    channel: &str,
    version: &str,
    candidate_commit: &str,
) {
    let digest = sha256_hex(bytes);
    checksums.push(format!("{digest}  {filename}\n"));
    entries.push(format!("{{\"filename\":\"{}\",\"sha256\":\"{}\",\"license\":\"Apache-2.0\",\"producer_version\":\"{}\",\"source_commit\":\"{}\",\"channel\":\"{}\"}}", json_string(filename), digest, json_string(version), json_string(candidate_commit), json_string(channel)));
}

/// Atomically stages selected binaries, license material, and package metadata.
fn stage_binary_package(
    root: &Path,
    source_directory: &Path,
    output_directory: &Path,
    binaries: &[String],
    assets: &[DistributionAsset],
    summary: &str,
) -> Result<(), String> {
    if output_directory.exists() {
        return Err(format!(
            "package output already exists: {}; run `cargo xtask clean` before a new assembly",
            output_directory.display()
        ));
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
    fs::write(partial_directory.join("package-summary.json"), summary)
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

/// Runs release checks and assembles artifacts without tagging or publishing.
fn release_prepare() -> Result<(), String> {
    let plan = release_plan()?;
    require_main_head_checkout()?;
    quality(QualityProfile::Release)?;
    documentation()?;
    package()?;
    let result_directory = unique_generated_directory(
        &result_root()?
            .join(constants::RELEASE_RESULT_DIRECTORY)
            .join("preparation"),
    )?;
    write_task_summary(&result_directory, "release-prepare", "main")?;
    println!(
        "{} release {} prepared; no publish action was performed",
        constants::INFO,
        plan.release_tag
    );
    Ok(())
}

/// Returns the host triple reported by the selected Rust compiler.
fn rust_host() -> Result<String, String> {
    let verbose = command_output(constants::RUSTC_COMMAND, &["-vV"])?;
    verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -vV did not report a host triple".to_owned())
}

/// Executes the stable version command family.
fn version(action: VersionAction) -> Result<(), String> {
    match action {
        VersionAction::Show => show_versions(),
        VersionAction::Check => check_versions(),
        VersionAction::Prepare(requested) => prepare_version(&requested),
    }
}

/// Displays the package version separately from every frozen contract version.
fn show_versions() -> Result<(), String> {
    let root = workspace_root()?;
    let package = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    println!("{} package-release {package}", constants::INFO);
    let freeze = read_workspace_text(&root, constants::CONTRACT_FREEZE_FILE)?;
    for (name, value) in configuration_section(&freeze, "contract_versions")? {
        println!("{} contract {name}={value}", constants::INFO);
    }
    Ok(())
}

/// Checks that package versions inherit the one workspace release version.
fn check_versions() -> Result<(), String> {
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
    let freeze = read_workspace_text(&root, constants::CONTRACT_FREEZE_FILE)?;
    if configuration_value(&freeze, "status").as_deref() != Some("approved")
        || configuration_section(&freeze, "contract_versions")?.is_empty()
    {
        return Err("contract freeze must remain approved and version-complete".to_owned());
    }
    ensure_no_unreviewed_contract_changes(&root)?;
    println!("{} centralized package versions: pass", constants::INFO);
    Ok(())
}

/// Rejects ordinary version work mixed with unreviewed normative changes.
fn ensure_no_unreviewed_contract_changes(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "diff",
            "--name-only",
            "HEAD",
            "--",
            constants::CONTRACT_FREEZE_FILE,
            "portable/ARCHITECTURE.md",
            "portable/specs/REQUIREMENTS.md",
            "portable/specs/contracts",
            "portable/specs/decisions",
            "portable/specs/fixtures",
            constants::CONFORMANCE_MANIFEST_FILE,
            "portable/conformance/oracles",
            "portable/conformance/fixture-oracle-review.toml",
            "portable/development/01-IDENTITY-AND-VOCABULARY.md",
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

/// Emits a reviewable version-change plan without tagging or publishing.
fn prepare_version(requested: &str) -> Result<(), String> {
    validate_semver(requested)?;
    let root = workspace_root()?;
    let current = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    validate_version_transition(&current, requested)?;
    check_versions()?;
    let freeze_bytes = fs::read(root.join(constants::CONTRACT_FREEZE_FILE))
        .map_err(|error| format!("could not read contract freeze: {error}"))?;
    let freeze_digest = sha256_hex(&freeze_bytes);
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
    println!("{} version plan: {}", constants::INFO, output.display());
    Ok(())
}

/// Reads scalar key/value pairs from one exact TOML section.
fn configuration_section(content: &str, section: &str) -> Result<Vec<(String, String)>, String> {
    let heading = format!("[{section}]");
    let mut selected = false;
    let mut values = Vec::new();
    for line in content.lines().map(str::trim) {
        if line.starts_with('[') {
            selected = line == heading;
            continue;
        }
        if !selected || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid [{section}] entry: {line}"))?;
        values.push((
            name.trim().to_owned(),
            value.trim().trim_matches('"').to_owned(),
        ));
    }
    Ok(values)
}

/// Reads one unsectioned scalar from constrained TOML-like configuration.
fn configuration_value(content: &str, key: &str) -> Option<String> {
    content.lines().map(str::trim).find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        (candidate.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    })
}

/// Checks the root release lock and its declared dependency-source policy.
fn verify_dependency_lock(root: &Path, package_version: &str) -> Result<(), String> {
    let policy = read_workspace_text(root, constants::DEPENDENCY_SOURCES_FILE)?;
    for requirement in [
        "lockfile = \"Cargo.lock\"",
        "review = \"quality/dependency-review.md\"",
        "allow_crates_io_registry = true",
        "allow_git_sources = false",
        "allow_external_paths = false",
    ] {
        if !policy.contains(requirement) {
            return Err(format!(
                "dependency-source policy must declare `{requirement}`"
            ));
        }
    }
    let review = read_workspace_text(root, "quality/dependency-review.md")?;
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
        let package_name = manifest
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| {
                format!(
                    "package manifest has no package directory: {}",
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
    let output = Command::new(constants::CARGO_COMMAND)
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
fn verify_manifest_dependency_paths(root: &Path, manifest: &Path) -> Result<(), String> {
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
fn workspace_package_version(manifest: &str) -> Result<String, String> {
    let mut selected = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            selected = line == "[workspace.package]";
            continue;
        }
        if selected
            && let Some(value) = line
                .strip_prefix("version = \"")
                .and_then(|line| line.strip_suffix('"'))
        {
            validate_semver(value)?;
            return Ok(value.to_owned());
        }
    }
    Err("root Cargo.toml has no [workspace.package] version".to_owned())
}

/// Returns every non-root workspace package manifest.
fn workspace_package_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut manifests = Vec::new();
    for directory in [root.join("crates"), root.join("xtask")] {
        collect_named_files(&directory, "Cargo.toml", &mut manifests)?;
    }
    manifests.sort();
    Ok(manifests)
}

/// Collects regular files with one exact filename below a directory.
fn collect_named_files(
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
fn validate_semver(value: &str) -> Result<(), String> {
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
fn validate_version_transition(current: &str, requested: &str) -> Result<(), String> {
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
fn parsed_semver(value: &str) -> Result<(u64, u64, u64, Vec<String>), String> {
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
fn compare_semver(
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
fn compare_prerelease(left: &[String], right: &[String]) -> std::cmp::Ordering {
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
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

/// Executes the active portable lifecycle command family.
fn portable(action: PortableAction) -> Result<(), String> {
    match action {
        PortableAction::Verify => verify_portable(),
        PortableAction::Snapshot => snapshot_portable(),
    }
}

/// Verifies the active portable package's required roots and traceability.
fn verify_portable() -> Result<(), String> {
    let root = workspace_root()?;
    for relative in [
        constants::PORTABLE_PLAN_FILE,
        constants::PORTABLE_LIFECYCLE_FILE,
        constants::CONTRACT_FREEZE_FILE,
        constants::CONFORMANCE_MANIFEST_FILE,
    ] {
        if !root.join(relative).is_file() {
            return Err(format!("active portable file is missing: {relative}"));
        }
    }
    let package_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    let major = package_version
        .split('.')
        .next()
        .ok_or_else(|| "workspace version has no major component".to_owned())?;
    let lifecycle = read_workspace_text(&root, constants::PORTABLE_LIFECYCLE_FILE)?;
    let expected_series = format!("v{major}");
    if configuration_value(&lifecycle, "status").as_deref() != Some("active")
        || configuration_value(&lifecycle, "active_series").as_deref()
            != Some(expected_series.as_str())
    {
        return Err(format!(
            "active portable series must be {expected_series} for package {package_version}"
        ));
    }
    check_traceability()?;
    verify_fixture_freeze_digests(&root)?;
    verify_portable_links(&root)?;
    ensure_no_archived_portable_dependencies(&root)?;
    verify_existing_portable_snapshots(&root)?;
    println!("{} active portable package: pass", constants::INFO);
    Ok(())
}

/// Verifies every locally retained digest-addressed portable snapshot.
fn verify_existing_portable_snapshots(root: &Path) -> Result<(), String> {
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
fn verify_portable_snapshot_directory(root: &Path, directory: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(directory.join("manifest.sha256"))
        .map_err(|error| format!("could not read snapshot manifest: {error}"))?;
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

/// Verifies immutable fixture-manifest identities recorded by the freeze.
fn verify_fixture_freeze_digests(root: &Path) -> Result<(), String> {
    let freeze = read_workspace_text(root, constants::CONTRACT_FREEZE_FILE)?;
    for (path_key, digest_key) in [
        ("manifest_path", "manifest_sha256"),
        ("fixture_oracle_review_path", "fixture_oracle_review_sha256"),
    ] {
        let path = configuration_value(&freeze, path_key)
            .ok_or_else(|| format!("contract freeze has no {path_key}"))?;
        let expected = configuration_value(&freeze, digest_key)
            .ok_or_else(|| format!("contract freeze has no {digest_key}"))?;
        let bytes = fs::read(root.join(&path))
            .map_err(|error| format!("could not read frozen input {path}: {error}"))?;
        let actual = sha256_hex(&bytes);
        if actual != expected {
            return Err(format!(
                "frozen input {path} has SHA-256 {actual}, expected {expected}; contract review is required"
            ));
        }
    }
    Ok(())
}

/// Verifies repository-local links in every active portable Markdown file.
fn verify_portable_links(root: &Path) -> Result<(), String> {
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

/// Extracts local, anchor-free Markdown link paths from document text.
fn markdown_link_targets(content: &str) -> Vec<String> {
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

/// Rejects production or executable-test references to archived roadmap inputs.
fn ensure_no_archived_portable_dependencies(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    collect_regular_files(&root.join("crates"), &mut files)?;
    collect_regular_files(&root.join("xtask"), &mut files)?;
    let forbidden = [
        concat!("neutral-roadmap", "/neutral-lang/"),
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
            forbidden
                .iter()
                .any(|value| content.contains(value))
                .then(|| {
                    path.strip_prefix(root)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                })
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

/// Creates an atomic digest-addressed copy of the active portable package.
fn snapshot_portable() -> Result<(), String> {
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
    let manifest = format!(
        "# SPDX-License-Identifier: Apache-2.0\n# sha256  bytes  repository-relative-path\n{records}"
    );
    if output.exists() {
        let existing = fs::read_to_string(output.join("manifest.sha256"))
            .map_err(|error| format!("could not read existing snapshot manifest: {error}"))?;
        if existing != manifest {
            return Err(format!(
                "portable snapshot collision at {}",
                output.display()
            ));
        }
        println!(
            "{} portable snapshot: {}",
            constants::INFO,
            output.display()
        );
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
        "{{\"schema_version\":1,\"tree_sha256\":\"{tree_digest}\",\"file_count\":{},\"archive_repository\":\"{}\",\"archive_path\":\"{}\",\"next_series\":\"{}\",\"status\":\"review-required\"}}\n",
        files.len(),
        json_string(&configuration_value(&lifecycle, "archive_repository").unwrap_or_default()),
        json_string(&configuration_value(&lifecycle, "archive_path").unwrap_or_default()),
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
    println!(
        "{} portable snapshot: {}",
        constants::INFO,
        output.display()
    );
    Ok(())
}

/// Builds sorted exact-byte digest records for active portable files.
fn portable_digest_records(root: &Path, files: &[PathBuf]) -> Result<String, String> {
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

/// Verifies generated-output ownership and ignored tracking policy.
fn check_generated_outputs() -> Result<(), String> {
    let root = workspace_root()?;
    let inventory = read_workspace_text(&root, constants::GENERATED_OUTPUTS_FILE)?;
    for name in [
        "rustdoc",
        "coverage",
        "fuzz",
        "mutation",
        "benchmark",
        "package",
        "sbom",
        "release",
        "portable-snapshot",
    ] {
        if !inventory.contains(&format!("name = \"{name}\"")) {
            return Err(format!("generated-output inventory is missing {name}"));
        }
    }
    for entry in inventory.split("[[output]]").skip(1) {
        let path = configuration_value(entry, "path")
            .ok_or_else(|| "generated output has no path".to_owned())?;
        if !(path.starts_with("target/") || path.starts_with("test-results/"))
            || configuration_value(entry, "tracking").as_deref() != Some("ignored")
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
        || !ignore.lines().any(|line| line.trim() == "test-results/")
    {
        return Err("target and test-results must remain ignored generated roots".to_owned());
    }
    println!("{} generated-output ownership: pass", constants::INFO);
    Ok(())
}

/// Verifies directory ownership, durable test levels, fuzz ownership, and hygiene.
fn check_repository_structure() -> Result<(), String> {
    let root = workspace_root()?;
    let layout = read_workspace_text(&root, constants::REPOSITORY_LAYOUT_FILE)?;
    let expected = BTreeSet::from([
        ".cargo".to_owned(),
        ".devcontainer".to_owned(),
        ".github".to_owned(),
        "config".to_owned(),
        "crates".to_owned(),
        "fuzz".to_owned(),
        "portable".to_owned(),
        "quality".to_owned(),
        "scripts".to_owned(),
        "xtask".to_owned(),
    ]);
    let mut configured = BTreeSet::new();
    for entry in layout.split("[[directory]]").skip(1) {
        let path = configuration_value(entry, "path")
            .ok_or_else(|| "repository directory has no path".to_owned())?;
        let readme = configuration_value(entry, "readme")
            .ok_or_else(|| format!("repository directory {path} has no README"))?;
        if configuration_value(entry, "owner").is_none()
            || configuration_value(entry, "lifecycle").is_none()
            || !root.join(&path).is_dir()
            || !root.join(&readme).is_file()
        {
            return Err(format!(
                "repository directory {path} has incomplete ownership"
            ));
        }
        let readme_content = read_workspace_text(&root, &readme)?;
        if !readme_content.starts_with("<!-- SPDX-License-Identifier: Apache-2.0 -->") {
            return Err(format!(
                "repository README lacks its license marker: {readme}"
            ));
        }
        configured.insert(path);
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
    println!("{} repository ownership and hygiene: pass", constants::INFO);
    Ok(())
}

/// Verifies generated-state ignores without allowing source-of-truth inputs.
fn verify_ignore_policy(root: &Path) -> Result<(), String> {
    let ignore = read_workspace_text(root, ".gitignore")?;
    for required in [
        "target",
        "test-results/",
        "**/mutants.out*/",
        "fuzz/artifacts/",
        "fuzz/corpus/*/",
        "fuzz/coverage/",
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
    for required in [
        "Cargo.lock",
        "rust-toolchain.toml",
        "config/release.toml",
        "portable/specs/contracts/freeze.toml",
        "portable/conformance/manifest.toml",
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
fn verify_coverage_policy() -> Result<(), String> {
    for (key, expected) in [
        ("toolchain", "nightly-only"),
        (
            "html_output",
            "test-results/analysis/coverage/html/index.html",
        ),
        (
            "json_output",
            "test-results/analysis/coverage/coverage.json",
        ),
        ("minimum_line_percent", "85"),
        ("minimum_function_percent", "90"),
        ("minimum_region_percent", "80"),
    ] {
        if quality_value("coverage", key)? != expected {
            return Err(format!(
                "coverage policy {key} must remain configured as {expected}"
            ));
        }
    }
    if quality_value("coverage", "exclusion_regex")? != "(^|/)xtask/" {
        return Err("coverage may exclude only the separately command-tested xtask".to_owned());
    }
    let exclusions = quality_value("coverage", "exclusion_policy")?;
    if !exclusions.starts_with("reviewed:") || !exclusions.contains("only xtask") {
        return Err("coverage exclusions require an explicit reviewed rationale".to_owned());
    }
    Ok(())
}

/// Verifies every durable test purpose has one documented stable command.
fn verify_test_level_inventory(root: &Path) -> Result<(), String> {
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
fn verify_fuzz_ownership(root: &Path) -> Result<(), String> {
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
fn verify_generated_file_hygiene(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "ls-files",
            "--",
            "target",
            "test-results",
            "mutants.out",
            "mutants.out.old",
            "fuzz/artifacts",
            "fuzz/corpus",
            "portable/archive",
            "portable/archived",
        ])
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

/// Reads one scalar value from a section of the quality configuration.
fn quality_value(section: &str, key: &str) -> Result<String, String> {
    let configuration_path = workspace_root()?.join(constants::QUALITY_GATES_FILE);
    let configuration = fs::read_to_string(&configuration_path)
        .map_err(|error| format!("could not read {}: {error}", configuration_path.display()))?;
    quality_value_from(&configuration, section, key)
        .ok_or_else(|| format!("quality configuration has no [{section}] {key} value"))
}

/// Extracts one scalar value from a simple TOML section without interpreting it.
fn quality_value_from(configuration: &str, section: &str, key: &str) -> Option<String> {
    let heading = format!("[{section}]");
    let mut selected = false;
    for line in configuration.lines().map(str::trim) {
        if line.starts_with('[') {
            selected = line == heading;
            continue;
        }
        if !selected || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() == key {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Reads one quoted-string array from the quality configuration.
fn quality_array(section: &str, key: &str) -> Result<Vec<String>, String> {
    let value = quality_value(section, key)?;
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("quality [{section}] {key} must be an array"))?;
    let values = inner
        .split(',')
        .map(str::trim)
        .map(|value| value.trim_matches('"'))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if values.is_empty() {
        Err(format!("quality [{section}] {key} array is empty"))
    } else {
        Ok(values)
    }
}

/// Runs one controlled benchmark, stress, or soak profile.
fn performance(profile: PerformanceProfile) -> Result<(), String> {
    let profile = match profile {
        PerformanceProfile::Pr => "pr",
        PerformanceProfile::Release => "release",
        PerformanceProfile::Soak => "soak",
    };
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
        _ => unreachable!("performance profile is closed by command parsing"),
    }
}

/// Runs the built CLI and probe command-shell smoke checks.
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

/// Verifies that every current test category has its configured minimum.
fn verify_test_counts(profile: &str) -> Result<(), String> {
    let test_list = command_output(
        constants::CARGO_COMMAND,
        &[
            "test",
            "--workspace",
            "--lib",
            "--bins",
            "--tests",
            "--",
            "--list",
        ],
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

/// Returns the durable current test-minimum profile.
fn active_test_profile() -> &'static str {
    constants::CURRENT_TEST_PROFILE
}

/// Reads one test-minimum configuration owned by the workspace.
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

/// Runs one internal CI profile using only stable public task compositions.
fn ci(profile: CiProfile) -> Result<(), String> {
    match profile {
        CiProfile::Pr => run_ci_gate("pr", QualityProfile::Pr),
        CiProfile::Release => run_ci_gate("release", QualityProfile::Release),
    }
}

/// Runs a stable quality composition and writes its generated CI summary.
fn run_ci_gate(profile: &str, quality_profile: QualityProfile) -> Result<(), String> {
    verify_environment()?;
    quality(quality_profile)?;
    documentation()?;

    let result_directory = unique_result_directory(profile)?;
    write_task_summary(&result_directory, "ci", profile)?;
    println!("{} CI {profile}: pass", constants::INFO);
    Ok(())
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
    unique_generated_directory(&profile_root)
}

/// Creates a process-unique directory below one approved generated-output root.
fn unique_generated_directory(parent: &Path) -> Result<PathBuf, String> {
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

/// Writes one minimal machine-readable passing task summary.
fn write_task_summary(directory: &Path, task: &str, profile: &str) -> Result<(), String> {
    fs::write(
        directory.join("task-summary.json"),
        format!(
            "{{\"task\":\"{}\",\"profile\":\"{}\",\"status\":\"pass\"}}\n",
            json_string(task),
            json_string(profile)
        ),
    )
    .map_err(|error| format!("could not write task summary: {error}"))
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
    verify_pure_source_effects(&workspace_root)?;
    verify_release_path_independence(&workspace_root)?;
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

/// Rejects ambient host APIs from the captured-compilation source closure.
fn verify_pure_source_effects(root: &Path) -> Result<(), String> {
    for package in [
        constants::NEUTRAL_CORE,
        constants::NEUTRAL_IR,
        constants::NEUTRAL_VOCABULARY,
        constants::NEUTRAL_COMPILER,
    ] {
        let mut files = Vec::new();
        collect_regular_files(&root.join("crates").join(package).join("src"), &mut files)?;
        for file in files
            .iter()
            .filter(|path| path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs"))
        {
            let content = fs::read_to_string(file)
                .map_err(|error| format!("could not read {}: {error}", file.display()))?;
            for forbidden in [
                "std::fs",
                "std::env",
                "std::net",
                "std::process",
                "Command::new",
            ] {
                if content.contains(forbidden) {
                    return Err(format!(
                        "pure compilation source {} contains ambient API {forbidden}",
                        file.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Rejects user-specific absolute paths from release configuration and adapters.
fn verify_release_path_independence(root: &Path) -> Result<(), String> {
    for relative in [
        constants::RELEASE_CONFIG_FILE,
        "xtask/src/release.rs",
        "scripts/linux/release.sh",
        "scripts/win/release.ps1",
    ] {
        let content = read_workspace_text(root, relative)?;
        for forbidden in [
            "/home/",
            "C:\\Users\\",
            "$HOME",
            "${HOME}",
            "%USERPROFILE%",
            "~/",
        ] {
            if content.contains(forbidden) {
                return Err(format!(
                    "release surface {relative} contains user-specific path marker {forbidden}"
                ));
            }
        }
    }
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
        (constants::XTASK, set(["sha2"])),
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
