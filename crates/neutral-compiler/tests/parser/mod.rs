// SPDX-License-Identifier: Apache-2.0

//! Fault injection at private parser token boundaries.

use super::*;
use crate::{
    CapturedExpressionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project, compile_expression_project,
};
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_ir::expression::{profile as expression_profile, templates::ExpansionLimits};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

/// Compiles source syntax through the captured expression frontend.
fn compile_expression_source(body: &str) -> Result<(), crate::ExpressionCompileFailure> {
    let source = format!("neu \"1.0\"\nmodule example::parser\n{body}\n");
    let limits = ProjectCaptureLimitValues {
        total_source_bytes: 1_048_576,
        source_bytes_per_unit: 1_048_576,
        source_units: 2,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 2,
        vocabulary_bytes_per_unit: 1_048_576,
        total_vocabulary_bytes: 1_048_576,
        imports_per_module: 8,
        import_edges: 16,
        scc_units: 2,
        declarations: 4096,
        diagnostics: 64,
        output_bytes: 1_048_576,
    };
    let captured = capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            expression_profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:parser",
                "example::parser",
                source.into_bytes(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(limits),
                CancellationToken::new(),
            ),
        ),
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64).unwrap(),
        )),
        expression_profile::ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .expect("source headers should capture");
    compile_expression_project(&captured, &CancellationToken::new()).map(|_| ())
}

/// Creates ordered tokens for a deliberately incomplete grammar fragment.
fn tokens(kinds: Vec<TokenKind>) -> Vec<Token> {
    kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| Token {
            kind,
            span: span(index, index + 1),
        })
        .collect()
}

/// Constructs a parser over tokens that may intentionally lack an EOF sentinel.
fn parser(tokens: &[Token]) -> Parser<'_> {
    Parser {
        tokens,
        index: 0,
        limits: StructuralLimits::new(4096, 16).unwrap(),
        value_nodes: 0,
    }
}

#[test]
/// Rejects missing header and declaration operands without an EOF sentinel.
fn truncated_headers_and_declarations_fail_boundedly() {
    assert!(parser(&[]).parse_unit().is_err());
    assert!(parser(&tokens(vec![TokenKind::Neu])).parse_unit().is_err());
    assert!(
        parser(&tokens(vec![TokenKind::Module]))
            .parse_module()
            .is_err()
    );
    assert!(
        parser(&tokens(vec![TokenKind::Use]))
            .parse_vocabulary_use()
            .is_err()
    );
    assert!(
        parser(&tokens(vec![TokenKind::Record]))
            .parse_record()
            .is_err()
    );
    assert!(
        parser(&tokens(vec![TokenKind::Num]))
            .parse_binding()
            .is_err()
    );
    assert!(
        parser(&tokens(vec![TokenKind::Num]))
            .parse_record_field()
            .is_err()
    );
    let binding = tokens(vec![
        TokenKind::Num,
        TokenKind::Identifier("value".into()),
        TokenKind::Equals,
    ]);
    assert!(parser(&binding).parse_binding().is_err());
    assert!(parser(&binding).parse_record_field().is_err());
}

#[test]
/// Rejects missing nested values, type operands, and field delimiters.
fn truncated_recursive_operands_fail_boundedly() {
    assert!(parser(&[]).parse_type().is_err());
    assert!(parser(&[]).parse_value(0).is_err());
    assert!(
        parser(&[])
            .expect_field_delimiter(&TokenKind::Comma)
            .is_err()
    );
    assert!(
        parser(&tokens(vec![
            TokenKind::Identifier("V".into()),
            TokenKind::DoubleColon
        ]))
        .parse_type()
        .is_err()
    );
    assert!(
        parser(&tokens(vec![
            TokenKind::Identifier("V".into()),
            TokenKind::DoubleColon,
            TokenKind::Equals
        ]))
        .parse_type()
        .is_err()
    );
    assert!(
        parser(&tokens(vec![TokenKind::OpenParen]))
            .parse_reference_value()
            .is_err()
    );
    assert!(parser(&[]).parse_list_value(span(0, 0), 0).is_err());
    assert!(parser(&[]).parse_record_value(span(0, 0), 0).is_err());
    assert!(
        parser(&tokens(vec![
            TokenKind::Identifier("field".into()),
            TokenKind::Colon
        ]))
        .parse_record_value(span(0, 0), 0)
        .is_err()
    );
    // Both a remaining token and an exhausted stream must yield a safe location.
    parser(&tokens(vec![TokenKind::Equals])).other_here();
}

/// Truncated template and matrix declarations retain the template syntax diagnostic.
#[test]
fn expression_truncated_template_and_matrix_declarations_reject_safely() {
    for body in ["template", "expand", "template Item() -> num"] {
        let error = compile_expression_source(body).expect_err("truncated declaration");
        assert_eq!(error.code, "NEU-TPL-002", "{body}");
        assert!(error.location.is_some(), "{body}: {error:?}");
    }
}

/// Empty reference and instantiation primaries report the missing-name syntax class.
#[test]
fn expression_named_primary_operations_reject_empty_name() {
    for body in ["Ref<num> item = ref()", "num item = instantiate ()"] {
        let error = compile_expression_source(body).expect_err("empty primary name");
        assert_eq!(error.code, "NEU-EXP-002", "{body}: {error:?}");
    }
}

/// Bindings need `=` before expression parsing; input declarations remain assignment-free.
#[test]
fn expression_binding_declaration_requires_assignment_delimiter() {
    for body in ["num item", "public num item"] {
        let error = compile_expression_source(body).expect_err("missing assignment");
        assert_eq!(error.code, "NEU-EXP-002", "{body}");
        assert!(error.location.is_some(), "{body}");
    }
    assert!(compile_expression_source("input num item").is_ok());
}
