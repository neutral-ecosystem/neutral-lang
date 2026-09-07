// SPDX-License-Identifier: Apache-2.0

//! Crate-local tests for the complete in-process CLI host adapter.

use super::{constants, execute};
use std::{fs, path::PathBuf};

/// Valid source used across in-process host commands.
const SOURCE: &[u8] = b"neu \"0.1\"\nmodule library_test\nnum answer = 42\n";

/// Returns a process-unique temporary path for one test artifact.
fn temporary_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("neutral-cli-library-{}-{name}", std::process::id()))
}

#[test]
/// Exercises validation, formatting, compilation, overwrite, and failure classes.
fn cli_library_executes_complete_host_paths() {
    let source = temporary_path("source.neu");
    let formatted = temporary_path("formatted.neu");
    let artifact = temporary_path("artifact.neuir");
    fs::write(&source, SOURCE).expect("source should be writable");

    execute([
        constants::VALIDATE.to_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should validate");
    execute([
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should format");
    execute([
        constants::COMPILE.to_owned(),
        constants::OUTPUT.to_owned(),
        artifact.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should compile");
    let failure = execute([
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect_err("existing output should fail without overwrite");
    assert_eq!(failure.class().code(), constants::EXIT_OUTPUT);
    assert!(!failure.messages().is_empty());
    execute([
        constants::FORMAT.to_owned(),
        constants::OVERWRITE.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("overwrite should atomically replace output");

    fs::remove_file(source).expect("source should be removable");
    fs::remove_file(formatted).expect("formatted output should be removable");
    fs::remove_file(artifact).expect("artifact should be removable");
}
