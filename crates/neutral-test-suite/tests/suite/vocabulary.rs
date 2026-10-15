// SPDX-License-Identifier: Apache-2.0

//! vocabulary cross-package regressions.

use super::*;

#[test]
/// Verifies qualified vocabulary data, defaults, derivation, reader, and probe end to end.
fn system_captured_vocabulary_crosses_reader_and_probe() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("exact captured vocabulary fixture should compile");
    };
    let document = ValidatedDocument::from_compiler_output(artifacts)
        .expect("captured vocabulary artifacts should pass reader validation");
    let vocabulary = document
        .vocabulary()
        .expect("reader should expose exact vocabulary");
    assert_eq!(
        vocabulary.identity().identity(),
        FIXTURE_VOCABULARY_IDENTITY
    );
    assert_eq!(vocabulary.identity().version(), FIXTURE_VOCABULARY_VERSION);
    assert_eq!(
        vocabulary.identity().schema_version(),
        VOCABULARY_SCHEMA_VERSION
    );
    assert_eq!(
        vocabulary.identity().encoding_version(),
        VOCABULARY_ENCODING_VERSION
    );
    assert_eq!(
        vocabulary.identity().content_digest(),
        VocabularyContentDigest::from_bytes(VOCABULARY_BUNDLE)
    );
    let metadata = document
        .declaration_by_name("metadata")
        .expect("metadata should exist");
    assert_eq!(metadata.resolved_type().to_string(), "Fixture::Metadata");
    let defaults = document
        .artifacts()
        .field_provenance()
        .iter()
        .filter(|record| record.origin() == ValueOrigin::VocabularyDefault)
        .count();
    assert_eq!(defaults, 5);
    assert_eq!(
        document.artifacts().derivation().vocabulary(),
        Some(vocabulary.identity())
    );
    let summary = summarize(&document);
    let expected_summary_prefix =
        format!("{FIXTURE_VOCABULARY_IDENTITY}@{FIXTURE_VOCABULARY_VERSION}");
    assert!(
        summary
            .vocabulary()
            .is_some_and(|value| value.starts_with(&expected_summary_prefix))
    );
    assert!(
        summary
            .vocabulary_types()
            .iter()
            .any(|value| value.starts_with("record Fixture::Metadata"))
    );
}

#[test]
/// Verifies the frozen minimal source and exact bundle match their accepted oracle.
fn conformance_minimal_vocabulary_oracle() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("minimal vocabulary oracle should accept");
    };
    assert_eq!(
        artifacts
            .logical_document()
            .vocabulary()
            .expect("oracle requires vocabulary")
            .identity()
            .content_digest(),
        VocabularyContentDigest::from_bytes(VOCABULARY_BUNDLE),
    );
}

#[test]
/// Verifies missing capture and unknown qualified types fail with frozen diagnostics.
fn conformance_vocabulary_resolution_failures() {
    let missing = compile_failure(MISSING_VOCABULARY_CAPTURE);
    assert_eq!(missing.class(), ResultClass::Vocabulary);
    assert_eq!(
        missing.diagnostics()[0].code().as_str(),
        diagnostics::MISSING_VOCABULARY
    );
    let CompilationResult::Failure(unknown) =
        compile_with_vocabulary(UNKNOWN_VOCABULARY_TYPE, VOCABULARY_BUNDLE)
    else {
        panic!("unknown qualified type must fail");
    };
    assert_eq!(
        unknown.diagnostics()[0].code().as_str(),
        diagnostics::UNKNOWN_VOCABULARY_TYPE
    );
}

#[test]
/// Verifies vocabulary payload fields use distinct closed-schema diagnostics.
fn conformance_vocabulary_payload_failures() {
    for (source, code) in [
        (
            UNKNOWN_VOCABULARY_FIELD,
            diagnostics::UNKNOWN_VOCABULARY_FIELD,
        ),
        (
            WRONG_VOCABULARY_FIELD_TYPE,
            diagnostics::VOCABULARY_FIELD_TYPE_MISMATCH,
        ),
        (
            MISSING_VOCABULARY_FIELD,
            diagnostics::MISSING_VOCABULARY_FIELD,
        ),
        (
            DUPLICATE_VOCABULARY_FIELD,
            diagnostics::DUPLICATE_VOCABULARY_FIELD,
        ),
        (
            VOCABULARY_NAME_COLLISION,
            diagnostics::VOCABULARY_NAME_COLLISION,
        ),
    ] {
        let CompilationResult::Failure(failure) =
            compile_with_vocabulary(source, VOCABULARY_BUNDLE)
        else {
            panic!("invalid vocabulary payload must fail");
        };
        assert_eq!(failure.class(), ResultClass::Vocabulary);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
    }
}

#[test]
/// Verifies captured executable shapes fail before source payload validation.
fn security_vocabulary_executable_shape_precedes_payloads() {
    let CompilationResult::Failure(failure) =
        compile_with_vocabulary(WRONG_VOCABULARY_FIELD_TYPE, EXECUTABLE_VOCABULARY_BUNDLE)
    else {
        panic!("executable vocabulary content must fail");
    };
    assert_eq!(failure.class(), ResultClass::Vocabulary);
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::EXECUTABLE_VOCABULARY_MEMBER
    );
}

#[test]
/// Verifies an unknown locked structural feature fails with its stable class.
fn security_vocabulary_unknown_feature_fails_closed() {
    let lock = VocabularyLock::new(
        FIXTURE_VOCABULARY_IDENTITY,
        FIXTURE_VOCABULARY_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(UNKNOWN_FEATURE_VOCABULARY_BUNDLE),
        Vec::new(),
    )
    .expect("unknown feature lock should be structurally valid");
    let request = CompilationRequest::new(
        MINIMAL_VOCABULARY.to_vec(),
        StructuralLimits::new(16_384, 16).expect("vocabulary limits should be valid"),
        CancellationToken::new(),
    )
    .with_captured_vocabulary(UNKNOWN_FEATURE_VOCABULARY_BUNDLE.to_vec(), lock);
    let CompilationResult::Failure(failure) = compile(request).expect("source should capture")
    else {
        panic!("unknown vocabulary feature must fail");
    };
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::UNKNOWN_VOCABULARY_FEATURE
    );
}

#[test]
/// Verifies source vocabulary requirements never perform implicit acquisition.
fn property_vocabulary_use_requires_identical_host_capture() {
    let first = compile_failure(MISSING_VOCABULARY_CAPTURE);
    let second = compile_failure(MISSING_VOCABULARY_CAPTURE);
    assert_eq!(first, second);
    assert_eq!(first.detail(), CompilationFailureDetail::VocabularyRejected);
}

#[test]
/// Verifies JSON member order changes captured facts but not logical meaning.
fn property_vocabulary_bundle_formatting_is_nonsemantic() {
    let CompilationResult::Success(canonical) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("canonical bundle should compile");
    };
    let CompilationResult::Success(reordered) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, REORDERED_VOCABULARY_BUNDLE)
    else {
        panic!("reordered bundle should compile");
    };
    assert!(canonical.logically_equivalent(&reordered));
    assert_ne!(
        canonical.derivation().vocabulary(),
        reordered.derivation().vocabulary()
    );
}

#[test]
/// Verifies repeated exact vocabulary compilation is deterministic.
fn property_qualified_vocabulary_compilation_is_deterministic() {
    let first = compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE);
    let second = compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE);
    assert_eq!(first, second);
}

#[test]
/// Verifies omitted vocabulary fields become ordinary final values with companion provenance.
fn property_vocabulary_defaults_remain_ordinary_values() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("minimal vocabulary should compile");
    };
    let metadata = artifacts
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "metadata")
        .expect("metadata declaration should exist");
    let neutral_ir::LogicalValue::VocabularyRecord(value) = metadata.value() else {
        panic!("metadata should retain its qualified contextual type");
    };
    assert_eq!(value.fields().len(), 6);
    assert_eq!(
        artifacts
            .field_provenance()
            .iter()
            .filter(|record| record.origin() == ValueOrigin::VocabularyDefault)
            .count(),
        5
    );
}

#[test]
/// Verifies an exact lock digest mismatch fails before source parsing.
fn security_vocabulary_lock_mismatch_precedes_source_parsing() {
    let lock = VocabularyLock::new(
        FIXTURE_VOCABULARY_IDENTITY,
        FIXTURE_VOCABULARY_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(b"different"),
        Vec::new(),
    )
    .expect("mismatched lock should still be structurally valid");
    let request = CompilationRequest::new(
        b"not Neutral source".to_vec(),
        StructuralLimits::new(16_384, 16).expect("vocabulary limits should be valid"),
        CancellationToken::new(),
    )
    .with_captured_vocabulary(VOCABULARY_BUNDLE.to_vec(), lock);
    let CompilationResult::Failure(failure) = compile(request).expect("source should capture")
    else {
        panic!("lock mismatch must fail");
    };
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::VOCABULARY_LOCK_MISMATCH
    );
}

#[test]
/// Verifies the reader fails closed when qualified data loses its exact contract.
fn security_reader_rejects_missing_vocabulary_contract() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("exact captured vocabulary fixture should compile");
    };
    let logical = LogicalDocument::with_record_types(
        artifacts.logical_document().module().clone(),
        artifacts.logical_document().record_types().to_vec(),
        artifacts.logical_document().declarations().to_vec(),
    );
    let hostile = Arc::new(replace_logical_document(&artifacts, logical));
    assert_eq!(
        ValidatedDocument::from_compiler_output(hostile).unwrap_err(),
        ReaderError::InvalidDerivation,
    );
}

/// Vocabulary record paths retain exact cyclic identity targets through decoding.
#[test]
fn vocabulary_reference_paths_survive_independent_reader_roundtrip() {
    let source = b"neu \"0.1\"\nmodule vocabulary_edges\nuse Fixture\nFixture::Node first = { next: ref(second), }\nFixture::Node second = { next: ref(first), }\n";
    let artifacts = match compile_with_vocabulary(source, VOCABULARY_BUNDLE) {
        CompilationResult::Success(artifacts) => artifacts,
        CompilationResult::Failure(failure) => {
            panic!("typed vocabulary references: {:?}", failure.diagnostics())
        }
    };
    let reader = ValidatedDocument::from_compiler_output(artifacts).unwrap();
    assert_eq!(reader.artifacts().reference_provenance().len(), 2);
    let encoded = encode(
        &reader,
        &ProducerInfo::new("reference-test", TEST_PRODUCER_VERSION),
    )
    .unwrap();
    let decoded = decode(
        encoded.as_bytes(),
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(decoded.declarations(), reader.declarations());
}
