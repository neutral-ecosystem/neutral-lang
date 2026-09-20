// SPDX-License-Identifier: Apache-2.0

//! Unit tests for foundational value contracts.

use super::profile::{
    LanguageCapability, LanguageProfile, PROFILE_DIAGNOSTIC_FAMILY, PROFILE_UNAVAILABLE_DIAGNOSTIC,
    ProfileAvailability, language_profile, language_profiles,
};
use super::{
    ByteSpan, CancellationToken, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity,
    DigestTextError, EncodedSectionDigest, SemanticDigest, SourceContentDigest, SourceLocation,
    StructuralLimits, VocabularyContentDigest, line_column_at, nht_frame,
};

#[test]
/// Verifies exact profile selection, availability, and stable capabilities.
fn profile_catalogue_is_explicit_complete_and_deterministic() {
    assert_eq!(
        LanguageProfile::from_source_version("0.1"),
        Some(LanguageProfile::V0_1)
    );
    assert_eq!(
        LanguageProfile::from_source_version("1.0"),
        Some(LanguageProfile::V1_0)
    );
    for lookalike in ["01.0", "1.00", "1", "v1.0", "1.0 "] {
        assert_eq!(LanguageProfile::from_source_version(lookalike), None);
    }
    assert_eq!(language_profiles().len(), 2);
    assert_eq!(
        language_profile(LanguageProfile::V0_1).availability(),
        ProfileAvailability::Available
    );
    assert!(
        language_profile(LanguageProfile::V0_1)
            .capabilities()
            .contains(&LanguageCapability::ImmutableData)
    );
    assert_eq!(
        language_profile(LanguageProfile::V1_0).availability(),
        ProfileAvailability::Unavailable
    );
    assert!(
        language_profile(LanguageProfile::V1_0)
            .capabilities()
            .is_empty()
    );
    assert!(PROFILE_UNAVAILABLE_DIAGNOSTIC.starts_with(PROFILE_DIAGNOSTIC_FAMILY));
}

#[test]
/// Verifies that exact source bytes affect the typed digest.
fn exact_source_bytes_affect_the_digest() {
    assert_ne!(
        SourceContentDigest::from_bytes(b"a\n"),
        SourceContentDigest::from_bytes(b"a\r\n")
    );
}

#[test]
/// Verifies frozen SHA-256 text vectors for exact captured source bytes.
fn source_digest_uses_the_frozen_sha256_text_form() {
    assert_eq!(
        SourceContentDigest::from_bytes(b"").to_string(),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        SourceContentDigest::from_bytes(b"abc").to_string(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
/// Verifies vocabulary byte identity and strict lowercase digest text parsing.
fn vocabulary_digest_uses_exact_bytes_and_strict_text() {
    let digest = VocabularyContentDigest::from_bytes(b"abc");
    let expected = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(digest.to_string(), expected);
    assert_eq!(
        VocabularyContentDigest::parse_text(expected).expect("frozen digest should parse"),
        digest
    );
    assert!(digest.securely_matches(digest));
    assert!(!digest.securely_matches(VocabularyContentDigest::from_bytes(b"abd")));
    for invalid in [
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "sha256:BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
        "sha512:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "sha256:ba78",
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad0",
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ag",
    ] {
        assert_eq!(
            VocabularyContentDigest::parse_text(invalid),
            Err(DigestTextError::InvalidSha256Text)
        );
    }
}

#[test]
/// Verifies exact NHT framing widths, order, and domain-separated hashing.
fn semantic_digest_uses_the_frozen_nht_v1_frame() {
    assert_eq!(
        nht_frame("x", b"ab").expect("short frame should succeed"),
        [0, 1, b'x', 0, 0, 0, 0, 0, 0, 0, 2, b'a', b'b']
    );
    assert_ne!(
        SemanticDigest::from_nht("domain-a", b"value").expect("ASCII domain should hash"),
        SemanticDigest::from_nht("domain-b", b"value").expect("ASCII domain should hash")
    );
}

#[test]
/// Verifies that inverted source spans are rejected.
fn inverted_spans_are_rejected() {
    assert!(ByteSpan::new(2, 1).is_err());
}

#[test]
/// Verifies that CRLF and lone CR retain deterministic original-byte locations.
fn line_columns_distinguish_supported_line_endings() {
    assert_eq!(
        line_column_at(b"a\r\nb\rc", 3).expect("offset should be valid"),
        super::LineColumn { line: 2, column: 1 }
    );
    assert_eq!(
        line_column_at(b"a\r\nb\rc", 5).expect("offset should be valid"),
        super::LineColumn { line: 3, column: 1 }
    );
}

#[test]
/// Verifies canonical diagnostic ordering uses source position before stable code.
fn diagnostics_sort_by_source_position_before_stable_code() {
    let source = SourceContentDigest::from_bytes(b"source");
    let later = Diagnostic::new(
        DiagnosticCode::new("AAA").expect("code should be valid"),
        DiagnosticLayer::Syntax,
        DiagnosticSeverity::Error,
        SourceLocation::new(source, ByteSpan::new(2, 3).expect("span should be valid")),
        Vec::new(),
        false,
    );
    let earlier = Diagnostic::new(
        DiagnosticCode::new("ZZZ").expect("code should be valid"),
        DiagnosticLayer::Syntax,
        DiagnosticSeverity::Error,
        SourceLocation::new(source, ByteSpan::new(1, 2).expect("span should be valid")),
        Vec::new(),
        false,
    );
    assert!(earlier < later);
    assert!(!earlier.is_truncated());
}

#[test]
/// Verifies every diagnostic tie-breaker agrees with equality and ordered collections.
fn diagnostic_ordering_resolves_equal_source_positions() {
    let location = SourceLocation::new(
        SourceContentDigest::from_bytes(b"source"),
        ByteSpan::new(0, 1).unwrap(),
    );
    let baseline = Diagnostic::new(
        DiagnosticCode::new("AAA").unwrap(),
        DiagnosticLayer::Syntax,
        DiagnosticSeverity::Note,
        location,
        Vec::new(),
        false,
    );
    assert_eq!(baseline.cmp(&baseline), std::cmp::Ordering::Equal);
    let mut variants = Vec::new();
    let mut value = baseline.clone();
    value.code = DiagnosticCode::new("BBB").unwrap();
    variants.push(value);
    variants.push(baseline.clone().with_related(vec![location]));
    let mut value = baseline.clone();
    value.parameters.push("detail".into());
    variants.push(value);
    let mut value = baseline.clone();
    value.layer = DiagnosticLayer::Semantics;
    variants.push(value);
    let mut value = baseline.clone();
    value.severity = DiagnosticSeverity::Error;
    variants.push(value);
    let mut value = baseline.clone();
    value.truncated = true;
    variants.push(value);
    for value in variants {
        assert!(baseline < value);
        assert!(value > baseline);
        assert_ne!(baseline, value);
    }
}

#[test]
/// Verifies decoded string limits are explicit, nonzero captured budgets.
fn structural_limits_capture_a_string_byte_budget() {
    let limits = StructuralLimits::new(1_024, 16)
        .expect("base limits should be valid")
        .with_string_bytes(64)
        .expect("string limit should be valid");
    assert_eq!(limits.string_bytes(), 64);
    assert!(limits.with_string_bytes(0).is_err());
}

#[test]
/// Verifies numeric digit and scale limits are explicit captured budgets.
fn structural_limits_capture_exact_number_budgets() {
    let limits = StructuralLimits::new(1_024, 16)
        .expect("base limits should be valid")
        .with_numeric_digits(64)
        .expect("numeric digit limit should be valid")
        .with_numeric_scale(32)
        .expect("numeric scale limit should be valid");
    assert_eq!(limits.numeric_digits(), 64);
    assert_eq!(limits.numeric_scale(), 32);
    assert!(limits.with_numeric_digits(0).is_err());
    assert!(limits.with_numeric_scale(0).is_err());
}

#[test]
/// Verifies declaration, record, list, and traversal budgets are explicit.
fn structural_limits_capture_collection_budgets() {
    let limits = StructuralLimits::new(1_024, 16)
        .expect("base limits should be valid")
        .with_declarations(8)
        .expect("declaration limit should be valid")
        .with_record_fields(16)
        .expect("record field limit should be valid")
        .with_nesting_depth(4)
        .expect("nesting depth limit should be valid")
        .with_list_items(32)
        .expect("list item limit should be valid")
        .with_traversal_nodes(64)
        .expect("traversal node limit should be valid");
    assert_eq!(limits.declarations(), 8);
    assert_eq!(limits.record_fields(), 16);
    assert_eq!(limits.nesting_depth(), 4);
    assert_eq!(limits.list_items(), 32);
    assert_eq!(limits.traversal_nodes(), 64);
    assert!(limits.with_declarations(0).is_err());
    assert!(limits.with_record_fields(0).is_err());
    assert!(limits.with_nesting_depth(0).is_err());
    assert!(limits.with_list_items(0).is_err());
    assert!(limits.with_traversal_nodes(0).is_err());
}

#[test]
/// Exercises typed digest reconstruction and raw-byte access contracts.
fn typed_digests_round_trip_validated_raw_bytes() {
    let source_bytes = [1_u8; 32];
    let source = SourceContentDigest::from_raw_bytes(source_bytes);
    assert_eq!(source.as_bytes(), source_bytes);

    let section = EncodedSectionDigest::from_bytes(b"section");
    assert_eq!(section, EncodedSectionDigest::from_bytes(b"section"));
    assert_ne!(section.as_bytes(), [0_u8; 32]);

    let vocabulary_bytes = [2_u8; 32];
    let vocabulary = VocabularyContentDigest::from_raw_bytes(vocabulary_bytes);
    assert_eq!(vocabulary.as_bytes(), vocabulary_bytes);

    let semantic_bytes = [3_u8; 32];
    let semantic = SemanticDigest::from_raw_bytes(semantic_bytes);
    assert_eq!(semantic.as_bytes(), semantic_bytes);
    assert_eq!(semantic.to_string(), "03".repeat(32));
}

#[test]
/// Exercises source-location, diagnostic, and line-column access contracts.
fn source_and_diagnostic_accessors_preserve_captured_facts() {
    let source = SourceContentDigest::from_bytes(b"a\nb");
    let span = ByteSpan::new(0, 1).expect("span should be valid");
    assert_eq!(span.start(), 0);
    assert_eq!(span.end(), 1);
    assert_eq!(span.len(), 1);
    assert!(!span.is_empty());
    assert!(
        ByteSpan::new(1, 1)
            .expect("empty span should be valid")
            .is_empty()
    );

    let location = SourceLocation::new(source, span);
    assert_eq!(location.source(), source);
    assert_eq!(location.span(), span);

    let related = SourceLocation::new(source, ByteSpan::new(2, 3).expect("span should be valid"));
    let diagnostic = Diagnostic::new(
        DiagnosticCode::new("TEST001").expect("code should be valid"),
        DiagnosticLayer::Semantics,
        DiagnosticSeverity::Note,
        location,
        vec!["safe".to_owned()],
        true,
    )
    .with_related(vec![related, related]);
    assert_eq!(diagnostic.code().as_str(), "TEST001");
    assert_eq!(diagnostic.layer(), DiagnosticLayer::Semantics);
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Note);
    assert_eq!(diagnostic.primary(), location);
    assert_eq!(diagnostic.related(), [related]);
    assert_eq!(diagnostic.parameters(), ["safe"]);
    assert!(diagnostic.is_truncated());

    let position = line_column_at(b"a\nb", 2).expect("offset should be valid");
    assert_eq!(position.line(), 2);
    assert_eq!(position.column(), 1);
    assert!(line_column_at(b"a", 2).is_err());
}

#[test]
/// Exercises base structural-limit accessors and shared cancellation state.
fn base_limits_and_cancellation_contracts_are_observable() {
    let limits = StructuralLimits::new(128, 7).expect("nonzero limits should be valid");
    assert_eq!(limits.source_bytes(), 128);
    assert_eq!(limits.diagnostics(), 7);
    assert!(StructuralLimits::new(0, 7).is_err());
    assert!(StructuralLimits::new(128, 0).is_err());

    let cancellation = CancellationToken::new();
    let observer = cancellation.clone();
    assert!(!observer.is_cancelled());
    cancellation.cancel();
    assert!(observer.is_cancelled());
}
