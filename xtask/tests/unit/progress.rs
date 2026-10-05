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

/// Successful completion fills the entire bar without claiming evidence approval.
#[test]
fn completed_tool_reaches_one_hundred_percent() {
    for elapsed in [0, 150, 301] {
        let line = render_status("source", elapsed, 300, true);
        assert!(line.contains("[====================] 100%"));
        assert!(line.contains("tool complete; evidence validation follows"));
        assert!(!line.contains("waiting for tool"));
    }
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
    assert!(!wait(&mut child, "test", Some(300)).unwrap().success());
}

/// Unbudgeted measurements preserve successful exit without inventing a deadline.
#[test]
fn unbudgeted_tool_can_succeed() {
    let mut child = std::process::Command::new(crate::configuration::rustc_command().unwrap())
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    assert!(wait(&mut child, "test", None).unwrap().success());
}
