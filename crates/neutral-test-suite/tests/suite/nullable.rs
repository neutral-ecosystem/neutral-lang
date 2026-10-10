// SPDX-License-Identifier: Apache-2.0

//! nullable cross-package regressions.

use super::*;

#[test]
/// Verifies nullable scalar null and outer widening fixtures through public IR.
fn conformance_nullable_scalar_positive_oracles() {
    let null_cases = [
        (NULLABLE_STRING_NULL, "string?"),
        (NULLABLE_NUM_NULL, "num?"),
        (NULLABLE_BOOL_NULL, "bool?"),
    ];
    for (source, expected_type) in null_cases {
        let artifacts = compile_artifacts(source);
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.resolved_type().to_string(), expected_type);
        assert_eq!(declaration.value(), &neutral_ir::LogicalValue::Null);
        assert_eq!(
            artifacts.provenance()[0].normalization(),
            neutral_ir::Normalization::NullIdentity
        );
    }

    let widened = compile_artifacts(NULLABLE_SCALAR_WIDENING);
    let declaration = &widened.logical_document().declarations()[0];
    assert_eq!(declaration.resolved_type().to_string(), "string?");
    assert_eq!(declaration.value().to_string(), "\"present\"");
}

#[test]
/// Verifies reader/probe retain explicit null as a typed declaration value.
fn system_nullable_null_remains_distinct_from_absence() {
    let document = compile_reader(NULLABLE_STRING_NULL);
    let declaration = document
        .declaration_by_name("value")
        .expect("explicit null declaration must not be omitted");
    assert_eq!(declaration.resolved_type().to_string(), "string?");
    assert_eq!(declaration.value(), &neutral_ir::LogicalValue::Null);
    assert_eq!(
        summarize(&document).declarations(),
        ["value: string? = null"]
    );
}

#[test]
/// Verifies null and malformed nullability fail at their frozen boundaries.
fn conformance_nullable_scalar_negative_oracles() {
    let cases: [FailureOracle<'_>; 2] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/nullability/nonnullable-null.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (89, 93),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/nullability/double-nullable.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (80, 81),
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
/// Verifies nullable type identity enters null definition fingerprints.
fn property_typed_null_fingerprints_remain_distinct() {
    let string = compile_artifacts(NULLABLE_STRING_NULL);
    let number = compile_artifacts(NULLABLE_NUM_NULL);
    let boolean = compile_artifacts(NULLABLE_BOOL_NULL);
    let fingerprints = [string, number, boolean]
        .map(|artifacts| artifacts.logical_document().declarations()[0].fingerprint());
    assert_ne!(fingerprints[0], fingerprints[1]);
    assert_ne!(fingerprints[1], fingerprints[2]);
    assert_ne!(fingerprints[0], fingerprints[2]);
}
