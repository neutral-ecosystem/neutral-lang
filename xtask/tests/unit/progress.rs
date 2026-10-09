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
    assert!(!wait(&mut child, "test", Some(300), None).unwrap().success());
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
    assert!(wait(&mut child, "test", None, None).unwrap().success());
}

/// Mutation progress counts finished mutants, excluding baseline and incomplete reports.
#[test]
fn mutation_counts_use_discovered_total_and_finished_outcomes() {
    assert_eq!(
        mutation_counts("[{}, {}, {}]", r#"{"total_mutants":2}"#),
        Some((2, 3))
    );
    assert_eq!(
        mutation_counts("[{}]", r#"{"total_mutants":0}"#),
        Some((0, 1))
    );
    assert_eq!(mutation_counts("[{}]", "{"), None);
    assert_eq!(mutation_counts("[{}]", r#"{"total_mutants":2}"#), None);
    assert_eq!(mutation_counts("[]", r#"{"total_mutants":0}"#), None);
}

/// Completed counters must explain survivors rather than making 100% look like success.
#[test]
fn mutation_progress_explains_completed_but_failed_campaigns() {
    let directory =
        std::env::temp_dir().join(format!("neutral-mutation-progress-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("mutants.json"), "[{}, {}, {}, {}]").unwrap();
    fs::write(
        directory.join("outcomes.json"),
        r#"{"total_mutants":4,"caught":1,"missed":1,"unviable":1,"timeout":1}"#,
    )
    .unwrap();
    let line = mutation_progress(&directory, "mutation", Duration::from_secs(10)).unwrap();
    assert!(line.contains("mutants 4/4"));
    assert!(line.contains("caught 1 | missed 1 | unviable 1 | timed out 1"));
    assert!(!line.starts_with("PASS"));
    fs::write(directory.join("outcomes.json"), "{").unwrap();
    assert_eq!(
        mutation_progress(&directory, "mutation", Duration::ZERO),
        None
    );
    fs::remove_dir_all(directory).unwrap();
}
