// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Durations remain readable across subsecond, minute, and hour boundaries.
#[test]
fn duration_uses_compact_units() {
    assert_eq!(duration(Duration::from_millis(12)), "12ms");
    assert_eq!(duration(Duration::from_millis(1500)), "1.5s");
    assert_eq!(duration(Duration::from_secs(301)), "5m 01s");
    assert_eq!(duration(Duration::from_secs(3661)), "1h 01m 01s");
}

/// Commands keep whitespace, quotes, and shell metacharacters safely delimited.
#[test]
fn command_quotes_arguments() {
    assert_eq!(
        command("cargo", &["test", "a b", "it's", "$(id)", ""]),
        "cargo test 'a b' 'it'\\''s' '$(id)' ''"
    );
}

/// Workspace paths shorten without changing paths belonging to another directory.
#[test]
fn paths_are_relative_only_within_workspace() {
    let root = crate::workspace_root().unwrap();
    assert_eq!(
        path(&root.join("test-results/report.json")),
        "test-results/report.json"
    );
    assert_eq!(path(Path::new("/tmp/report.json")), "/tmp/report.json");
}

/// Color is automatic only on capable terminals and respects explicit opt-outs.
#[test]
fn color_controls_preserve_plain_logs() {
    assert!(color_policy(true, Some("xterm-256color"), false, None));
    assert!(!color_policy(false, Some("xterm"), false, None));
    assert!(!color_policy(true, Some("dumb"), false, None));
    assert!(!color_policy(true, None, false, Some("never")));
    assert!(color_policy(false, Some("dumb"), false, Some("always")));
    assert!(!color_policy(true, None, true, Some("always")));
}

/// Human rows normalize status, command, and file labels without terminal escapes.
#[test]
fn human_status_rows_are_consistent() {
    assert_eq!(
        format_row("PASS", "quality [2/7] check (1.2s)", false),
        "PASS  quality [2/7] check (1.2s)"
    );
    assert_eq!(
        format_row("START", "xtask check", false),
        "START xtask check"
    );
    assert_eq!(format_row("CMD", "cargo check", false), "CMD   cargo check");
    assert_eq!(
        format_row("FILE", "reports: test-results/report", false),
        "FILE  reports: test-results/report"
    );
    assert!(format_row("INFO", "test: passing is not complete", false).contains("INFO"));
    assert!(!format_row("FAIL", "failed", false).contains('\x1b'));
}

/// Semantic status colors reset before the message, while text remains understandable.
#[test]
fn colors_use_distinct_statuses_and_reset() {
    let success = format_row("PASS", "coverage", true);
    assert_eq!(success, "\x1b[32mPASS \x1b[0m coverage");
    assert!(format_row("WARN", "warning", true).starts_with("\x1b[33m"));
    assert!(format_row("FAIL", "failure", true).starts_with("\x1b[31m"));
}
