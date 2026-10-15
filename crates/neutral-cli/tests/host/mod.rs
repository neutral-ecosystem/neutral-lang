// SPDX-License-Identifier: Apache-2.0

//! Direct host-adapter boundary tests for the CLI package.

use super::{CliFailure, ExitClass, atomic_write, create_temporary, read_bounded, required_option};
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

    let directory = temporary_path("commands");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("command directory should exist");
    let source = directory.join("source.neu");
    let compiled = directory.join("compiled.neuir");
    let formatted = directory.join("formatted.neu");
    fs::write(&source, b"neu \"0.1\"\nmodule minimal\nnum answer = 42\n")
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
    assert_eq!(
        fs::read(&formatted).expect("formatted output should exist"),
        b"neu \"0.1\"\nmodule minimal\n\nnum answer = 42\n"
    );
    fs::remove_dir_all(directory).expect("command directory should be removable");
}

#[test]
/// Captures a project with an optional key and explicitly locked vocabulary input.
fn host_project_capture_accepts_optional_project_facts() {
    use crate::{command::ParseOutcome, constants};
    use neutral_core::VocabularyContentDigest;
    use neutral_vocabulary::{
        PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION,
    };

    let directory = temporary_path("project-capture");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("capture directory should exist");
    let source = directory.join("source.neu");
    let bundle = directory.join("bundle.json");
    let bytes = format!(
        "{{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"{PROJECT_VOCABULARY_ENCODING_VERSION}\",\"schema_version\":\"{PROJECT_VOCABULARY_SCHEMA_VERSION}\",\"identity\":\"Fixture\",\"version\":\"1.0.0\",\"required_features\":[],\"types\":[]}}"
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
    ];
    let ParseOutcome::Execute(options) =
        crate::command::parse(arguments).expect("capture options should parse")
    else {
        panic!("capture command should execute");
    };
    super::capture_project_command(&options).expect("complete project should capture");
    fs::remove_dir_all(directory).expect("capture directory should be removable");
}

#[test]
/// Maps diagnostic and diagnostic-free compilation failures to safe CLI messages.
fn host_compilation_failures_keep_safe_classification() {
    let failure = match super::compile(super::CompilationRequest::new(
        b"invalid source".to_vec(),
        neutral_core::StructuralLimits::new(1024, 8).unwrap(),
        neutral_core::CancellationToken::new(),
    ))
    .expect("source capture should succeed")
    {
        super::CompilationResult::Failure(failure) => failure,
        super::CompilationResult::Success(_) => panic!("invalid source should fail"),
    };
    let mapped = super::compilation_failure(&failure);
    assert_eq!(mapped.class(), ExitClass::Validation);
    assert!(!mapped.messages().is_empty());
    assert_eq!(
        super::successful_document(super::CompilationResult::Failure(failure.clone())).unwrap_err(),
        mapped
    );
    assert_eq!(
        super::ensure_success(super::CompilationResult::Failure(failure.clone())).unwrap_err(),
        mapped
    );
    assert_eq!(
        super::format_failure(super::FormatError::Compilation(failure)),
        mapped
    );

    let cancellation = neutral_core::CancellationToken::new();
    let captured = neutral_compiler::capture(super::CompilationRequest::new(
        b"neu \"0.1\"\nmodule minimal\nnum answer = 42\n".to_vec(),
        neutral_core::StructuralLimits::new(1024, 8).unwrap(),
        cancellation.clone(),
    ))
    .expect("source should capture before cancellation");
    cancellation.cancel();
    let super::CompilationResult::Failure(cancelled) =
        neutral_compiler::compile_captured(&captured)
    else {
        panic!("cancelled compilation should fail");
    };
    assert_eq!(
        super::compilation_failure(&cancelled).messages(),
        ["cancelled"]
    );
}

/// Every host failure adapter preserves its stable class without leaking host details.
#[test]
fn capture_failure_classes_and_safe_messages_are_complete() {
    use neutral_compiler::{CaptureError, ProjectCaptureError, ProjectHostError};
    use neutral_core::SourceContentDigest;
    for (error, class, message) in [
        (CaptureError::Cancelled, ExitClass::Cancelled, "cancelled"),
        (
            CaptureError::MissingSource,
            ExitClass::Validation,
            "missing-source",
        ),
        (
            CaptureError::SourceLimitExceeded {
                actual: 2,
                limit: 1,
            },
            ExitClass::Validation,
            "source-limit-exceeded",
        ),
        (
            CaptureError::SourceDigestMismatch {
                expected: SourceContentDigest::from_bytes(b"expected"),
                actual: SourceContentDigest::from_bytes(b"actual"),
            },
            ExitClass::Validation,
            "source-digest-mismatch",
        ),
    ] {
        let failure = super::capture_failure(&error);
        assert_eq!(failure.class(), class);
        assert_eq!(failure.messages(), [message]);
        assert_eq!(
            super::format_failure(super::FormatError::Capture(error)),
            failure
        );
    }
    let error = ProjectHostError::ConflictingSourceMapping;
    assert_eq!(
        super::project_host_failure(error).messages(),
        [error.code()]
    );
    for error in [
        ProjectCaptureError::Cancelled,
        ProjectCaptureError::InvalidRequest,
    ] {
        let failure = super::project_capture_failure(error);
        assert_eq!(failure.messages(), [error.code()]);
        assert_eq!(
            failure.class(),
            if error == ProjectCaptureError::Cancelled {
                ExitClass::Cancelled
            } else {
                ExitClass::Validation
            }
        );
    }
}

/// Failed output creation or publication cannot replace a directory or leak temporary files.
#[test]
fn atomic_output_failures_cleanup_uncommitted_files() {
    let directory = temporary_path("failed-publication");
    fs::create_dir_all(&directory).unwrap();
    let destination = directory.join("existing-directory");
    fs::create_dir(&destination).unwrap();
    assert_eq!(
        atomic_write(&destination, b"new", true)
            .unwrap_err()
            .messages(),
        ["output-commit-failed"]
    );
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    let missing = directory.join("missing").join("artifact.nir");
    assert_eq!(
        atomic_write(&missing, b"new", false)
            .unwrap_err()
            .messages(),
        ["output-create-failed"]
    );
    assert!(destination.is_dir());
    fs::remove_dir_all(directory).unwrap();
}

/// Captured project vocabulary options retain exact lock facts and reject malformed digests.
#[test]
fn project_vocabulary_lock_acquisition_is_explicit() {
    let path = temporary_path("vocabulary.json");
    let bytes = b"captured bytes";
    fs::write(&path, bytes).unwrap();
    let mut options = super::VocabularyOptions {
        bundle: Some(path.to_string_lossy().into_owned()),
        identity: Some("Fixture".to_owned()),
        version: Some("1.0.0".to_owned()),
        encoding_version: Some(neutral_vocabulary::PROJECT_VOCABULARY_ENCODING_VERSION.to_owned()),
        schema_version: Some(neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION.to_owned()),
        digest: Some(neutral_core::VocabularyContentDigest::from_bytes(bytes).to_string()),
        features: Vec::new(),
    };
    assert!(super::project_vocabulary(&options, 64).unwrap().is_some());
    let request = || {
        super::CompilationRequest::new(
            Vec::new(),
            neutral_core::StructuralLimits::new(64, 4).unwrap(),
            neutral_core::CancellationToken::new(),
        )
    };
    assert!(super::captured_vocabulary(request(), &options, 64).is_ok());
    options.digest = Some("invalid".to_owned());
    assert_eq!(
        super::captured_vocabulary(request(), &options, 64)
            .unwrap_err()
            .messages(),
        ["invalid-vocabulary-digest"]
    );
    assert_eq!(
        super::project_vocabulary(&options, 64)
            .unwrap_err()
            .messages(),
        ["invalid-vocabulary-digest"]
    );
    options.digest = Some(neutral_core::VocabularyContentDigest::from_bytes(bytes).to_string());
    options.identity = Some("invalid identity".to_owned());
    assert_eq!(
        super::captured_vocabulary(request(), &options, 64)
            .unwrap_err()
            .messages(),
        ["invalid-vocabulary-lock"]
    );
    assert_eq!(
        super::project_vocabulary(&options, 64)
            .unwrap_err()
            .messages(),
        ["invalid-vocabulary-lock"]
    );
    fs::remove_file(path).unwrap();
}

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

/// Reader that deterministically injects a host I/O fault.
struct FailingReader;

impl std::io::Read for FailingReader {
    /// Fails every input operation without consuming bytes.
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("injected input failure"))
    }
}

#[test]
/// Host input faults and formatter failures retain their public classifications.
fn host_faults_do_not_become_partial_success() {
    assert_eq!(
        read_bounded(Box::new(FailingReader), 16)
            .unwrap_err()
            .class(),
        ExitClass::Input
    );
    for (error, expected) in [
        (
            super::FormatError::OutputLimitExceeded,
            ExitClass::Validation,
        ),
        (super::FormatError::InternalDefect, ExitClass::Internal),
    ] {
        assert_eq!(super::format_failure(error).class(), expected);
    }
}
