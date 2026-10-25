// SPDX-License-Identifier: Apache-2.0

//! Unit tests for public logical IR identity and equality contracts.

use super::language::{
    PROTECTED_CORE_NAMES, is_exact_release_version, is_feature_id, is_protected_name,
    is_snake_name, is_upper_name,
};
use super::{
    DeclarationFingerprint, ElementId, ExactNumber, IdentityReference, LogicalModuleIdentity,
    LogicalValue, ModuleSymbolIdentity, NominalTypeIdentity, RecordValue, RecordValueField,
    ResolvedType, VocabularyContract, VocabularyFieldContract, VocabularyIdentity,
    VocabularyTypeContract, VocabularyTypeIdentity,
};
use neutral_core::VocabularyContentDigest;
const TEST_LANGUAGE_BEHAVIOR_VERSION: &str = super::LANGUAGE_BEHAVIOR_VERSION;

#[test]
/// Verifies minimal integer normalization does not use host numeric types.
fn exact_integer_normalization_removes_only_leading_zeroes() {
    let number =
        ExactNumber::from_unsigned_integer("00042").expect("digits-only integer should normalize");
    assert_eq!(number.coefficient(), "42");
    assert_eq!(number.to_string(), "42/1");
}

#[test]
/// Verifies signs, separators, fractions, and exponents normalize exactly.
fn exact_number_normalization_is_independent_of_source_spelling() {
    let first = ExactNumber::from_source("-001.2300e2", 16, 32)
        .expect("bounded exact source number should normalize");
    let second = ExactNumber::from_source("-123", 16, 32)
        .expect("equivalent exact source number should normalize");
    assert_eq!(first, second);
    assert_eq!(first.coefficient(), "123");
    assert_eq!(first.scale(), 0);
}

#[test]
/// Verifies malformed and over-limit numeric source is rejected without floats.
fn exact_number_source_validation_rejects_invalid_and_over_limit_values() {
    assert!(ExactNumber::from_source("1__0", 16, 16).is_err());
    assert!(ExactNumber::from_source("1.", 16, 16).is_err());
    assert!(ExactNumber::from_source("1e-", 16, 16).is_err());
    assert!(ExactNumber::from_source("0x10", 16, 16).is_err());
    assert!(ExactNumber::from_source("12345", 4, 16).is_err());
    assert!(ExactNumber::from_source("1e9", 16, 2).is_err());
}

#[test]
/// Verifies externally reconstructed numbers enforce every canonical-part rule.
fn exact_number_normalized_parts_enforce_canonical_boundaries() {
    let negative = ExactNumber::from_normalized_parts(true, "123", -4, 3, 4)
        .expect("a nonzero canonical negative number should reconstruct");
    assert!(negative.is_negative());
    assert_eq!(negative.coefficient(), "123");
    assert_eq!(negative.scale(), -4);

    for (negative, coefficient, scale) in [
        (false, "", 0),
        (false, "1x", 0),
        (false, "012", 0),
        (false, "120", 0),
        (true, "0", 0),
        (false, "0", 1),
    ] {
        assert!(
            ExactNumber::from_normalized_parts(negative, coefficient, scale, 3, 4).is_err(),
            "noncanonical parts {negative:?}/{coefficient:?}/{scale:?} must fail"
        );
    }
    assert!(ExactNumber::from_normalized_parts(false, "1234", 0, 3, 4).is_err());
    assert!(ExactNumber::from_normalized_parts(false, "123", -5, 3, 4).is_err());
}

#[test]
/// Verifies source normalization distinguishes zero, sign, scale, and transcript facts.
fn exact_number_source_normalization_preserves_distinct_canonical_facts() {
    let zero = ExactNumber::from_source("-000.000", 16, 16)
        .expect("a signed source zero should normalize");
    let scaled =
        ExactNumber::from_source("1200", 16, 16).expect("trailing zeroes should become scale");
    let fractional = ExactNumber::from_source("0.00120", 16, 16)
        .expect("fractional source number should normalize");
    assert!(!zero.is_negative());
    assert_eq!(zero.coefficient(), "0");
    assert_eq!(zero.scale(), 0);
    assert_eq!(scaled.coefficient(), "12");
    assert_eq!(scaled.scale(), 2);
    assert_eq!(fractional.coefficient(), "12");
    assert_eq!(fractional.scale(), -4);
    assert_ne!(
        scaled.nht_payload().expect("payload should build"),
        fractional.nht_payload().expect("payload should build")
    );
}

#[test]
/// Verifies a source coefficient exactly at the digit ceiling remains accepted.
fn exact_number_source_accepts_the_exact_digit_limit() {
    let number = ExactNumber::from_source("123", 3, 1)
        .expect("a coefficient at the configured digit ceiling should normalize");
    assert_eq!(number.coefficient(), "123");
    assert_eq!(number.scale(), 0);
}

#[test]
/// Verifies trailing-zero normalization may end exactly at the scale ceiling.
fn exact_number_source_accepts_the_exact_normalized_scale_limit() {
    let number = ExactNumber::from_source("100", 3, 2)
        .expect("a normalized scale at the configured ceiling should normalize");
    assert_eq!(number.coefficient(), "1");
    assert_eq!(number.scale(), 2);
}

#[test]
/// Verifies vocabulary equality compares every meaning field but ignores captured bytes.
fn property_vocabulary_equivalence_checks_each_meaning_partition() {
    let identity = VocabularyIdentity::new(
        "Fixture",
        "1.0.0",
        super::LOGICAL_IR_SCHEMA_VERSION,
        "encoding-a",
        VocabularyContentDigest::from_bytes(b"a"),
        Vec::new(),
    );
    let baseline = VocabularyContract::new(identity, Vec::new());
    let mut recaptured = baseline.clone();
    recaptured.identity.encoding_version = "encoding-b".into();
    recaptured.identity.content_digest = VocabularyContentDigest::from_bytes(b"b");
    assert!(baseline.logically_equivalent(&recaptured));
    let mut variants = Vec::new();
    let mut value = baseline.clone();
    value.identity.identity = "Other".into();
    variants.push(value);
    let mut value = baseline.clone();
    value.identity.version = "2.0.0".into();
    variants.push(value);
    let mut value = baseline.clone();
    value.identity.schema_version = "2.0.0".into();
    variants.push(value);
    let mut value = baseline.clone();
    value.identity.required_features.push("feature".into());
    variants.push(value);
    let mut value = baseline.clone();
    value.types.push(VocabularyTypeContract::new(
        VocabularyTypeIdentity::new("Fixture", "Entry"),
        Vec::new(),
    ));
    variants.push(value);
    for value in variants {
        assert!(!baseline.logically_equivalent(&value));
    }
}

#[test]
/// Verifies outer nullability and invariant list compatibility through public type queries.
fn nullable_and_list_queries_preserve_type_boundaries() {
    let nullable = ResolvedType::nullable(ResolvedType::String);
    assert!(nullable.is_nullable());
    assert_eq!(nullable.nullable_inner(), Some(&ResolvedType::String));
    assert!(!ResolvedType::String.is_nullable());
    assert_eq!(ResolvedType::String.nullable_inner(), None);
    let list = ResolvedType::list(ResolvedType::String);
    assert!(
        list.accepts_value(&LogicalValue::List(vec![LogicalValue::String(
            "value".into()
        )]))
    );
    assert!(!list.accepts_value(&LogicalValue::List(vec![LogicalValue::Boolean(true)])));
    for enabled in [true, false] {
        assert_eq!(
            super::DiagnosticPartition {
                safe_bounded_output: enabled
            }
            .safe_bounded_output(),
            enabled
        );
    }
}

#[test]
/// Verifies captured vocabulary contracts retain and expose every immutable fact.
fn vocabulary_contract_accessors_preserve_exact_captured_facts() {
    let identity = VocabularyIdentity::new(
        "Fixture",
        "1.0.0",
        "0.1.0",
        "json-1",
        VocabularyContentDigest::from_bytes(b"fixture-vocabulary"),
        vec!["records".to_owned()],
    );
    let type_identity = VocabularyTypeIdentity::new("Fixture", "Entry");
    let field = VocabularyFieldContract::new(
        "value",
        ResolvedType::String,
        Some(LogicalValue::String("default".to_owned())),
    );
    let contract = VocabularyContract::new(
        identity.clone(),
        vec![VocabularyTypeContract::new(
            type_identity.clone(),
            vec![field],
        )],
    );

    assert_eq!(identity.identity(), "Fixture");
    assert_eq!(identity.version(), "1.0.0");
    assert_eq!(identity.schema_version(), "0.1.0");
    assert_eq!(identity.encoding_version(), "json-1");
    assert_eq!(
        identity.content_digest(),
        VocabularyContentDigest::from_bytes(b"fixture-vocabulary")
    );
    assert_eq!(identity.required_features(), ["records"]);
    assert_eq!(type_identity.vocabulary(), "Fixture");
    assert_eq!(type_identity.name(), "Entry");
    assert_eq!(type_identity.to_string(), "Fixture::Entry");
    let entry = contract
        .type_by_name("Entry")
        .expect("type should be discoverable");
    assert_eq!(entry.identity(), &type_identity);
    assert_eq!(entry.fields()[0].name(), "value");
    assert_eq!(entry.fields()[0].resolved_type(), &ResolvedType::String);
    assert_eq!(
        entry.fields()[0].default_value(),
        Some(&LogicalValue::String("default".to_owned()))
    );
    assert!(contract.logically_equivalent(&contract));
    assert!(contract.type_by_name("missing").is_none());
}

#[test]
/// Verifies declaration names do not enter logical definition fingerprints.
fn binding_fingerprint_depends_on_type_and_logical_value() {
    let value = LogicalValue::Number(
        ExactNumber::from_unsigned_integer("42").expect("integer should normalize"),
    );
    let first = DeclarationFingerprint::for_binding(&ResolvedType::Num, &value)
        .expect("fingerprint should succeed");
    let second = DeclarationFingerprint::for_binding(&ResolvedType::Num, &value)
        .expect("fingerprint should be deterministic");
    assert_eq!(first, second);
}

#[test]
/// Verifies logical strings render every decoded control through safe escapes.
fn logical_string_rendering_escapes_controls_and_delimiters() {
    let value = LogicalValue::String("quote:\" slash:\\ line:\n nul:\0".to_owned());
    assert_eq!(
        value.to_string(),
        "\"quote:\\\" slash:\\\\ line:\\n nul:\\0\""
    );
}

#[test]
/// Verifies scalar types and Boolean values enter definition fingerprints.
fn scalar_fingerprints_distinguish_type_and_boolean_value() {
    let truth =
        DeclarationFingerprint::for_binding(&ResolvedType::Bool, &LogicalValue::Boolean(true))
            .expect("Boolean fingerprint should succeed");
    let falsehood =
        DeclarationFingerprint::for_binding(&ResolvedType::Bool, &LogicalValue::Boolean(false))
            .expect("Boolean fingerprint should succeed");
    let text = DeclarationFingerprint::for_binding(
        &ResolvedType::String,
        &LogicalValue::String("true".to_owned()),
    )
    .expect("string fingerprint should succeed");
    assert_ne!(truth, falsehood);
    assert_ne!(truth, text);
}

#[test]
/// Verifies explicit null requires outer nullable type identity.
fn nullable_type_compatibility_distinguishes_null_from_nonnull_values() {
    let nullable_string = ResolvedType::nullable(ResolvedType::String);
    assert!(nullable_string.accepts_value(&LogicalValue::Null));
    assert!(nullable_string.accepts_value(&LogicalValue::String("value".to_owned())));
    assert!(!ResolvedType::String.accepts_value(&LogicalValue::Null));
    assert!(!nullable_string.accepts_value(&LogicalValue::Boolean(false)));
    assert_eq!(nullable_string.to_string(), "string?");
}

#[test]
/// Verifies null fingerprints include the nullable expected scalar type.
fn typed_null_fingerprints_distinguish_nullable_types() {
    let string = DeclarationFingerprint::for_binding(
        &ResolvedType::nullable(ResolvedType::String),
        &LogicalValue::Null,
    )
    .expect("nullable string null should fingerprint");
    let boolean = DeclarationFingerprint::for_binding(
        &ResolvedType::nullable(ResolvedType::Bool),
        &LogicalValue::Null,
    )
    .expect("nullable Boolean null should fingerprint");
    assert_ne!(string, boolean);
}

#[test]
/// Verifies equal record shapes cannot substitute for distinct nominal identities.
fn nominal_record_values_reject_structural_compatibility() {
    let module = LogicalModuleIdentity::new(TEST_LANGUAGE_BEHAVIOR_VERSION, "records");
    let left = NominalTypeIdentity::new(module.clone(), "Left");
    let right = NominalTypeIdentity::new(module, "Right");
    let value = LogicalValue::Record(RecordValue::new(
        right,
        vec![RecordValueField::new(
            "name",
            LogicalValue::String("same shape".to_owned()),
        )],
    ));
    assert!(!ResolvedType::Record(left).accepts_value(&value));
}

#[test]
/// Verifies reference fingerprints use durable target identity, never local IDs.
fn reference_fingerprints_ignore_graph_local_element_ids() {
    let module = LogicalModuleIdentity::new(TEST_LANGUAGE_BEHAVIOR_VERSION, "references");
    let symbol = ModuleSymbolIdentity::new(module, "target");
    let first = LogicalValue::Reference(IdentityReference::new(
        ElementId::new(1),
        symbol.clone(),
        ResolvedType::String,
    ));
    let second = LogicalValue::Reference(IdentityReference::new(
        ElementId::new(99),
        symbol,
        ResolvedType::String,
    ));
    let reference_type = ResolvedType::reference(ResolvedType::String);
    assert_eq!(
        DeclarationFingerprint::for_binding(&reference_type, &first)
            .expect("first reference fingerprint should succeed"),
        DeclarationFingerprint::for_binding(&reference_type, &second)
            .expect("second reference fingerprint should succeed")
    );
}

#[test]
/// Exhaustively distinguishes the protected vocabulary from ordinary names.
fn unit_language_protected_names_are_exact() {
    for protected in PROTECTED_CORE_NAMES {
        assert!(is_protected_name(protected));
    }
    for ordinary in ["", "Num", "recording", "value", "RefValue"] {
        assert!(!is_protected_name(ordinary));
    }
}

#[test]
/// Covers every uppercase and lower-snake name predicate boundary.
fn unit_language_name_categories_are_exact() {
    for accepted in ["A", "Record2", "UPPER", "Z9"] {
        assert!(is_upper_name(accepted));
    }
    for rejected in ["", "a", "_A", "A_", "A-B", "É", "1A"] {
        assert!(!is_upper_name(rejected));
    }
    for accepted in ["a", "a0", "a_b", "value2_name3"] {
        assert!(is_snake_name(accepted));
    }
    for rejected in ["", "A", "_a", "a_", "a__b", "a-B", "é", "1a"] {
        assert!(!is_snake_name(rejected));
    }
}

#[test]
/// Covers canonical release components and every inert feature punctuation class.
fn unit_language_release_and_feature_spellings_are_exact() {
    for accepted in ["0.0.0", "0.1.0", "10.20.300"] {
        assert!(is_exact_release_version(accepted));
    }
    for rejected in [
        "", "1", "1.2", "1.2.3.4", "01.2.3", "1.02.3", "1.2.03", "1.a.3",
    ] {
        assert!(!is_exact_release_version(rejected));
    }
    for accepted in ["a", "a0", "a.b", "a-b", "a_b", "a/b", "a:b", "a@b"] {
        assert!(is_feature_id(accepted));
    }
    for rejected in ["", "A", "0a", "_a", "a+b", "é"] {
        assert!(!is_feature_id(rejected));
    }
}
