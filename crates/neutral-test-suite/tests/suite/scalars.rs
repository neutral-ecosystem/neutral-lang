// SPDX-License-Identifier: Apache-2.0

//! scalars cross-package regressions.

use super::*;

#[test]
/// Verifies frozen string and Boolean values, source facts, and provenance.
fn conformance_string_and_boolean_positive_oracles() {
    let cases = [
        (
            STRING_SOURCE,
            "string",
            "\"quote:\\\" slash:\\\\ newline:\\n nul:\\0 unicode:🙂 raw:é\"",
            152,
            (72, 151),
            (72, 78),
            (79, 86),
            (89, 151),
            neutral_ir::Normalization::StringEscapeDecoding,
            51,
        ),
        (
            BOOLEAN_TRUE,
            "bool",
            "true",
            92,
            (72, 91),
            (72, 76),
            (77, 84),
            (87, 91),
            neutral_ir::Normalization::BooleanIdentity,
            0,
        ),
        (
            BOOLEAN_FALSE,
            "bool",
            "false",
            93,
            (72, 92),
            (72, 76),
            (77, 84),
            (87, 92),
            neutral_ir::Normalization::BooleanIdentity,
            0,
        ),
    ];
    for (
        source,
        expected_type,
        expected_value,
        length,
        declaration,
        type_span,
        name_span,
        value_span,
        normalization,
        decoded_bytes,
    ) in cases
    {
        let artifacts = compile_artifacts(source);
        let binding = &artifacts.logical_document().declarations()[0];
        let mapping = artifacts
            .source_map()
            .entry(binding.element_id())
            .expect("scalar binding should have source facts");
        assert_eq!(binding.resolved_type().to_string(), expected_type);
        assert_eq!(binding.value().to_string(), expected_value);
        assert_eq!(artifacts.source_map().source_byte_length(), length);
        assert_eq!(span_pair(mapping.declaration_span()), declaration);
        assert_eq!(span_pair(mapping.type_span()), type_span);
        assert_eq!(span_pair(mapping.name_span()), name_span);
        assert_eq!(span_pair(mapping.value_span()), value_span);
        assert_eq!(artifacts.provenance()[0].normalization(), normalization);
        assert_eq!(
            artifacts
                .derivation()
                .resource_facts()
                .decoded_string_bytes(),
            decoded_bytes
        );
    }
}

#[test]
/// Verifies every frozen invalid string, Boolean, and version spelling.
fn conformance_string_and_boolean_negative_oracles() {
    let cases: [FailureOracle<'_>; 9] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-unknown-escape.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (93, 95),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-invalid-surrogate.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (90, 98),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-out-of-range.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (90, 100),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-raw-control.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (93, 94),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-unterminated.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNTERMINATED_STRING_LITERAL,
            (89, 102),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-type-mismatch.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (89, 93),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/booleans/invalid-boolean-literal.neu"
            ),
            ResultClass::Semantics,
            diagnostics::UNKNOWN_VALUE,
            (87, 91),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/version-escape.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
            (4, 14),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/version-leading-zero.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
            (4, 10),
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
/// Verifies reader traversal exposes typed values without reading source text.
fn integration_reader_exposes_typed_string_and_boolean_values() {
    let string_document = compile_reader(STRING_SOURCE);
    assert!(matches!(
        string_document.declarations()[0].value(),
        neutral_ir::LogicalValue::String(_)
    ));
    let bool_document = compile_reader(BOOLEAN_FALSE);
    assert!(matches!(
        bool_document.declarations()[0].value(),
        neutral_ir::LogicalValue::Boolean(false)
    ));
}

#[test]
/// Verifies probe rendering never emits decoded control characters directly.
fn system_probe_escapes_hostile_string_controls() {
    let summary = summarize(&compile_reader(STRING_SOURCE));
    let rendered = &summary.declarations()[0];
    assert!(rendered.contains("\\n"));
    assert!(rendered.contains("\\0"));
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains('\r'));
    assert!(!rendered.contains('\t'));
    assert!(!rendered.contains('\0'));
}

#[test]
/// Verifies all simple escapes and Unicode scalar boundaries decode exactly.
fn property_string_escapes_and_unicode_scalar_boundaries() {
    let values = [
        r#""""#,
        r#""\"\\""#,
        r#""\n\r\t\0""#,
        r#""\u{0}""#,
        r#""\u{10ffff}""#,
        "\"é🙂\"",
    ];
    for value in values {
        let source = format!("neu \"0.1\"\nmodule scalar_strings\nstring message = {value}\n");
        assert!(matches!(
            compile(CompilationRequest::new(
                source.into_bytes(),
                limits(),
                CancellationToken::new(),
            )),
            Ok(CompilationResult::Success(_))
        ));
    }
}

#[test]
/// Verifies decoded string limits fail through the resource result boundary.
fn security_decoded_string_limit_fails_before_ir_allocation() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-limit.neu"
    );
    let limits = StructuralLimits::new(1_024, 16)
        .expect("base limits should be valid")
        .with_string_bytes(8)
        .expect("string limit should be valid");
    let result = compile(CompilationRequest::new(
        source.to_vec(),
        limits,
        CancellationToken::new(),
    ))
    .expect("source bytes should remain within capture limits");
    let CompilationResult::Failure(failure) = result else {
        panic!("over-limit decoded string must produce no IR");
    };
    assert_eq!(failure.class(), ResultClass::Resource);
    assert_eq!(
        failure.detail(),
        CompilationFailureDetail::ResourceLimitExceeded
    );
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::STRING_LIMIT_EXCEEDED
    );
    assert_eq!(
        span_pair(failure.diagnostics()[0].primary().span()),
        (89, 100)
    );
}
