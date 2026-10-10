// SPDX-License-Identifier: Apache-2.0

//! records cross-package regressions.

use super::*;

#[test]
/// Verifies frozen nominal record fixtures through public logical IR.
fn conformance_nominal_record_positive_oracles() {
    let basic = compile_artifacts(NOMINAL_RECORD);
    let record = &basic.logical_document().record_types()[0];
    assert_eq!(record.name(), "Metadata");
    assert_eq!(
        record
            .fields()
            .iter()
            .map(|field| format!("{}: {}", field.name(), field.resolved_type()))
            .collect::<Vec<_>>(),
        ["enabled: bool", "image: string", "note: string?"]
    );
    let binding = &basic.logical_document().declarations()[0];
    assert_eq!(binding.resolved_type().to_string(), "Metadata");
    assert_eq!(
        binding.value().to_string(),
        "{enabled: true, image: \"example.invalid/tool:1\", note: null}"
    );

    let forward = compile_artifacts(RECORD_FORWARD_ORDER);
    assert_eq!(
        forward.logical_document().record_types()[0].name(),
        "Config"
    );
    assert_eq!(
        forward.logical_document().declarations()[0].name(),
        "config"
    );

    let nested = compile_artifacts(NESTED_RECORD);
    assert_eq!(nested.logical_document().record_types().len(), 2);
    assert_eq!(
        nested.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{metadata: {name: \"neutral\"}}"
    );
}

#[test]
/// Verifies reader and probe expose nominal schemas and contextual values.
fn system_nominal_records_cross_reader_and_probe() {
    let document = compile_reader(NOMINAL_RECORD);
    assert_eq!(
        document
            .record_type_by_name("Metadata")
            .unwrap()
            .fields()
            .len(),
        3
    );
    let summary = summarize(&document);
    assert_eq!(
        summary.record_types(),
        ["record Metadata { enabled: bool, image: string, note: string? }"]
    );
    assert_eq!(
        summary.declarations(),
        ["metadata: Metadata = {enabled: true, image: \"example.invalid/tool:1\", note: null}"]
    );
}

#[test]
/// Verifies root and field source order cannot change logical record meaning.
fn property_record_declaration_and_field_order_is_nonsemantic() {
    let first = b"neu \"0.1\"\nmodule ordering\nrecord Config { string name, bool enabled, }\nConfig config = { name: \"x\", enabled: true, }\n";
    let second = b"neu \"0.1\"\nmodule ordering\nConfig config = { enabled: true, name: \"x\", }\nrecord Config { bool enabled, string name, }\n";
    let first = compile_artifacts(first);
    let second = compile_artifacts(second);
    assert!(
        first
            .logical_document()
            .logically_equivalent(second.logical_document())
    );
}

#[test]
/// Verifies frozen nominal record failures own stable codes and source spans.
fn conformance_nominal_record_negative_oracles() {
    let cases: [FailureOracle<'_>; 9] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/duplicate-declaration.neu"
            ),
            ResultClass::Semantics,
            diagnostics::DUPLICATE_DECLARATION,
            (114, 118),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/duplicate-schema-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::DUPLICATE_RECORD_FIELD,
            (121, 125),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/missing-value-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::MISSING_RECORD_FIELD,
            (142, 166),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/unknown-value-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::UNKNOWN_RECORD_FIELD,
            (151, 156),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/duplicate-value-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::DUPLICATE_VALUE_FIELD,
            (153, 157),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/wrong-kind-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::WRONG_DECLARATION_KIND,
            (120, 125),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/embedded-recursion.neu"
            ),
            ResultClass::Semantics,
            diagnostics::EMBEDDED_RECORD_RECURSION,
            (135, 140),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/field-shorthand.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (130, 131),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/wrong-field-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (137, 142),
        ),
    ];
    for (source, class, code, expected_span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), class);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}

#[test]
/// Verifies record field and nesting limits fail through the resource boundary.
fn security_record_limits_fail_before_schema_resolution() {
    let fields = b"neu \"0.1\"\nmodule limits\nrecord Item { string first, string second, }\n";
    let field_limits = limits()
        .with_record_fields(1)
        .expect("record field limit should be valid");
    let nested = b"neu \"0.1\"\nmodule limits\nrecord Inner { string name, }\nrecord Outer { Inner inner, }\nOuter value = { inner: { name: \"x\", }, }\n";
    let depth_limits = limits()
        .with_nesting_depth(1)
        .expect("record depth limit should be valid");
    for (source, limits) in [
        (fields.as_slice(), field_limits),
        (nested.as_slice(), depth_limits),
    ] {
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("bounded record source should capture");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit record source must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::RECORD_LIMIT_EXCEEDED
        );
    }
}

#[test]
/// Verifies every required/defaulted and nullable/non-nullable field state.
fn conformance_closed_defaults_positive_oracles() {
    let matrix = compile_artifacts(FIELD_STATE_DEFAULTS);
    let schema = &matrix.logical_document().record_types()[0];
    assert_eq!(
        schema
            .fields()
            .iter()
            .map(|field| (field.name(), field.is_required()))
            .collect::<Vec<_>>(),
        [
            ("default_name", false),
            ("default_note", false),
            ("required_name", true),
            ("required_note", true),
        ]
    );
    assert_eq!(
        matrix.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{default_name: \"fallback\", default_note: null, required_name: \"given\", required_note: null}"
    );
    assert_eq!(
        matrix
            .field_provenance()
            .iter()
            .map(|record| (record.field_path().join("."), record.origin()))
            .collect::<Vec<_>>(),
        [
            ("default_name".to_owned(), ValueOrigin::UserRecordDefault),
            ("default_note".to_owned(), ValueOrigin::UserRecordDefault),
            ("required_name".to_owned(), ValueOrigin::ExplicitRecordField),
            ("required_note".to_owned(), ValueOrigin::ExplicitRecordField),
        ]
    );

    let nested = compile_artifacts(NESTED_RECORD_DEFAULT);
    assert_eq!(
        nested.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{metadata: {label: \"nested\"}}"
    );
    assert_eq!(
        nested.logical_document().record_types()[0].fields()[0]
            .default_value()
            .expect("closed record default must be materialized")
            .to_string(),
        "{label: \"nested\"}"
    );
}

#[test]
/// Verifies explicit and omitted fields retain final values and distinct provenance.
fn property_default_omission_changes_provenance_not_value_kind() {
    let omitted = compile_artifacts(
        b"neu \"0.1\"\nmodule same_default\nrecord Config { string name = \"same\", }\nConfig config = {}\n",
    );
    let explicit = compile_artifacts(
        b"neu \"0.1\"\nmodule same_default\nrecord Config { string name = \"same\", }\nConfig config = { name: \"same\", }\n",
    );
    assert!(
        omitted
            .logical_document()
            .logically_equivalent(explicit.logical_document())
    );
    assert_eq!(
        omitted.field_provenance()[0].origin(),
        ValueOrigin::UserRecordDefault
    );
    assert_eq!(
        explicit.field_provenance()[0].origin(),
        ValueOrigin::ExplicitRecordField
    );
}

#[test]
/// Verifies reader and probe expose final values and field provenance.
fn system_closed_defaults_cross_reader_and_probe() {
    let document = compile_reader(EXPLICIT_DEFAULT_OVERRIDE);
    let summary = summarize(&document);
    assert_eq!(
        summary.declarations(),
        ["config: Config = {name: \"explicit\"}"]
    );
    assert_eq!(
        summary.record_types(),
        ["record Config { name: string = \"fallback\" }"]
    );
    assert_eq!(summary.field_provenance().len(), 1);
    assert!(summary.field_provenance()[0].ends_with(":name:explicit-record-field"));
}

#[test]
/// Verifies the reader rejects record values with missing field evidence.
fn security_reader_rejects_incomplete_field_provenance() {
    let artifacts = compile_artifacts(FIELD_STATE_DEFAULTS)
        .as_ref()
        .clone()
        .with_field_provenance(Vec::new());
    assert_eq!(
        ValidatedDocument::from_compiler_output(Arc::new(artifacts))
            .expect_err("record fields without provenance must fail closed"),
        ReaderError::InvalidFieldProvenance
    );
}

#[test]
/// Verifies non-closed and ill-typed defaults fail with frozen ownership.
fn conformance_closed_defaults_negative_oracles() {
    let cases: [FailureOracle<'_>; 5] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/nonconstant-default.neu"
            ),
            ResultClass::Semantics,
            diagnostics::NON_CONSTANT_DEFAULT,
            (137, 143),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/reference-default.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (114, 124),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/expression-default.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_SYMBOL,
            (111, 112),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/wrong-default-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (111, 115),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/missing-nested-required.neu"
            ),
            ResultClass::Semantics,
            diagnostics::MISSING_RECORD_FIELD,
            (159, 161),
        ),
    ];
    for (source, class, code, expected_span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), class);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}
