// SPDX-License-Identifier: Apache-2.0

//! Unit tests for semantic name validation and collection helpers.

use super::{AsciiNameCategory, classify_ascii_name};
use crate::language::names;

#[test]
/// Verifies exact ASCII snake and uppercase-leading name categories.
fn unit_semantics_validates_frozen_names() {
    assert_eq!(classify_ascii_name("answer_two2"), AsciiNameCategory::Snake);
    assert_eq!(classify_ascii_name("Record2"), AsciiNameCategory::Upper);
    assert_eq!(
        classify_ascii_name("Record_Name"),
        AsciiNameCategory::Invalid
    );
    assert_eq!(
        classify_ascii_name("answer__two"),
        AsciiNameCategory::Invalid
    );
}

#[test]
/// Verifies the frozen core namespace cannot be redeclared.
fn unit_semantics_protects_core_names() {
    assert!(names::is_protected_name(names::NUM));
    assert!(names::is_protected_name(names::RECORD));
    assert!(!names::is_protected_name("answer"));
}

#[test]
/// Invalid parsed numeric defaults retain their semantic diagnostic and source span.
fn unit_semantics_rejects_invalid_closed_number_default() {
    let span = neutral_core::ByteSpan::new(4, 7).unwrap();
    let limits = neutral_core::StructuralLimits::new(1024, 1).unwrap();
    let error = super::lower_closed_default(
        &neutral_ir::ResolvedType::Num,
        &crate::frontend::ParsedValue::Number("bad".into()),
        span,
        &std::collections::BTreeMap::new(),
        limits,
    )
    .expect_err("malformed numeric default must fail");

    assert_eq!(error.code, crate::diagnostics::INVALID_NUMBER);
    assert_eq!(error.span, span);
    assert_eq!(error.class, neutral_core::ResultClass::Semantics);
}

#[test]
/// Nested validated defaults preserve list order, record identities, and scalar values.
fn unit_semantics_lowers_nested_vocabulary_defaults() {
    use neutral_core::{StructuralLimits, VocabularyContentDigest};
    use neutral_ir::{
        LogicalValue, RecordValueField, VocabularyRecordValue, VocabularyTypeIdentity,
    };
    use neutral_vocabulary::{VocabularyLimits, VocabularyLock, validate_captured_bundle};

    let bytes = br#"{
        "format":"neutral-vocabulary-bundle", "encoding_version":"0.1",
        "schema_version":"0.1", "identity":"Fixture", "version":"0.1.0",
        "required_features":[], "types":[
            {"name":"Leaf","fields":[
                {"name":"label","type":{"kind":"string"},"default":null}]},
            {"name":"Branch","fields":[
                {"name":"leaves","type":{"kind":"list","element":{"kind":"record","target":"Leaf"}},"default":null}]},
            {"name":"Holder","fields":[
                {"name":"branch","type":{"kind":"record","target":"Branch"},"default":{
                    "kind":"record","fields":[{"name":"leaves","value":{
                        "kind":"list","items":[{"kind":"record","fields":[
                            {"name":"label","value":{"kind":"string","value":"nested"}}]}]}}]}}]}
        ]
    }"#;
    let lock = VocabularyLock::new(
        "Fixture",
        "0.1.0",
        "0.1",
        "0.1",
        VocabularyContentDigest::from_bytes(bytes),
        Vec::new(),
    )
    .unwrap();
    let bundle = validate_captured_bundle(
        bytes,
        &lock,
        VocabularyLimits::from_structural(StructuralLimits::new(4096, 1).unwrap()),
    )
    .unwrap();
    let field = &bundle.logical().type_by_name("Holder").unwrap().fields()[0];
    let actual = super::lower_vocabulary_default(
        "Fixture",
        field.field_type(),
        field.default_value().unwrap(),
    );
    let leaf = LogicalValue::VocabularyRecord(VocabularyRecordValue::new(
        VocabularyTypeIdentity::new("Fixture", "Leaf"),
        vec![RecordValueField::new(
            "label",
            LogicalValue::String("nested".into()),
        )],
    ));
    let expected = LogicalValue::VocabularyRecord(VocabularyRecordValue::new(
        VocabularyTypeIdentity::new("Fixture", "Branch"),
        vec![RecordValueField::new(
            "leaves",
            LogicalValue::List(vec![leaf]),
        )],
    ));

    assert_eq!(actual, expected);
}
