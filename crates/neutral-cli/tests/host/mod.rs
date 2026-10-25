// SPDX-License-Identifier: Apache-2.0

//! Direct host-adapter boundary tests for the CLI package.

use super::{
    CliFailure, ExitClass, atomic_write, create_temporary, read_bounded, required_option,
    stdout_write_failure, write_selected_output,
};
use neutral_test_support::TestDirectory;
use std::{fs, io::Cursor, path::PathBuf};

#[test]
/// Executes side-effect-free commands and validates all three host command adapters.
fn host_executes_profiles_validate_compile_and_format() {
    use crate::constants;

    super::execute([constants::HELP.to_owned()]).expect("help should succeed");
    super::execute([constants::VERSION.to_owned()]).expect("version should succeed");
    super::execute([constants::PROFILES.to_owned()]).expect("profiles should succeed");
    assert_eq!(
        super::execute(["unknown".to_owned()]).unwrap_err().class(),
        ExitClass::Usage
    );

    let fixture = TestDirectory::new().unwrap();
    let directory = fixture.path();
    let source = directory.join("source.neu");
    let compiled = directory.join("compiled.neuir");
    let formatted = directory.join("formatted.neu");
    fs::write(&source, b"neu \"1.0\"\nmodule minimal\nnum answer = 42\n")
        .expect("source should be written");
    let run = |command: &str, output: Option<&PathBuf>| {
        let mut arguments = vec![command.to_owned(), source.to_string_lossy().into_owned()];
        if let Some(output) = output {
            arguments.extend([
                constants::OUTPUT.to_owned(),
                output.to_string_lossy().into_owned(),
            ]);
        }
        super::execute(arguments).expect("valid host command should succeed");
    };
    run(constants::VALIDATE, None);
    run(constants::COMPILE, Some(&compiled));
    run(constants::FORMAT, Some(&formatted));
    assert!(fs::metadata(&compiled).unwrap().len() > 0);
    let formatted_bytes = fs::read(&formatted).expect("formatted output should exist");
    let formatted_source = std::str::from_utf8(&formatted_bytes).unwrap();
    assert!(formatted_source.starts_with("neu \"1.0\"\nmodule minimal\n"));
    assert!(formatted_source.contains("num answer = 42"));
    assert_eq!(
        fs::read(source).unwrap(),
        b"neu \"1.0\"\nmodule minimal\nnum answer = 42\n"
    );
}

#[test]
/// Parse, validation, compile, and format failures retain stable host outcomes.
fn host_command_failures_keep_stable_classification() {
    use crate::constants;

    assert_eq!(
        super::execute(["--unknown".to_owned(), "value".to_owned()])
            .unwrap_err()
            .class(),
        ExitClass::Usage
    );
    assert_eq!(
        super::execute([
            "--unknown".to_owned(),
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
        ])
        .unwrap_err()
        .class(),
        ExitClass::Usage
    );
    assert_eq!(
        super::execute([
            "--unknown".to_owned(),
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "d".to_owned(),
        ])
        .unwrap_err()
        .class(),
        ExitClass::Usage
    );
    assert_eq!(
        super::execute(vec!["--unknown".to_owned(), "value".to_owned()])
            .unwrap_err()
            .class(),
        ExitClass::Usage
    );

    let fixture = TestDirectory::new().unwrap();
    let directory = fixture.path();
    let source = directory.join("invalid.neu");
    fs::write(&source, b"invalid source").expect("invalid source should be written");
    for (command, output) in [
        (constants::VALIDATE, None),
        (constants::COMPILE, Some(directory.join("compiled.neuir"))),
        (constants::FORMAT, Some(directory.join("formatted.neu"))),
    ] {
        let mut arguments = vec![command.to_owned(), source.to_string_lossy().into_owned()];
        if let Some(output) = output {
            arguments.extend([
                constants::OUTPUT.to_owned(),
                output.to_string_lossy().into_owned(),
            ]);
        }
        assert_eq!(
            super::execute(arguments).unwrap_err().class(),
            ExitClass::Validation
        );
    }
}

#[test]
/// Cancellation requested before compilation maps to the cancelled exit class.
fn host_cancelled_commands_keep_stable_classification() {
    use crate::constants;

    for command in [constants::VALIDATE, constants::COMPILE] {
        let fixture = TestDirectory::new().unwrap();
        let source = fixture.path().join("cancelled-source");
        fs::write(&source, b"neu \"1.0\"\nmodule minimal\n")
            .expect("valid source should be written");
        let mut arguments = vec![command.to_owned(), source.to_string_lossy().into_owned()];
        arguments.push(constants::CANCEL_BEFORE_START.to_owned());
        if command == constants::COMPILE {
            arguments.extend([
                constants::OUTPUT.to_owned(),
                fixture
                    .path()
                    .join("cancelled-output")
                    .to_string_lossy()
                    .into_owned(),
            ]);
        }
        assert_eq!(
            super::execute(arguments).unwrap_err().class(),
            ExitClass::Cancelled
        );
    }
}

#[test]
/// The encoder boundary rejects a source string above its independent text limit.
fn host_maps_artifact_encoding_limit() {
    use crate::constants;

    let fixture = TestDirectory::new().unwrap();
    let directory = fixture.path();
    let source = directory.join("large.neu");
    let output = directory.join("large.neuir");
    let text_limit = neutral_encoding::constants::MAXIMUM_TEXT_BYTES;
    let source_bytes = format!(
        "neu \"1.0\"\nmodule minimal\nstring payload = \"{}\"\n",
        "x".repeat(text_limit + 1)
    );
    fs::write(&source, source_bytes.as_bytes()).expect("large source should be written");
    let limit = (source_bytes.len() as u64 + 1).to_string();
    let failure = super::execute(vec![
        constants::COMPILE.to_owned(),
        source.to_string_lossy().into_owned(),
        constants::OUTPUT.to_owned(),
        output.to_string_lossy().into_owned(),
        constants::MAX_SOURCE_BYTES.to_owned(),
        limit.clone(),
        constants::MAX_STRING_BYTES.to_owned(),
        limit,
    ])
    .unwrap_err();
    assert_eq!(failure.class(), ExitClass::Validation);
    assert_eq!(failure.messages(), ["NEU-EXP-009"]);
}

#[test]
/// Captures a project with an optional key and explicitly locked vocabulary input.
fn host_project_capture_accepts_optional_project_facts() {
    use crate::{command::ParseOutcome, constants};
    use neutral_core::VocabularyContentDigest;
    use neutral_vocabulary::{
        PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION,
    };

    let fixture = TestDirectory::new().unwrap();
    let directory = fixture.path();
    let source = directory.join("source.neu");
    let bundle = directory.join("bundle.json");
    let bytes = format!(
        "{{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"{PROJECT_VOCABULARY_ENCODING_VERSION}\",\"schema_version\":\"{PROJECT_VOCABULARY_SCHEMA_VERSION}\",\"identity\":\"Fixture\",\"version\":\"1.0.0\",\"required_features\":[\"vocabulary-composition-v2\"],\"types\":[]}}"
    );
    fs::write(
        &source,
        b"neu \"1.0\"\nmodule example\nuse Fixture as fixture\n",
    )
    .expect("source should be written");
    fs::write(&bundle, &bytes).expect("vocabulary bundle should be written");
    let arguments = [
        constants::CAPTURE_PROJECT.to_owned(),
        source.to_string_lossy().into_owned(),
        constants::SOURCE_ID.to_owned(),
        "source:test".to_owned(),
        constants::MODULE_ID.to_owned(),
        "example".to_owned(),
        constants::PROJECT_KEY.to_owned(),
        "project:test".to_owned(),
        constants::VOCABULARY_BUNDLE.to_owned(),
        bundle.to_string_lossy().into_owned(),
        constants::VOCABULARY_IDENTITY.to_owned(),
        "Fixture".to_owned(),
        constants::VOCABULARY_VERSION.to_owned(),
        "1.0.0".to_owned(),
        constants::VOCABULARY_ENCODING.to_owned(),
        PROJECT_VOCABULARY_ENCODING_VERSION.to_owned(),
        constants::VOCABULARY_SCHEMA.to_owned(),
        PROJECT_VOCABULARY_SCHEMA_VERSION.to_owned(),
        constants::VOCABULARY_DIGEST.to_owned(),
        VocabularyContentDigest::from_bytes(bytes.as_bytes()).to_string(),
        constants::VOCABULARY_FEATURE.to_owned(),
        "vocabulary-composition-v2".to_owned(),
    ];
    let ParseOutcome::Execute(options) =
        crate::command::parse(arguments).expect("capture options should parse")
    else {
        panic!("capture command should execute");
    };
    super::capture_project_command(&options).expect("complete project should capture");
}

#[test]
fn atomic_output_failures_cleanup_uncommitted_files() {
    let fixture = TestDirectory::new().unwrap();
    let directory = fixture.path();
    let destination = directory.join("existing-directory");
    fs::create_dir(&destination).unwrap();
    assert_eq!(
        atomic_write(&destination, b"new", true)
            .unwrap_err()
            .messages(),
        ["output-commit-failed"]
    );
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
    let missing = directory.join("missing").join("artifact.nir");
    assert_eq!(
        atomic_write(&missing, b"new", false)
            .unwrap_err()
            .messages(),
        ["output-create-failed"]
    );
    assert!(destination.is_dir());

    assert_eq!(
        atomic_write(std::path::Path::new(""), b"new", true)
            .unwrap_err()
            .messages(),
        ["output-commit-failed"]
    );
    assert_eq!(
        super::write_temporary(Err(std::io::Error::other("injected write failure")))
            .unwrap_err()
            .messages(),
        ["output-write-failed"]
    );
}

#[test]
/// Verifies private input and required-option helpers preserve stable classes.
fn host_input_helpers_enforce_declared_bounds() {
    let limited = read_bounded(Box::new(Cursor::new(b"123".to_vec())), 2)
        .expect_err("one byte over the selected limit must fail");
    assert_eq!(limited.class(), ExitClass::Validation);
    let missing = required_option(None).expect_err("a missing validated option must fail");
    assert_eq!(missing.class(), ExitClass::Internal);
    let fixture = TestDirectory::new().unwrap();
    let missing_input =
        super::read_selected_input(&fixture.path().join("missing-input").to_string_lossy(), 16)
            .expect_err("a missing selected file must fail");
    assert_eq!(missing_input.class(), ExitClass::Input);
}

#[test]
/// Internal missing-output state is reported without attempting a write.
fn host_missing_output_is_an_internal_failure() {
    use crate::{command::ParseOutcome, constants};

    let ParseOutcome::Execute(options) = crate::command::parse([
        constants::COMPILE.to_owned(),
        "source.neu".to_owned(),
        constants::OUTPUT.to_owned(),
        "artifact.neuir".to_owned(),
    ])
    .expect("complete compile command should parse") else {
        panic!("compile command should execute");
    };
    let mut options = *options;
    options.output = None;
    assert_eq!(
        super::write_selected_output(&options, b"artifact")
            .unwrap_err()
            .messages(),
        ["missing-validated-output"]
    );
}

#[test]
/// Exercises the standard-output write and flush path without emitting bytes.
fn host_writes_selected_standard_output() {
    use crate::{command::ParseOutcome, constants};

    let ParseOutcome::Execute(options) = crate::command::parse([
        constants::COMPILE.to_owned(),
        constants::STANDARD_STREAM.to_owned(),
        constants::OUTPUT.to_owned(),
        constants::STANDARD_STREAM.to_owned(),
    ])
    .expect("standard-stream command should parse") else {
        panic!("command should execute");
    };
    write_selected_output(&options, b"").expect("empty standard output should succeed");
    let failure = stdout_write_failure(std::io::Error::other("injected write failure"));
    assert_eq!(failure.class(), ExitClass::Output);
    assert_eq!(failure.messages(), ["stdout-write-failed"]);
}

#[test]
/// Verifies temporary creation and atomic publication retain no abandoned file.
fn host_temporary_and_atomic_output_paths_are_safe() {
    let fixture = TestDirectory::new().unwrap();
    let directory = fixture.path();
    let (temporary, file) = create_temporary(directory).expect("temporary file should open");
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
}

#[test]
/// Verifies failures retain only the stable public exit classification.
fn host_failures_expose_stable_classification() {
    let failure = CliFailure::one(ExitClass::Input, "input-read-failed");
    assert_eq!(failure.class(), ExitClass::Input);
    assert_eq!(failure.messages(), ["input-read-failed"]);
}

/// Reader that deterministically injects a host I/O fault.
struct FailingReader;

impl std::io::Read for FailingReader {
    /// Fails every input operation without consuming bytes.
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("injected input failure"))
    }
}

#[test]
/// Host input faults retain their public classification without partial success.
fn host_faults_do_not_become_partial_success() {
    assert_eq!(
        read_bounded(Box::new(FailingReader), 16)
            .unwrap_err()
            .class(),
        ExitClass::Input
    );
}
