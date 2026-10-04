// SPDX-License-Identifier: Apache-2.0

//! Workstation verification, tool inventory, and environment evidence.

use super::*;

/// Creates ignored automation-result directories and records the local environment.
pub(super) fn bootstrap() -> Result<(), String> {
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
pub(super) fn verify_environment() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    for required_path in [
        "Cargo.lock",
        "rust-toolchain.toml",
        constants::AUTOMATION_CONFIG_FILE,
        "config/dependency-sources.toml",
        constants::CONFORMANCE_CONFIG_FILE,
        "config/generated-outputs.toml",
        "config/host-policy.toml",
        "config/ir-encoding.toml",
        "config/quality-gates.toml",
        "config/release.toml",
        "config/repository-layout.toml",
        "config/test-levels.toml",
        constants::TEST_SUITES_FILE,
        constants::QUALITY_MANIFEST_FILE,
    ] {
        if !workspace_root.join(required_path).is_file() {
            return Err(format!("missing required workspace file: {required_path}"));
        }
    }

    let inherited_manifest =
        ReleasedBundle::load(&workspace_root)?.member("conformance/manifest.toml");
    if !workspace_root.join(&inherited_manifest).is_file() {
        return Err(format!(
            "missing required workspace file: {inherited_manifest}"
        ));
    }

    let rustc_version = command_output(&rustc_command()?, &["--version"])?;
    let _cargo_version = command_output(&cargo_command()?, &["--version"])?;
    let rust_channel = rust_channel()?;
    if !rust_version_matches_channel(&rustc_version, &rust_channel) {
        return Err(format!(
            "Rust channel {rust_channel} is required; found {rustc_version}"
        ));
    }
    let verbose_rustc = command_output(&rustc_command()?, &["-vV"])?;
    let host = verbose_rustc
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| "rustc -vV did not report a host target".to_owned())?;
    let host_policy = read_workspace_text(&workspace_root, "config/host-policy.toml")?;
    if !configuration_array_from(&host_policy, "hosts", "supported")?
        .iter()
        .any(|supported| supported == host)
    {
        return Err(format!(
            "unsupported release host {host}; use a host listed in config/host-policy.toml"
        ));
    }

    test_execution::verify_runner()?;
    println!("{} environment verification: pass", constants::INFO);
    Ok(())
}

/// Verifies the complete stable, analysis, and release workstation tool set.
pub(super) fn verify_complete_environment() -> Result<(), String> {
    verify_environment()?;
    let mut failures = Vec::new();
    for tool in required_tool_specs()? {
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
pub(super) struct ToolSpec {
    /// Stable machine-readable manifest key.
    pub(super) key: &'static str,
    /// Human-readable tool name used in diagnostics.
    pub(super) label: &'static str,
    /// Executable resolved from the selected environment.
    pub(super) command: String,
    /// Arguments that print a bounded identity or version.
    pub(super) arguments: &'static [&'static str],
    /// Action the operator can take when verification fails.
    pub(super) install_hint: &'static str,
}

/// Describes the advisory checker shared by release qualification and workstation checks.
fn advisory_tool_spec(cargo: &str) -> ToolSpec {
    ToolSpec {
        key: "cargo_audit",
        label: "dependency advisory checker",
        command: cargo.to_owned(),
        arguments: &["audit", "--version"],
        install_hint: "run `cargo install cargo-audit --locked`",
    }
}

/// Describes the optional compatibility backend's standard process-isolated runner.
fn nextest_tool_spec(cargo: &str) -> ToolSpec {
    ToolSpec {
        key: "cargo_nextest",
        label: "process-isolated test runner",
        command: cargo.to_owned(),
        arguments: &["nextest", "--version"],
        install_hint: "run `cargo install cargo-nextest --locked`",
    }
}

/// Returns the complete release-workstation tool inventory.
pub(super) fn required_tool_specs() -> Result<Vec<ToolSpec>, String> {
    let cargo = cargo_command()?;
    Ok(vec![
        nextest_tool_spec(&cargo),
        ToolSpec {
            key: "rustfmt",
            label: "Rustfmt",
            command: constants::RUSTFMT_COMMAND.to_owned(),
            arguments: &["--version"],
            install_hint: "run `rustup component add rustfmt`",
        },
        ToolSpec {
            key: "clippy",
            label: "Clippy",
            command: cargo.clone(),
            arguments: &["clippy", "--version"],
            install_hint: "run `rustup component add clippy`",
        },
        ToolSpec {
            key: "rustup_active_toolchain",
            label: "Rustup",
            command: constants::RUSTUP_COMMAND.to_owned(),
            arguments: &["show", "active-toolchain"],
            install_hint: "install Rustup from https://rustup.rs",
        },
        ToolSpec {
            key: "nightly_rustc",
            label: "isolated nightly Rust",
            command: constants::RUSTUP_COMMAND.to_owned(),
            arguments: &["run", "nightly", "rustc", "--version"],
            install_hint: "run `rustup toolchain install nightly --profile minimal`",
        },
        ToolSpec {
            key: "cargo_llvm_cov",
            label: "LLVM coverage tools",
            command: cargo.clone(),
            arguments: &["llvm-cov", "--version"],
            install_hint: "run `rustup component add --toolchain nightly llvm-tools-preview` and `cargo install cargo-llvm-cov`",
        },
        ToolSpec {
            key: "cargo_fuzz",
            label: "coverage-guided fuzzing tools",
            command: cargo.clone(),
            arguments: &["fuzz", "--version"],
            install_hint: "run `cargo install cargo-fuzz`",
        },
        ToolSpec {
            key: "cargo_mutants",
            label: "mutation testing tools",
            command: cargo.clone(),
            arguments: &["mutants", "--version"],
            install_hint: "run `cargo install cargo-mutants`",
        },
        advisory_tool_spec(&cargo),
        ToolSpec {
            key: "valgrind",
            label: "Valgrind",
            command: constants::VALGRIND_COMMAND.to_owned(),
            arguments: &["--version"],
            install_hint: "install the distribution `valgrind` package",
        },
        ToolSpec {
            key: "git",
            label: "Git",
            command: constants::GIT_COMMAND.to_owned(),
            arguments: &["--version"],
            install_hint: "install the distribution `git` package",
        },
        ToolSpec {
            key: "sh",
            label: "POSIX shell",
            command: constants::POSIX_SHELL_COMMAND.to_owned(),
            arguments: &["-c", "printf 'POSIX shell'"],
            install_hint: "install a POSIX-compatible `sh`",
        },
        ToolSpec {
            key: "tar",
            label: "tar",
            command: constants::TAR_COMMAND.to_owned(),
            arguments: &["--version"],
            install_hint: "install the distribution `tar` package",
        },
        ToolSpec {
            key: "curl",
            label: "curl",
            command: constants::CURL_COMMAND.to_owned(),
            arguments: &["--version"],
            install_hint: "install TLS-enabled `curl` with system certificates",
        },
        ToolSpec {
            key: "sha256sum",
            label: "SHA-256 checksum utility",
            command: constants::SHA256_COMMAND.to_owned(),
            arguments: &["--version"],
            install_hint: "install the distribution `coreutils` package",
        },
    ])
}

/// Returns a verified one-line tool version or an actionable error.
pub(super) fn tool_version(tool: &ToolSpec) -> Result<String, String> {
    command_output(&tool.command, tool.arguments)
        .map(|output| output.lines().next().unwrap_or_default().to_owned())
        .map_err(|error| {
            format!(
                "{} is unavailable ({error}); {}",
                tool.label, tool.install_hint
            )
        })
}

/// Prints the machine-readable environment manifest without writing tracked files.
pub(super) fn print_environment_manifest() -> Result<(), String> {
    println!("{} {}", constants::MANIFEST, environment_manifest()?);
    Ok(())
}

/// Builds the machine-readable environment manifest used in generated evidence.
pub(super) fn environment_manifest() -> Result<String, String> {
    let rustc_version = command_output(&rustc_command()?, &["--version"])?;
    let cargo_version = command_output(&cargo_command()?, &["--version"])?;
    let rust_channel = rust_channel()?;
    let active_stage = active_stage()?;
    let host_image = host_image();
    let kernel = command_output(constants::UNAME_COMMAND, &["-srmo"])
        .unwrap_or_else(|error| format!("unavailable: {error}"));
    let tools = environment_tools_json()?;
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
pub(super) fn host_image() -> String {
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
pub(super) fn environment_tools_json() -> Result<String, String> {
    Ok(required_tool_specs()?
        .into_iter()
        .map(|tool| {
            let version =
                tool_version(&tool).unwrap_or_else(|error| format!("unavailable: {error}"));
            format!("    \"{}\": \"{}\"", tool.key, json_string(&version))
        })
        .collect::<Vec<_>>()
        .join(",\n"))
}

/// Reads the selected Rust channel from the repository toolchain manifest.
pub(super) fn rust_channel() -> Result<String, String> {
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
pub(super) fn rust_version_matches_channel(rustc_version: &str, channel: &str) -> bool {
    if channel == "stable" {
        rustc_version.starts_with("rustc ")
            && !["-nightly", "-beta", "-dev"]
                .iter()
                .any(|marker| rustc_version.contains(marker))
    } else {
        rustc_version.starts_with(&format!("rustc {channel} "))
    }
}

/// Derives the active conformance stage from required portable manifest suites.
pub(super) fn active_stage() -> Result<u8, String> {
    portable_stage::active_conformance_stage(&workspace_root()?)
}

/// Escapes a string for the limited JSON values emitted by automation evidence.
pub(super) fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                write!(&mut escaped, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            character => escaped.push(character),
        }
    }
    escaped
}
