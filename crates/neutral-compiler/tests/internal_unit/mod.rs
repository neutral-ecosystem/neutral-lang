// SPDX-License-Identifier: Apache-2.0

//! Unit tests for captured-input and I/O-free compiler contracts.

use super::diagnostics;
use super::{
    CaptureError, CompilationCheckpoint, CompilationFailureDetail, CompilationRequest,
    CompilationResult, capture, compile, compile_captured, compile_captured_with_checkpoints,
    format, format_captured,
};
use neutral_core::{CancellationToken, SourceContentDigest, StructuralLimits};

/// Returns small deterministic limits for capture tests.
fn test_limits() -> StructuralLimits {
    StructuralLimits::new(64, 4).expect("test limits should be valid")
}

#[test]
/// Verifies that capture records exact source bytes and their typed digest.
fn capture_records_exact_source_identity() {
    let source = b"neu \"0.1\"\n".to_vec();
    let captured = capture(CompilationRequest::new(
        source.clone(),
        test_limits(),
        CancellationToken::new(),
    ))
    .expect("source should capture");
    assert_eq!(captured.source(), source);
    assert_eq!(
        captured.source_digest(),
        SourceContentDigest::from_bytes(&source)
    );
}

#[test]
/// Verifies that digest mismatches fail before compiler work starts.
fn capture_rejects_an_unmatched_digest() {
    let request =
        CompilationRequest::new(b"source".to_vec(), test_limits(), CancellationToken::new())
            .requiring_source_digest(SourceContentDigest::from_bytes(b"other"));
    assert!(matches!(
        capture(request),
        Err(CaptureError::SourceDigestMismatch { .. })
    ));
}

#[test]
/// Verifies that the frozen minimal source produces authoritative artifacts.
fn compile_captured_accepts_the_minimal_frontend_slice() {
    let captured = capture(CompilationRequest::new(
        b"neu \"0.1\"\nmodule minimal\n\nnum answer = 42\n".to_vec(),
        test_limits(),
        CancellationToken::new(),
    ))
    .expect("source should capture");
    let super::CompilationResult::Success(artifacts) = compile_captured(&captured) else {
        panic!("minimal source should compile successfully");
    };
    assert_eq!(
        artifacts.logical_document().module().module_name(),
        "minimal"
    );
    assert_eq!(artifacts.logical_document().declarations().len(), 1);
}

#[test]
/// Proves cancellation at every compiler handoff prevents authoritative IR publication.
fn cancellation_at_every_compiler_checkpoint_fails_closed() {
    for cancelled_at in [
        CompilationCheckpoint::Capture,
        CompilationCheckpoint::Vocabulary,
        CompilationCheckpoint::Frontend,
        CompilationCheckpoint::Semantics,
    ] {
        let cancellation = CancellationToken::new();
        let captured = capture(CompilationRequest::new(
            b"neu \"0.1\"\nmodule minimal\nstring greeting = \"hello\"\n".to_vec(),
            test_limits(),
            cancellation.clone(),
        ))
        .expect("valid source should capture");
        let result = compile_captured_with_checkpoints(&captured, |checkpoint| {
            if checkpoint == cancelled_at {
                cancellation.cancel();
            }
        });
        let CompilationResult::Failure(failure) = result else {
            panic!("checkpoint cancellation must not publish authoritative IR");
        };
        assert_eq!(failure.detail(), CompilationFailureDetail::Cancelled);
        assert!(failure.diagnostics().is_empty());
    }
}

#[test]
/// Verifies frozen syntax failures expose exact codes and original-byte spans.
fn compilation_exposes_frozen_frontend_diagnostics_without_ir() {
    let cases: [(&[u8], &str, (u64, u64)); 2] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/missing-module-header.neu"
            ),
            diagnostics::MISSING_MODULE_HEADER,
            (10, 10),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/unsupported-language-version.neu"
            ),
            diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
            (4, 9),
        ),
    ];

    for (source, expected_code, expected_span) in cases {
        let captured = capture(CompilationRequest::new(
            source.to_vec(),
            test_limits(),
            CancellationToken::new(),
        ))
        .expect("frozen negative fixture should capture");
        let super::CompilationResult::Failure(failure) = compile_captured(&captured) else {
            panic!("frozen negative fixture must not produce authoritative IR");
        };
        assert_eq!(failure.class(), neutral_core::ResultClass::Syntax);
        assert_eq!(failure.detail(), CompilationFailureDetail::SyntaxRejected);
        assert_eq!(failure.diagnostics().len(), 1);
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code().as_str(), expected_code);
        assert_eq!(
            (
                diagnostic.primary().span().start(),
                diagnostic.primary().span().end()
            ),
            expected_span
        );
    }
}

#[test]
/// Verifies public convenience entry points and capture metadata agree.
fn compiler_convenience_entry_points_preserve_contracts() {
    let source = b"neu \"0.1\"\nmodule minimal\n\nnum answer = 42\n".to_vec();
    let request = CompilationRequest::new(source.clone(), test_limits(), CancellationToken::new());
    let Ok(CompilationResult::Success(_)) = compile(request) else {
        panic!("valid request should compile");
    };

    let captured = capture(CompilationRequest::new(
        source.clone(),
        test_limits(),
        CancellationToken::new(),
    ))
    .expect("valid request should capture");
    assert_eq!(
        captured.language_behavior_version(),
        super::LANGUAGE_BEHAVIOR_VERSION
    );
    assert_eq!(captured.limits(), test_limits());
    assert!(!captured.has_captured_vocabulary());

    let formatted = format_captured(&captured).expect("captured source should format");
    assert_eq!(formatted.as_bytes(), source);
    assert_eq!(formatted.into_bytes(), source);
    let formatted = format(CompilationRequest::new(
        source.clone(),
        test_limits(),
        CancellationToken::new(),
    ))
    .expect("valid request should format");
    assert_eq!(formatted.as_bytes(), source);
}
