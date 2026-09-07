// SPDX-License-Identifier: Apache-2.0

//! Fault injection at private parser token boundaries.

use super::*;

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
