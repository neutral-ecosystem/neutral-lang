// SPDX-License-Identifier: Apache-2.0

//! Tests the standalone probe command boundary.

use super::run;

#[test]
/// Verifies that the probe accepts its help flag.
fn package_shell_probe_accepts_help() {
    assert!(run(["--help".to_owned()]).is_ok());
}

#[test]
/// Verifies that the probe rejects a missing artifact.
fn package_shell_probe_rejects_empty_arguments() {
    assert!(run(Vec::new()).is_err());
}

#[test]
/// Verifies that the probe reports an unreadable artifact without panicking.
fn package_shell_probe_rejects_an_unreadable_artifact() {
    assert!(run(["definitely-missing-neutral-artifact.nir".to_owned()]).is_err());
}
