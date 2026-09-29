// SPDX-License-Identifier: Apache-2.0

//! Pure suite-activation parser tests.

use super::parse_active_stage;

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
