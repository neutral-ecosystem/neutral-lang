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
    assert!(stdout.lines().all(|line| line.starts_with("[info] HELP  ")));
    assert!(stdout.contains("cargo xtask"));
    assert!(stderr.starts_with("[info] START xtask --help\n"));
    assert!(stderr.contains("[info] PASS  xtask --help ("));
}

/// Parse failures produce one categorized error, not duplicate failure summaries.
#[test]
fn invalid_command_has_one_failure() {
    let output = invoke(&["not-a-command"]);
    assert!(!output.status.success());
    let (stdout, stderr) = streams(&output);
    assert_eq!(stdout, "");
    assert!(stderr.starts_with("[info] START xtask not-a-command\n"));
    assert_eq!(stderr.matches("[error] FAIL").count(), 1);
    assert!(!stderr.contains("[info] PASS"));
}

/// The tag command retains a bare manifest-derived value on stdout.
#[test]
fn release_tag_keeps_machine_stdout() {
    let output = invoke(&["release", "tag"]);
    assert!(output.status.success());
    let (stdout, stderr) = streams(&output);
    assert_eq!(stdout, format!("v{}\n", env!("CARGO_PKG_VERSION")));
    assert!(stderr.starts_with("[info] START xtask release tag\n"));
    assert!(stderr.contains("[info] PASS  xtask release tag ("));
}

/// Ordinary commands leave stdout free for script use and use styled human rows.
#[test]
fn version_show_uses_uniform_stderr() {
    let output = invoke(&["version", "show"]);
    assert!(output.status.success());
    let (stdout, stderr) = streams(&output);
    assert_eq!(stdout, "");
    assert!(stderr.lines().all(|line| line.starts_with("[info] ")));
    assert!(stderr.contains("[info] INFO  package-release"));
    assert!(stderr.contains("[info] PASS  xtask version show ("));
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
    assert!(stderr.starts_with("[info] START xtask environment manifest\n"));
    assert!(stderr.contains("[info] PASS  xtask environment manifest ("));
}
