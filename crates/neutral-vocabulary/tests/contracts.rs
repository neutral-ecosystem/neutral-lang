// SPDX-License-Identifier: Apache-2.0

//! Public-boundary integration tests for vocabulary contracts.

use neutral_core::{StructuralLimits, VocabularyContentDigest};
use neutral_vocabulary::{
    VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLimits, VocabularyLock,
};

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
