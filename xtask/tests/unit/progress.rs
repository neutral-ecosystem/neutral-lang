// SPDX-License-Identifier: Apache-2.0

use super::*;

/// The configured budget produces a readable countdown and midpoint bar.
#[test]
fn shows_elapsed_and_remaining_budget() {
    let line = render("source", 150, 300);
    assert!(line.contains("[==========          ] 50%"));
    assert!(line.contains("elapsed 2m 30s"));
    assert!(line.contains("ETA ~2m 30s"));
}

/// Exceeding the estimate never claims successful completion or underflows.
#[test]
fn overdue_tool_still_waits_for_success() {
    let line = render("ir", 400, 300);
    assert!(line.contains("99%"));
    assert!(line.contains("waiting for tool"));
    assert!(!line.contains("100%"));
    assert!(render("probe", 0, 0).contains("waiting for tool"));
}

/// Waiting preserves a child's actual failure rather than accepting elapsed time.
#[test]
fn preserves_child_exit_status() {
    let mut child = std::process::Command::new(crate::configuration::rustc_command().unwrap())
        .arg("--not-a-real-rustc-option")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    assert!(!wait(&mut child, "test", 300).unwrap().success());
}
