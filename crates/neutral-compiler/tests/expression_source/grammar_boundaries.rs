// SPDX-License-Identifier: Apache-2.0

//! Source-facing grammar boundaries formerly hidden in an unregistered old parser module.

use super::*;

/// Incomplete template/matrix headers fail before any declaration is published.
#[test]
fn incomplete_template_and_matrix_declarations_reject_safely() {
    for body in ["template", "expand", "template Item() -> num"] {
        let error = compile(body).expect_err("truncated declaration");
        assert_eq!(error.code, ec::TPL_SYNTAX, "{body}");
        assert!(error.location.is_some(), "{body}: {error:?}");
    }
}

/// Named primaries require an actual target, and ordinary bindings require assignment.
#[test]
fn empty_names_and_missing_assignment_reject_with_syntax_diagnostics() {
    for body in [
        "Ref<num> item = ref()",
        "num item = instantiate ()",
        "num item",
        "public num item",
    ] {
        let error = compile(body).expect_err("incomplete source production");
        assert_eq!(error.code, ec::EXP_SYNTAX, "{body}");
    }
    compile("input num item").unwrap();
}

/// The record's immediate field ceiling is independent of expression/declaration capacity.
#[test]
fn record_fields_accept_exact_limit_and_reject_one_over() {
    let source = "neu \"1.0\"\nmodule example::expressions\nrecord Item { num a, num b }\nItem item = { a: 1, b: 2 }\n";
    for count in [1, 2, 3] {
        let composition = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64)
                .unwrap()
                .with_record_fields(count)
                .unwrap(),
        ));
        let captured = capture_expression_project(request_with_inputs(
            vec![CapturedSourceInput::new(
                "source:fixture",
                "example::expressions",
                source.as_bytes().to_vec(),
            )],
            Vec::new(),
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            composition,
            (ExpressionLimits::hard(), ExpansionLimits::hard()),
            CancellationToken::new(),
        ))
        .unwrap();
        let result = compile_expression_project(&captured, &CancellationToken::new());
        if count < 2 {
            assert!(result.is_err(), "one-over field count must reject");
        } else {
            result.unwrap();
        }
    }
}
