// SPDX-License-Identifier: Apache-2.0

//! Fault injection at private parser token boundaries.

use super::super::FrontendErrorKind;
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
/// Parser boundaries reject missing EOF and non-name tokens in header and field positions.
fn invalid_header_and_field_name_tokens_fail_with_stable_kinds() {
    let no_eof = tokens(vec![
        TokenKind::Neu,
        TokenKind::StringLiteral(super::super::DecodedString {
            value: "0.1".to_owned(),
            had_escape: false,
        }),
        TokenKind::LineEnd,
        TokenKind::Module,
        TokenKind::Identifier("example".to_owned()),
        TokenKind::LineEnd,
    ]);
    assert!(parser(&no_eof).parse_unit().is_err());

    for (tokens, parse, expected) in [
        (
            tokens(vec![TokenKind::Use, TokenKind::LineEnd]),
            0,
            FrontendErrorKind::Other,
        ),
        (
            tokens(vec![TokenKind::Module, TokenKind::LineEnd]),
            1,
            FrontendErrorKind::Other,
        ),
        (
            tokens(vec![TokenKind::Record, TokenKind::LineEnd]),
            2,
            FrontendErrorKind::Other,
        ),
        (
            tokens(vec![TokenKind::Num, TokenKind::Number("1".to_owned())]),
            3,
            FrontendErrorKind::MalformedBoundary,
        ),
        (
            tokens(vec![TokenKind::Number("1".to_owned()), TokenKind::Colon]),
            4,
            FrontendErrorKind::MalformedBoundary,
        ),
    ] {
        let error = match parse {
            0 => parser(&tokens).parse_vocabulary_use().unwrap_err(),
            1 => parser(&tokens).parse_module().unwrap_err(),
            2 => parser(&tokens).parse_record().unwrap_err(),
            3 => parser(&tokens).parse_record_field().unwrap_err(),
            _ => parser(&tokens)
                .parse_record_value(span(0, 0), 0)
                .unwrap_err(),
        };
        assert_eq!(error.kind, expected);
    }
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

/// Every incomplete token prefix of a valid grammar production must reject without publishing syntax.
fn assert_truncated_production(
    source: &[u8],
    parse: impl Fn(&mut Parser<'_>) -> Result<(), FrontendError>,
) {
    let mut lexed = super::super::lexer::lex(source).unwrap();
    for token in &mut lexed.tokens {
        if matches!(token.kind, TokenKind::PhysicalLineEnd(_)) {
            token.kind = TokenKind::LineEnd;
        }
    }
    let mut complete = parser(&lexed.tokens);
    parse(&mut complete).expect("positive grammar control");
    for end in 0..complete.index {
        let failure =
            parse(&mut parser(&lexed.tokens[..end])).expect_err("truncated production must fail");
        assert!(
            matches!(
                failure.kind,
                FrontendErrorKind::Other
                    | FrontendErrorKind::MalformedBoundary
                    | FrontendErrorKind::MissingModuleHeader
            ),
            "prefix {end}: {failure:?}"
        );
        assert!(failure.span.end() <= source.len() as u64, "prefix {end}");
    }
}

/// Header, record, type and reference operands are all required at their own parser boundary.
#[test]
fn grammar_productions_reject_every_truncated_token_prefix() {
    assert_truncated_production(
        b"neu \"0.1\"\n\nmodule example\n\nuse Fixture\n\nnum value = 1\n",
        |p| p.parse_unit().map(|_| ()),
    );
    assert_truncated_production(b"module example\n", |p| p.parse_module().map(|_| ()));
    assert_truncated_production(b"use Fixture\n", |p| p.parse_vocabulary_use().map(|_| ()));
    assert_truncated_production(b"record Item { num value = 1, }\n", |p| {
        p.parse_record().map(|_| ())
    });
    assert_truncated_production(b"num value = 1,\n", |p| p.parse_record_field().map(|_| ()));
    for source in [b"List<num>\n".as_slice(), b"Ref<Item>\n"] {
        assert_truncated_production(source, |p| p.parse_type().map(|_| ()));
    }
    assert_truncated_production(b"(target)\n", |p| p.parse_reference_value().map(|_| ()));
    assert_truncated_production(b"{ first: 1, second: [2, 3,], }\n", |p| {
        p.parse_value(0).map(|_| ())
    });
}

/// Wrong token categories do not masquerade as headers, field delimiters or literal values.
#[test]
fn grammar_entrypoints_classify_wrong_operand_tokens() {
    let invalid = tokens(vec![TokenKind::Comma]);
    assert_eq!(
        parser(&invalid).parse_module().unwrap_err().kind,
        FrontendErrorKind::Other
    );
    assert_eq!(
        parser(&invalid).parse_vocabulary_use().unwrap_err().kind,
        FrontendErrorKind::Other
    );
    assert_eq!(
        parser(&invalid).parse_record().unwrap_err().kind,
        FrontendErrorKind::Other
    );
    assert_eq!(
        parser(&invalid).parse_value(0).unwrap_err().kind,
        FrontendErrorKind::Other
    );
    let header = tokens(vec![TokenKind::Neu, TokenKind::Num]);
    assert_eq!(
        parser(&header).parse_unit().unwrap_err().kind,
        FrontendErrorKind::Other
    );
}

/// Contextual name positions preserve keyword spelling for the later protected-name check.
#[test]
fn keyword_name_tokens_preserve_their_exact_spelling() {
    for (kind, expected) in [
        (TokenKind::Neu, "neu"),
        (TokenKind::Module, "module"),
        (TokenKind::Use, "use"),
        (TokenKind::Record, "record"),
        (TokenKind::List, "List"),
        (TokenKind::Num, "num"),
        (TokenKind::StringType, "string"),
        (TokenKind::BoolType, "bool"),
        (TokenKind::True, "true"),
        (TokenKind::False, "false"),
        (TokenKind::Null, "null"),
        (TokenKind::RefType, "Ref"),
        (TokenKind::RefValue, "ref"),
    ] {
        let token = Token {
            kind,
            span: span(0, expected.len()),
        };
        assert_eq!(identifier_spelling(&token).as_deref(), Some(expected));
    }
}

/// A record value has its own field-count bound even when the surrounding declaration count is small.
#[test]
fn contextual_record_values_enforce_their_immediate_field_limit() {
    let source = b"{ first: 1, second: 2, }";
    let lexed = super::super::lexer::lex(source).unwrap();
    for limit in [1, 2, 3] {
        let mut parser = parser(&lexed.tokens);
        parser.limits = parser.limits.with_record_fields(limit).unwrap();
        let result = parser.parse_value(0);
        if limit == 1 {
            let error = result.unwrap_err();
            assert_eq!(error.kind, FrontendErrorKind::RecordLimitExceeded);
            assert_eq!(error.span, span(12, 18));
        } else {
            let ParsedValue::Record(fields) = result.unwrap() else {
                panic!("record value");
            };
            assert_eq!(
                fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>(),
                ["first", "second"]
            );
        }
    }
}
