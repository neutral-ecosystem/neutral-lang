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
