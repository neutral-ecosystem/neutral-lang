// SPDX-License-Identifier: Apache-2.0

//! Pure progress-parser tests.

use super::{parse_active_stage, parse_checklist};

#[test]
/// Planned suites and future cases must not advance the active suite stage.
fn planned_suites_do_not_advance_stage() {
    let manifest = "[[suite]]\nactive_from_stage = 3\nstatus = \"required\"\n[[case]]\nactive_from_stage = 4\n[[suite]]\nactive_from_stage = 8\nstatus = \"planned\"\n";
    assert_eq!(parse_active_stage(manifest), Ok(3));
}

#[test]
/// Missing stage metadata for a required suite must fail closed.
fn required_suite_needs_stage() {
    assert!(parse_active_stage("[[suite]]\nstatus = \"required\"\n").is_err());
}

#[test]
/// The next unchecked item comes from source order, not a duplicated ledger.
fn checklist_reports_first_open_task() {
    let progress =
        parse_checklist("## Stage 2\n- [x] done\n### Step A\n- [ ] first\n- [ ] second\n");
    assert_eq!(progress.done, 1);
    assert_eq!(progress.remaining, 2);
    assert_eq!(progress.next_heading.as_deref(), Some("Step A"));
    assert_eq!(progress.next_task.as_deref(), Some("first"));
}
