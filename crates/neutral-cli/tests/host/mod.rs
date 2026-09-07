// SPDX-License-Identifier: Apache-2.0

//! Direct host-adapter boundary tests for the CLI package.

use super::{CliFailure, ExitClass, atomic_write, create_temporary, read_bounded, required_option};
use std::{fs, io::Cursor, path::PathBuf};

/// Returns a process-unique temporary path inside the system temporary directory.
fn temporary_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("neutral-cli-host-{}-{name}", std::process::id()))
}

#[test]
/// Verifies private input and required-option helpers preserve stable classes.
fn host_input_helpers_enforce_declared_bounds() {
    let limited = read_bounded(Box::new(Cursor::new(b"123".to_vec())), 2)
        .expect_err("one byte over the selected limit must fail");
    assert_eq!(limited.class(), ExitClass::Validation);
    let missing = required_option(None).expect_err("a missing validated option must fail");
    assert_eq!(missing.class(), ExitClass::Internal);
}

#[test]
/// Verifies temporary creation and atomic publication retain no abandoned file.
fn host_temporary_and_atomic_output_paths_are_safe() {
    let directory = temporary_path("directory");
    fs::create_dir_all(&directory).expect("temporary directory should exist");
    let (temporary, file) = create_temporary(&directory).expect("temporary file should open");
    drop(file);
    assert!(temporary.exists());
    fs::remove_file(&temporary).expect("temporary file should be removable");

    let output = directory.join("artifact.neuir");
    atomic_write(&output, b"first", false).expect("initial output should commit");
    let existing = atomic_write(&output, b"second", false)
        .expect_err("existing output must not be overwritten implicitly");
    assert_eq!(existing.class(), ExitClass::Output);
    atomic_write(&output, b"second", true).expect("explicit overwrite should commit");
    assert_eq!(
        fs::read(&output).expect("output should remain readable"),
        b"second"
    );
    fs::remove_dir_all(&directory).expect("temporary directory should be removable");
}

#[test]
/// Verifies failures retain only the stable public exit classification.
fn host_failures_expose_stable_classification() {
    let failure = CliFailure::one(ExitClass::Input, "input-read-failed");
    assert_eq!(failure.class(), ExitClass::Input);
    assert_eq!(failure.messages(), ["input-read-failed"]);
}
