// SPDX-License-Identifier: Apache-2.0

//! Subprocess contracts for human reporting and script-facing command output.

use std::process::{Command, Output};

/// Runs the actual executable with deterministic plain terminal reporting.
fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(arguments)
        .env("NO_COLOR", "1")
        .env("CARGO_TERM_COLOR", "always")
        .output()
        .expect("xtask process")
}

/// Decodes streams and ensures redirected output contains no terminal escapes.
fn streams(output: &Output) -> (String, String) {
    let stdout = String::from_utf8(output.stdout.clone()).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr.clone()).expect("UTF-8 stderr");
    assert!(!stdout.contains('\x1b'));
    assert!(!stderr.contains('\x1b'));
    (stdout, stderr)
}

/// Help uses the same row style and lifecycle as other commands.
#[test]
fn help_has_uniform_rows_and_lifecycle() {
    let output = invoke(&["--help"]);
    assert!(output.status.success());
    let (stdout, stderr) = streams(&output);
    assert!(stdout.lines().all(|line| line.starts_with("HELP  ")));
    assert!(stdout.contains("cargo xtask"));
    assert!(stderr.starts_with("START xtask --help\n"));
    assert!(stderr.contains("PASS  xtask --help ("));
}

/// Parse failures produce one categorized error, not duplicate failure summaries.
#[test]
fn invalid_command_has_one_failure() {
    let output = invoke(&["not-a-command"]);
    assert!(!output.status.success());
    let (stdout, stderr) = streams(&output);
    assert_eq!(stdout, "");
    assert!(stderr.starts_with("START xtask not-a-command\n"));
    assert_eq!(stderr.matches("FAIL").count(), 1);
    assert!(!stderr.contains("PASS"));
}

/// The tag command retains a bare manifest-derived value on stdout.
#[test]
fn release_tag_keeps_machine_stdout() {
    let output = invoke(&["release", "tag"]);
    assert!(output.status.success());
    let (stdout, stderr) = streams(&output);
    assert_eq!(stdout, format!("v{}\n", env!("CARGO_PKG_VERSION")));
    assert!(stderr.starts_with("START xtask release tag\n"));
    assert!(stderr.contains("PASS  xtask release tag ("));
}

/// Ordinary commands leave stdout free for script use and use styled human rows.
#[test]
fn version_show_uses_uniform_stderr() {
    let output = invoke(&["version", "show"]);
    assert!(output.status.success());
    let (stdout, stderr) = streams(&output);
    assert_eq!(stdout, "");
    assert!(stderr.lines().all(|line| {
        ["START ", "INFO  ", "PASS  "]
            .iter()
            .any(|label| line.starts_with(label))
    }));
    assert!(stderr.contains("INFO  package-release"));
    assert!(stderr.contains("PASS  xtask version show ("));
}

/// Environment evidence keeps the documented manifest prefix and valid JSON body.
#[test]
fn environment_manifest_keeps_machine_stdout() {
    let output = invoke(&["environment", "manifest"]);
    assert!(output.status.success());
    let (stdout, stderr) = streams(&output);
    let json = stdout.strip_prefix("[manifest] ").expect("manifest prefix");
    let value: serde_json::Value = serde_json::from_str(json).expect("manifest JSON");
    assert!(value.is_object());
    assert!(stderr.starts_with("START xtask environment manifest\n"));
    assert!(stderr.contains("PASS  xtask environment manifest ("));
}

/// Invalid generated-output configuration is rejected before any tracked version write.
#[test]
fn version_prepare_preflights_output_before_metadata_writes() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let root =
        std::env::temp_dir().join(format!("neutral-version-preflight-{}", std::process::id()));
    assert!(
        Command::new("git")
            .args(["clone", "--quiet", "--shared"])
            .arg(source)
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    // The tested executable is new; the disposable HEAD snapshot may predate this ignore rule.
    std::fs::write(root.join(".git/info/exclude"), ".neutral-version-update/\n").unwrap();
    // The executable and its required configuration schema must come from the same inputs.
    let automation = "config/automation.toml";
    if std::fs::read(source.join(automation)).unwrap()
        != std::fs::read(root.join(automation)).unwrap()
    {
        std::fs::copy(source.join(automation), root.join(automation)).unwrap();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args([
                    "-c",
                    "user.name=Neutral test",
                    "-c",
                    "user.email=test@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "--quiet",
                    "--only",
                    "-m",
                    "Use current automation schema",
                    "--",
                    automation,
                ])
                .status()
                .unwrap()
                .success()
        );
    }
    let manifest = std::fs::read(root.join("Cargo.toml")).unwrap();
    let lock = std::fs::read(root.join("Cargo.lock")).unwrap();
    let parsed: toml::Value = toml::from_str(std::str::from_utf8(&manifest).unwrap()).unwrap();
    let current = parsed["workspace"]["package"]["version"].as_str().unwrap();
    let mut parts = current.split('.');
    let requested = format!("{}.{}.9999", parts.next().unwrap(), parts.next().unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(&root)
        .args(["version", "prepare", &requested])
        .env("NEUTRAL_TEST_RESULTS", "../unsafe")
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success());
    assert!(
        stderr.contains("NEUTRAL_TEST_RESULTS must be a relative path"),
        "{stderr}"
    );
    assert_eq!(std::fs::read(root.join("Cargo.toml")).unwrap(), manifest);
    assert_eq!(std::fs::read(root.join("Cargo.lock")).unwrap(), lock);
    assert!(
        !root
            .join(format!("quality/evidence/v{requested}/README.md"))
            .exists()
    );
    std::fs::remove_dir_all(root).unwrap();
}
