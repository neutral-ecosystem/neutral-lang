// SPDX-License-Identifier: Apache-2.0

//! Shared `/3` nominal syntax helpers against a current capture context.

use super::*;
use crate::test_support::capture_expression_source;

fn declaration_tokens(captured: &crate::CapturedExpressionProject, body: &str) -> Vec<Token> {
    let context = &captured.composition;
    let source = &context.sources()[0];
    let start = source
        .bytes()
        .windows(body.len())
        .position(|window| window == body.as_bytes())
        .expect("fixture body appears in captured source") as u64;
    crate::frontend::lexer::lex_bounded(source.bytes(), usize::MAX, None)
        .unwrap()
        .tokens
        .into_iter()
        .filter(|token| {
            token.span.start() >= start
                && token.span.end() <= start + body.len() as u64
                && !matches!(
                    token.kind,
                    TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
                )
        })
        .collect()
}

#[test]
fn current_capture_nominal_declarations_retain_syntax_diagnostics() {
    for (body, has_location) in [
        ("public num answer", true),
        ("public List answer = 1", true),
        ("public record Item { List field }", false),
    ] {
        let captured = capture_expression_source(body);
        let context = &captured.composition;
        let tokens = declaration_tokens(&captured, body);
        let Err(failure) =
            parse_declaration(&context.sources()[0], &tokens, context.composition_limits())
        else {
            panic!("accepted malformed current nominal source: {body}");
        };
        assert_eq!(failure.code, codes::INVALID_SOURCE, "{body}");
        assert_eq!(failure.location.is_some(), has_location, "{body}");
    }
}

#[test]
fn inverted_current_declaration_spans_fail_as_invalid_source() {
    let captured = capture_expression_source("public num answer = 1");
    let context = &captured.composition;
    let tokens = [
        Token {
            kind: TokenKind::Num,
            span: ByteSpan::new(2, 3).unwrap(),
        },
        Token {
            kind: TokenKind::Identifier("answer".to_owned()),
            span: ByteSpan::new(0, 1).unwrap(),
        },
    ];
    let Err(failure) =
        parse_declaration(&context.sources()[0], &tokens, context.composition_limits())
    else {
        panic!("accepted inverted declaration span");
    };
    assert_eq!(failure.code, codes::INVALID_SOURCE);
    assert_eq!(failure.location, None);
}
