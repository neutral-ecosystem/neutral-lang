// SPDX-License-Identifier: Apache-2.0

//! Executable conformance tests for the strict captured vocabulary boundary.

use super::{
    ValidatedVocabularyBundle, VocabularyError, VocabularyLimits, VocabularyLock, VocabularyType,
    VocabularyValue, schema, validate_captured_bundle,
};
use neutral_core::{StructuralLimits, VocabularyContentDigest};

/// Comprehensive accepted exact-byte bundle.
const COMPREHENSIVE: &[u8] = include_bytes!(
    "../../../../portable/specs/fixtures/vocabulary/bundles/positive/comprehensive.json"
);
/// Logically equal reordered and reformatted bundle.
const REORDERED: &[u8] = include_bytes!(
    "../../../../portable/specs/fixtures/vocabulary/bundles/positive/reordered.json"
);
/// Empty valid bundle used for generated hostile byte cases.
const EMPTY_BUNDLE: &[u8] = b"{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"0.1\",\"schema_version\":\"0.1\",\"identity\":\"Fixture\",\"version\":\"0.1.0\",\"required_features\":[],\"types\":[]}";

/// Returns generous deterministic limits for bundle conformance tests.
fn limits() -> VocabularyLimits {
    let structural =
        StructuralLimits::new(100_000, 16).expect("nonzero vocabulary test limits should be valid");
    VocabularyLimits::from_structural(structural)
}

/// Creates exact lock facts for one fixture's current captured bytes.
fn lock(bytes: &[u8]) -> VocabularyLock {
    lock_with_features(bytes, Vec::new())
}

/// Creates exact lock facts with one caller-selected structural feature set.
fn lock_with_features(bytes: &[u8], features: Vec<String>) -> VocabularyLock {
    VocabularyLock::new(
        "Fixture",
        "0.1.0",
        schema::ENCODING_VERSION,
        schema::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        features,
    )
    .expect("test lock facts should be valid")
}

/// Validates one fixture using its matching exact lock facts.
fn validate(bytes: &[u8]) -> Result<ValidatedVocabularyBundle, VocabularyError> {
    validate_captured_bundle(bytes, &lock(bytes), limits())
}

#[test]
/// Verifies every allowed type/default form and reference-only recursion.
fn conformance_vocabulary_comprehensive_bundle() {
    let bundle = validate(COMPREHENSIVE).expect("comprehensive bundle should validate");
    assert_eq!(bundle.logical().identity(), "Fixture");
    assert_eq!(bundle.logical().types().len(), 3);
    let metadata = bundle
        .logical()
        .type_by_name("Metadata")
        .expect("metadata type should exist");
    assert_eq!(metadata.fields().len(), 6);
    let count = metadata
        .fields()
        .iter()
        .find(|field| field.name() == "count")
        .expect("numeric default should exist");
    let Some(VocabularyValue::Number(number)) = count.default_value() else {
        panic!("count must retain an exact numeric default")
    };
    assert_eq!(number.coefficient(), "105");
    assert_eq!(number.scale(), -1);
    let node = bundle
        .logical()
        .type_by_name("Node")
        .expect("recursive identity type should exist");
    assert!(
        matches!(node.fields()[0].field_type(), VocabularyType::Ref(target) if target == "Node")
    );
}

#[test]
/// Verifies byte formatting changes identity but not normalized logical meaning.
fn property_vocabulary_order_and_format_are_logically_nonsemantic() {
    let first = validate(COMPREHENSIVE).expect("canonical bundle should validate");
    let second = validate(REORDERED).expect("reordered bundle should validate");
    assert_ne!(first.content_digest(), second.content_digest());
    assert_ne!(first, second);
    assert!(first.logically_equivalent(&second));
}

#[test]
/// Verifies duplicate keys are rejected at envelope and nested object depths.
fn security_vocabulary_duplicate_members_fail_before_map_collapse() {
    for bytes in [
        include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/duplicate-envelope-key.json").as_slice(),
        include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/duplicate-nested-key.json").as_slice(),
    ] {
        assert_eq!(validate(bytes), Err(VocabularyError::DuplicateJsonMember));
    }
}

#[test]
/// Verifies unknown and executable-looking shapes fail the closed schema.
fn security_vocabulary_closed_schema_rejects_unknown_and_executable_shapes() {
    let unknown = include_bytes!(
        "../../../../portable/specs/fixtures/vocabulary/bundles/negative/unknown-envelope-member.json"
    );
    let executable = include_bytes!(
        "../../../../portable/specs/fixtures/vocabulary/bundles/negative/executable-member.json"
    );
    let executable_kind = include_bytes!(
        "../../../../portable/specs/fixtures/vocabulary/bundles/negative/unknown-type-kind.json"
    );
    assert_eq!(validate(unknown), Err(VocabularyError::UnknownMember));
    assert_eq!(
        validate(executable),
        Err(VocabularyError::ExecutableShapeForbidden)
    );
    assert_eq!(
        validate(executable_kind),
        Err(VocabularyError::ExecutableShapeForbidden)
    );
}

#[test]
/// Verifies raw numbers, truncation, BOM, invalid UTF-8, and surrogates fail.
fn security_vocabulary_strict_json_bytes_fail_closed() {
    let raw_number = include_bytes!(
        "../../../../portable/specs/fixtures/vocabulary/bundles/negative/raw-json-number.json"
    );
    let truncated = include_bytes!(
        "../../../../portable/specs/fixtures/vocabulary/bundles/negative/truncated.json"
    );
    assert_eq!(
        validate(raw_number),
        Err(VocabularyError::RawJsonNumberForbidden)
    );
    assert_eq!(validate(truncated), Err(VocabularyError::MalformedJson));

    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(EMPTY_BUNDLE);
    assert_eq!(
        validate_captured_bundle(&bom, &lock(&bom), limits()),
        Err(VocabularyError::BomForbidden)
    );
    let invalid_utf8 = [0xff];
    assert_eq!(
        validate_captured_bundle(&invalid_utf8, &lock(&invalid_utf8), limits()),
        Err(VocabularyError::InvalidUtf8)
    );
    let surrogate = br#"{"format":"neutral-vocabulary-bundle","encoding_version":"0.1","schema_version":"0.1","identity":"Fixture","version":"0.1.0","required_features":[],"types":[{"name":"Metadata","fields":[{"name":"label","type":{"kind":"string"},"default":{"kind":"string","value":"\uD800"}}]}]}"#;
    assert_eq!(
        validate_captured_bundle(surrogate, &lock(surrogate), limits()),
        Err(VocabularyError::MalformedJson)
    );
}

#[test]
/// Verifies type targets, embedding recursion, reference defaults, and features.
fn conformance_vocabulary_semantic_graph_failures() {
    let cases = [
        (include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/unknown-type-target.json").as_slice(), VocabularyError::UnknownTypeTarget),
        (include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/embedded-recursion.json").as_slice(), VocabularyError::InvalidTypeRecursion),
        (include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/reference-default.json").as_slice(), VocabularyError::InvalidDefault),
        (include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/incompatible-default.json").as_slice(), VocabularyError::InvalidDefault),
        (include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/incomplete-record-default.json").as_slice(), VocabularyError::InvalidDefault),
        (include_bytes!("../../../../portable/specs/fixtures/vocabulary/bundles/negative/unknown-feature.json").as_slice(), VocabularyError::UnknownRequiredFeature),
    ];
    for (bytes, expected) in cases {
        assert_eq!(validate(bytes), Err(expected));
    }
}

#[test]
/// Verifies digest and byte limits run before parsing or proportional work.
fn security_vocabulary_integrity_and_limits_precede_parsing() {
    let malformed = b"{";
    assert_eq!(
        validate_captured_bundle(malformed, &lock(EMPTY_BUNDLE), limits()),
        Err(VocabularyError::DigestMismatch)
    );
    let limited = limits()
        .with_bundle_bytes(8)
        .expect("nonzero byte limit should be valid");
    assert_eq!(
        validate_captured_bundle(EMPTY_BUNDLE, &lock(EMPTY_BUNDLE), limited),
        Err(VocabularyError::BundleByteLimitExceeded)
    );
    let limited = limits()
        .with_object_members(2)
        .expect("nonzero member limit should be valid");
    assert_eq!(
        validate_captured_bundle(EMPTY_BUNDLE, &lock(EMPTY_BUNDLE), limited),
        Err(VocabularyError::JsonLimitExceeded)
    );
}

#[test]
/// Verifies an allowed exact structural feature set is normalized and retained.
fn conformance_vocabulary_exact_feature_lock() {
    let bytes = b"{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"0.1\",\"schema_version\":\"0.1\",\"identity\":\"Fixture\",\"version\":\"0.1.0\",\"required_features\":[\"neutral.feature/records@1\"],\"types\":[]}";
    let feature = "neutral.feature/records@1".to_owned();
    let lock = lock_with_features(bytes, vec![feature.clone()]);
    let bundle = validate_captured_bundle(bytes, &lock, limits())
        .expect("exact locked feature should validate");
    assert_eq!(bundle.logical().required_features(), [feature]);
}
