// SPDX-License-Identifier: Apache-2.0

//! Crate-local tests for the complete in-process CLI host adapter.

use super::{constants, execute};
use neutral_core::CancellationToken;
use neutral_encoding::{DecodeLimits, decode};
use neutral_ir::{ExactNumber, LogicalValue};
use neutral_test_support::TestDirectory;
use std::fs;

/// Valid source used across in-process host commands.
const SOURCE: &[u8] = b"neu \"0.1\"\nmodule library_test\nnum answer = 42\n";
/// Canonical rendering independently expected for the fixture source.
const FORMATTED: &[u8] = b"neu \"0.1\"\nmodule library_test\n\nnum answer = 42\n";

#[test]
/// Validation accepts valid source and classifies invalid source without producing files.
fn cli_library_validates_source() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    fs::write(&source, SOURCE).unwrap();
    let arguments = [
        constants::VALIDATE.to_owned(),
        source.to_string_lossy().into_owned(),
    ];
    execute(arguments.clone()).expect("valid source should validate");
    fs::write(&source, b"invalid source").unwrap();
    let failure = execute(arguments).expect_err("invalid source must fail validation");
    assert_eq!(failure.class().code(), constants::EXIT_VALIDATION);
    assert_ne!(failure.messages(), &[] as &[String]);
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 1);
}

#[test]
/// Formatting writes exact canonical bytes and preserves the captured source.
fn cli_library_formats_canonical_source() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    let formatted = fixture.path().join("formatted.neu");
    fs::write(&source, SOURCE).unwrap();
    execute([
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should format");
    assert_eq!(fs::read(formatted).unwrap(), FORMATTED);
    assert_eq!(fs::read(source).unwrap(), SOURCE);
}

#[test]
/// Compilation publishes an independently readable artifact with the expected declaration value.
fn cli_library_compiles_readable_artifact() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    let artifact = fixture.path().join("artifact.neuir");
    fs::write(&source, SOURCE).unwrap();
    execute([
        constants::COMPILE.to_owned(),
        constants::OUTPUT.to_owned(),
        artifact.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("valid source should compile");
    let bytes = fs::read(artifact).unwrap();
    let reader = decode(&bytes, DecodeLimits::hard(), &CancellationToken::new()).unwrap();
    assert_eq!(reader.declarations().len(), 1);
    assert_eq!(
        reader
            .declaration_by_name("answer")
            .expect("compiled answer")
            .value(),
        &LogicalValue::Number(ExactNumber::from_source("42", 64, 64).unwrap())
    );
    assert_eq!(fs::read(source).unwrap(), SOURCE);
}

#[test]
/// Output protection preserves existing bytes until explicit overwrite replaces them completely.
fn cli_library_requires_explicit_overwrite() {
    let fixture = TestDirectory::new().unwrap();
    let source = fixture.path().join("source.neu");
    let formatted = fixture.path().join("formatted.neu");
    fs::write(&source, SOURCE).unwrap();
    fs::write(&formatted, b"existing output to preserve").unwrap();
    let arguments = [
        constants::FORMAT.to_owned(),
        constants::OUTPUT.to_owned(),
        formatted.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ];
    let failure = execute(arguments.clone()).expect_err("existing output requires overwrite");
    assert_eq!(failure.class().code(), constants::EXIT_OUTPUT);
    assert_eq!(failure.messages(), ["output-exists"]);
    assert_eq!(
        fs::read(&formatted).unwrap(),
        b"existing output to preserve"
    );
    execute(
        arguments
            .into_iter()
            .chain([constants::OVERWRITE.to_owned()]),
    )
    .expect("explicit overwrite should replace output");
    assert_eq!(fs::read(formatted).unwrap(), FORMATTED);
    assert_eq!(fs::read(source).unwrap(), SOURCE);
    assert_eq!(fs::read_dir(fixture.path()).unwrap().count(), 2);
}
