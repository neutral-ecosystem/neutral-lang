// SPDX-License-Identifier: Apache-2.0

//! Public-boundary integration tests for vocabulary contracts.

use neutral_core::{StructuralLimits, VocabularyContentDigest};
use neutral_vocabulary::{
    MAX_VOCABULARY_NESTING_DEPTH, PROJECT_VOCABULARY_ENCODING_VERSION,
    PROJECT_VOCABULARY_SCHEMA_VERSION, VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION,
    VocabularyError, VocabularyLimits, VocabularyLock, validate_project_bundle,
};

/// Builds one exact locked v1 bundle from its closed-schema type entries.
fn project_bundle(types: &str) -> Vec<u8> {
    format!(
        "{{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"{PROJECT_VOCABULARY_ENCODING_VERSION}\",\"schema_version\":\"{PROJECT_VOCABULARY_SCHEMA_VERSION}\",\"identity\":\"Fixture\",\"version\":\"1.0.0\",\"required_features\":[],\"types\":[{types}]}}"
    )
    .into_bytes()
}

/// Validates hostile v1 bytes against their exact matching content lock.
fn project_validate(bytes: &[u8], limits: VocabularyLimits) -> Result<(), VocabularyError> {
    let lock = VocabularyLock::new(
        "Fixture",
        "1.0.0",
        PROJECT_VOCABULARY_ENCODING_VERSION,
        PROJECT_VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        Vec::new(),
    )
    .expect("test lock is valid");
    validate_project_bundle(bytes, &lock, limits).map(|_| ())
}

/// Returns generous but finite v1 JSON limits for exact-lock tests.
fn project_limits() -> VocabularyLimits {
    VocabularyLimits::from_structural(StructuralLimits::new(4096, 1).expect("nonzero limits"))
}

#[test]
/// Rejects duplicate, unknown, and executable members at all schema levels.
fn security_project_vocabulary_closed_schema_rejects_hostile_members() {
    let valid = project_bundle(
        "{\"name\":\"Visible\",\"public\":true,\"fields\":[{\"name\":\"label\",\"type\":\"string\"}]}",
    );
    assert_eq!(project_validate(&valid, project_limits()), Ok(()));
    let cases = [
        (
            "{\"name\":\"Visible\",\"name\":\"Again\",\"public\":true,\"fields\":[]}",
            VocabularyError::DuplicateJsonMember,
        ),
        (
            "{\"name\":\"Visible\",\"public\":true,\"fields\":[],\"extra\":true}",
            VocabularyError::UnknownMember,
        ),
        (
            "{\"name\":\"Visible\",\"public\":true,\"fields\":[],\"script\":\"run\"}",
            VocabularyError::ExecutableShapeForbidden,
        ),
        (
            "{\"name\":\"Visible\",\"public\":true,\"fields\":[{\"name\":\"x\",\"type\":\"num\",\"extra\":true}]}",
            VocabularyError::UnknownMember,
        ),
        (
            "{\"name\":\"Visible\",\"public\":true,\"fields\":[{\"name\":\"x\",\"type\":\"num\",\"name\":\"y\"}]}",
            VocabularyError::DuplicateJsonMember,
        ),
    ];
    for (types, expected) in cases {
        let bytes = project_bundle(types);
        assert_eq!(project_validate(&bytes, project_limits()), Err(expected));
    }
}

#[test]
/// Exercises exact byte, type, field, member, and nesting bounds.
fn security_project_vocabulary_limits_fail_at_one_over_boundary() {
    let types = "{\"name\":\"Visible\",\"public\":true,\"fields\":[{\"name\":\"label\",\"type\":\"string\"}]}";
    let bytes = project_bundle(types);
    let exact = project_limits()
        .with_bundle_bytes(bytes.len() as u64)
        .expect("exact byte limit");
    assert_eq!(project_validate(&bytes, exact), Ok(()));
    let short = project_limits()
        .with_bundle_bytes((bytes.len() - 1) as u64)
        .expect("short byte limit");
    assert_eq!(
        project_validate(&bytes, short),
        Err(VocabularyError::BundleByteLimitExceeded)
    );
    assert_eq!(
        project_validate(&bytes, project_limits().with_types(1).expect("one type")),
        Ok(())
    );
    let two_types = project_bundle(&format!("{types},{types}"));
    assert_eq!(
        project_validate(
            &two_types,
            project_limits().with_types(1).expect("one type")
        ),
        Err(VocabularyError::JsonLimitExceeded)
    );
    assert_eq!(
        project_validate(
            &two_types,
            project_limits()
                .with_array_items(1)
                .expect("one array item")
        ),
        Err(VocabularyError::JsonLimitExceeded)
    );
    let structural = StructuralLimits::new(4096, 1)
        .expect("base limits")
        .with_record_fields(1)
        .expect("one field");
    let one_field = VocabularyLimits::from_structural(structural)
        .with_object_members(7)
        .expect("root object members");
    assert_eq!(project_validate(&bytes, one_field), Ok(()));
    let extra_field = project_bundle(
        "{\"name\":\"Visible\",\"public\":true,\"fields\":[{\"name\":\"a\",\"type\":\"string\"},{\"name\":\"b\",\"type\":\"string\"}]}",
    );
    assert_eq!(
        project_validate(&extra_field, one_field),
        Err(VocabularyError::JsonLimitExceeded)
    );
    let limited_members = project_limits()
        .with_object_members(6)
        .expect("six members");
    assert_eq!(
        project_validate(&bytes, limited_members),
        Err(VocabularyError::JsonLimitExceeded)
    );
    let depth_two = VocabularyLimits::from_structural(
        StructuralLimits::new(4096, 1)
            .expect("base limits")
            .with_nesting_depth(2)
            .expect("two levels"),
    );
    assert_eq!(
        project_validate(&bytes, depth_two),
        Err(VocabularyError::JsonLimitExceeded)
    );
    let one_node = VocabularyLimits::from_structural(
        StructuralLimits::new(4096, 1)
            .expect("base limits")
            .with_traversal_nodes(1)
            .expect("one node"),
    );
    assert_eq!(
        project_validate(&bytes, one_node),
        Err(VocabularyError::JsonLimitExceeded)
    );
    let two_features = String::from_utf8(project_bundle(""))
        .expect("ASCII bundle")
        .replace(
            "\"required_features\":[]",
            "\"required_features\":[\"a\",\"b\"]",
        );
    assert_eq!(
        project_validate(
            two_features.as_bytes(),
            project_limits().with_features(1).expect("one feature")
        ),
        Err(VocabularyError::JsonLimitExceeded)
    );
}

#[test]
/// Truncation, invalid UTF-8, BOM, and oversized strings fail without a model.
fn security_project_vocabulary_hostile_bytes_fail_closed() {
    let bytes = project_bundle("");
    for end in 0..bytes.len() {
        assert!(project_validate(&bytes[..end], project_limits()).is_err());
    }
    assert_eq!(
        project_validate(&[0xff], project_limits()),
        Err(VocabularyError::InvalidUtf8)
    );
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend(&bytes);
    assert_eq!(
        project_validate(&bom, project_limits()),
        Err(VocabularyError::BomForbidden)
    );
    let short_strings = VocabularyLimits::from_structural(
        StructuralLimits::new(4096, 1)
            .expect("base limits")
            .with_string_bytes(5)
            .expect("five-byte strings"),
    );
    assert_eq!(
        project_validate(&bytes, short_strings),
        Err(VocabularyError::JsonLimitExceeded)
    );
    let levels =
        usize::try_from(MAX_VOCABULARY_NESTING_DEPTH + 1).expect("test depth fits target usize");
    let deep = format!("{}null{}", "[".repeat(levels), "]".repeat(levels));
    assert_eq!(
        project_validate(deep.as_bytes(), project_limits()),
        Err(VocabularyError::JsonLimitExceeded)
    );
    assert_eq!(
        project_limits().nesting_depth(),
        MAX_VOCABULARY_NESTING_DEPTH
    );
}

#[test]
/// Every one-byte mutation of a valid v1 bundle returns a bounded result.
fn fuzz_smoke_project_vocabulary_single_byte_mutations_terminate() {
    let bytes = project_bundle(
        "{\"name\":\"Visible\",\"public\":true,\"fields\":[{\"name\":\"label\",\"type\":\"string\"}]}",
    );
    for index in 0..bytes.len() {
        let mut mutation = bytes.clone();
        mutation[index] ^= 0x80;
        let _ = project_validate(&mutation, project_limits());
    }
}

#[test]
/// Verifies every public vocabulary-limit observation at the crate boundary.
fn public_vocabulary_limits_retain_effective_policy() {
    let structural = StructuralLimits::new(100, 3)
        .expect("base limits should be valid")
        .with_string_bytes(90)
        .expect("string limit should be valid")
        .with_declarations(80)
        .expect("type limit should be valid")
        .with_record_fields(70)
        .expect("field limit should be valid")
        .with_nesting_depth(60)
        .expect("depth limit should be valid")
        .with_list_items(50)
        .expect("array limit should be valid")
        .with_traversal_nodes(40)
        .expect("node limit should be valid");
    let limits = VocabularyLimits::from_structural(structural)
        .with_bundle_bytes(30)
        .expect("bundle limit should be valid")
        .with_object_members(20)
        .expect("object limit should be valid")
        .with_array_items(10)
        .expect("array limit should be valid")
        .with_types(9)
        .expect("type limit should be valid")
        .with_features(8)
        .expect("feature limit should be valid");
    assert_eq!(limits.bundle_bytes(), 30);
    assert_eq!(limits.string_bytes(), 90);
    assert_eq!(limits.object_members(), 20);
    assert_eq!(limits.array_items(), 10);
    assert_eq!(limits.nesting_depth(), 60);
    assert_eq!(limits.total_nodes(), 40);
    assert_eq!(limits.types(), 9);
    assert_eq!(limits.fields(), 70);
    assert_eq!(limits.features(), 8);
}

#[test]
/// Verifies normalized vocabulary-lock facts through the public API.
fn public_vocabulary_lock_retains_exact_facts() {
    let digest = VocabularyContentDigest::from_bytes(b"bundle");
    let lock = VocabularyLock::new(
        "Fixture",
        "0.1.0",
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        digest,
        vec!["neutral.feature/records@1".to_owned()],
    )
    .expect("lock should be valid");
    assert_eq!(lock.identity(), "Fixture");
    assert_eq!(lock.version(), "0.1.0");
    assert_eq!(lock.encoding_version(), VOCABULARY_ENCODING_VERSION);
    assert_eq!(lock.schema_version(), VOCABULARY_SCHEMA_VERSION);
    assert_eq!(lock.content_digest(), digest);
    assert_eq!(lock.required_features(), ["neutral.feature/records@1"]);
}
