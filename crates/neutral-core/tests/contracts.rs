// SPDX-License-Identifier: Apache-2.0

//! Public-boundary integration tests for foundational contracts.

use neutral_core::{
    ByteSpan, CancellationToken, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity,
    EncodedSectionDigest, SemanticDigest, SourceContentDigest, SourceLocation, StructuralLimits,
    VocabularyContentDigest, line_column_at,
};

#[test]
/// Verifies the public value API retains exact typed source facts.
fn public_value_contract_retains_exact_facts() {
    let source_bytes = [1_u8; 32];
    let source = SourceContentDigest::from_raw_bytes(source_bytes);
    assert_eq!(source.as_bytes(), source_bytes);
    assert_ne!(
        EncodedSectionDigest::from_bytes(b"section").as_bytes(),
        [0_u8; 32]
    );
    assert_eq!(
        VocabularyContentDigest::from_raw_bytes([2_u8; 32]).as_bytes(),
        [2_u8; 32]
    );
    let semantic = SemanticDigest::from_raw_bytes([3_u8; 32]);
    assert_eq!(semantic.as_bytes(), [3_u8; 32]);
    assert_eq!(semantic.to_string(), "03".repeat(32));

    let span = ByteSpan::new(0, 1).expect("span should be valid");
    assert_eq!((span.start(), span.end(), span.len()), (0, 1, 1));
    assert!(!span.is_empty());
    let location = SourceLocation::new(source, span);
    assert_eq!(location.source(), source);
    assert_eq!(location.span(), span);
    let position = line_column_at(b"a\nb", 2).expect("position should exist");
    assert_eq!((position.line(), position.column()), (2, 1));
}

#[test]
/// Verifies public diagnostic, limit, and cancellation observations.
fn public_control_contract_exposes_bounded_state() {
    let source = SourceContentDigest::from_bytes(b"source");
    let location = SourceLocation::new(source, ByteSpan::new(0, 1).expect("valid span"));
    let diagnostic = Diagnostic::new(
        DiagnosticCode::new("TEST001").expect("valid code"),
        DiagnosticLayer::Semantics,
        DiagnosticSeverity::Note,
        location,
        vec!["safe".to_owned()],
        true,
    );
    assert_eq!(diagnostic.code().as_str(), "TEST001");
    assert_eq!(diagnostic.layer(), DiagnosticLayer::Semantics);
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Note);
    assert_eq!(diagnostic.primary(), location);
    assert_eq!(diagnostic.parameters(), ["safe"]);
    assert!(diagnostic.is_truncated());

    let limits = StructuralLimits::new(128, 7).expect("nonzero limits should be valid");
    assert_eq!((limits.source_bytes(), limits.diagnostics()), (128, 7));
    let cancellation = CancellationToken::new();
    let observer = cancellation.clone();
    assert!(!observer.is_cancelled());
    cancellation.cancel();
    assert!(observer.is_cancelled());
}
