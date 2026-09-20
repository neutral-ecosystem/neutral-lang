// SPDX-License-Identifier: Apache-2.0

//! Tests for the complete private active frontend pipeline.

use super::{
    FrontendErrorKind, ParsedValue, PhysicalLineEnd, TokenKind, TriviaKind, layout, lexer, parse,
};

/// Parses one exact captured source byte sequence.
fn parse_source(source: &[u8]) -> Result<super::ParsedUnit, super::FrontendError> {
    parse(
        source,
        neutral_core::StructuralLimits::new(4_096, 16)
            .expect("frontend test limits should be valid"),
    )
}

/// Returns the only binding in one scalar frontend test unit.
fn only_binding(unit: &super::ParsedUnit) -> &super::ParsedBinding {
    let [super::ParsedDeclaration::Binding(binding)] = unit.declarations.as_slice() else {
        panic!("scalar frontend test must contain exactly one binding");
    };
    binding
}

#[test]
/// Verifies raw tokens retain original spans and physical newline spellings.
fn lexer_retains_minimal_fixture_spans_and_physical_newlines() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/syntax/minimal-core.neu"
    );
    let source = lexer::lex(source).expect("frozen minimal fixture should lex");
    assert!(matches!(source.tokens[0].kind, TokenKind::Neu));
    assert_eq!(source.tokens[0].span.start(), 0);
    assert_eq!(source.tokens[0].span.end(), 3);
    assert!(source.tokens.iter().any(|token| matches!(
        token.kind,
        TokenKind::PhysicalLineEnd(super::PhysicalLineEnd::LineFeed)
    )));
}

#[test]
/// Verifies whitespace and both comment forms retain exact private trivia.
fn lexer_retains_nonsemantic_trivia_without_parser_tokens() {
    let source = lexer::lex(b"neu/*a*/ \"0.1\"//b\nmodule minimal\nnum answer = 42")
        .expect("commented source should lex");
    assert!(source.trivia.iter().any(|trivia| {
        trivia.kind == TriviaKind::BlockComment
            && (trivia.span.start(), trivia.span.end()) == (3, 8)
    }));
    assert!(source.trivia.iter().any(|trivia| {
        trivia.kind == TriviaKind::LineComment
            && (trivia.span.start(), trivia.span.end()) == (14, 17)
    }));
    assert!(
        source
            .trivia
            .iter()
            .any(|trivia| trivia.kind == TriviaKind::HorizontalWhitespace)
    );
}

#[test]
/// Verifies a multiline block comment still exposes raw physical newlines.
fn lexer_retains_physical_newlines_inside_block_comments() {
    let source = lexer::lex(b"/* first\r\nsecond */")
        .expect("terminated multiline block comment should lex");
    assert!(source.tokens.iter().any(|token| matches!(
        token.kind,
        TokenKind::PhysicalLineEnd(PhysicalLineEnd::CarriageReturnLineFeed)
    )));
}

#[test]
/// Verifies layout creates semantic terminators and preserves EOF.
fn layout_normalizes_minimal_fixture_line_ends() {
    let source = b"neu \"0.1\"\r\nmodule minimal\rnum answer = 42";
    let raw = lexer::lex(source).expect("source should lex");
    let source = layout::normalize(raw).expect("layout should normalize");
    assert_eq!(
        source
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::LineEnd))
            .count(),
        3
    );
    assert!(matches!(
        source.tokens.last().map(|token| &token.kind),
        Some(TokenKind::EndOfFile)
    ));
}

#[test]
/// Verifies physical newlines stay nonsemantic inside every delimiter family.
fn layout_tracks_open_and_close_delimiter_depth() {
    for source in [
        b"num value = [\n1,\n2\n]\n".as_slice(),
        b"record Value {\nstring label\n}\n".as_slice(),
        b"num value = (\n42\n)\n".as_slice(),
    ] {
        let raw = lexer::lex(source).expect("delimited source should lex");
        let normalized = layout::normalize(raw).expect("nested newlines should normalize");
        assert_eq!(
            normalized
                .tokens
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::LineEnd))
                .count(),
            1
        );
    }
}

#[test]
/// Verifies the positive frozen fixture produces its expected private spans.
fn parser_matches_the_minimal_frozen_oracle() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/syntax/minimal-core.neu"
    );
    let unit = parse_source(source).expect("frozen minimal fixture should parse");
    let binding = only_binding(&unit);
    assert_eq!((unit.module.span.start(), unit.module.span.end()), (10, 24));
    assert_eq!((binding.span.start(), binding.span.end()), (26, 41));
    assert_eq!(
        (binding.type_span.start(), binding.type_span.end()),
        (26, 29)
    );
    assert_eq!(
        (binding.name_span.start(), binding.name_span.end()),
        (30, 36)
    );
    assert_eq!(
        (binding.value_span.start(), binding.value_span.end()),
        (39, 41)
    );
    assert_eq!(unit.module.name, "minimal");
    assert_eq!(binding.name, "answer");
    assert_eq!(binding.value, ParsedValue::Number("42".to_owned()));
}

#[test]
/// Verifies the missing-module fixture matches its frozen diagnostic span.
fn parser_matches_the_missing_module_frozen_oracle() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/missing-module-header.neu"
    );
    let error = parse_source(source).expect_err("missing module must fail");
    assert_eq!(error.kind, FrontendErrorKind::MissingModuleHeader);
    assert_eq!((error.span.start(), error.span.end()), (10, 10));
}

#[test]
/// Verifies the unsupported-version fixture matches its frozen diagnostic span.
fn parser_matches_the_unsupported_version_frozen_oracle() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/unsupported-language-version.neu"
    );
    let error = parse_source(source).expect_err("unsupported version must fail");
    assert_eq!(error.kind, FrontendErrorKind::UnsupportedLanguageVersion);
    assert_eq!((error.span.start(), error.span.end()), (4, 9));
}

#[test]
/// Verifies the unavailable v1 profile cannot fall through to v0 parsing.
fn parser_rejects_the_unavailable_v1_profile() {
    let source = b"neu \"1.0\"\nmodule future\n\nnum answer = 42\n";
    let error = parse_source(source).expect_err("the unavailable v1 profile must fail");

    assert_eq!(error.kind, FrontendErrorKind::UnavailableLanguageProfile);
    assert_eq!((error.span.start(), error.span.end()), (4, 9));
}

#[test]
/// Verifies newline spellings and optional final termination are logically equal.
fn parser_treats_supported_newline_forms_as_logically_equal() {
    let variants: [&[u8]; 6] = [
        b"neu \"0.1\"\nmodule minimal\nnum answer = 42\n",
        b"neu \"0.1\"\nmodule minimal\nnum answer = 42",
        b"neu \"0.1\"\r\nmodule minimal\r\nnum answer = 42\r\n",
        b"neu \"0.1\"\r\nmodule minimal\r\nnum answer = 42",
        b"neu \"0.1\"\rmodule minimal\rnum answer = 42\r",
        b"neu \"0.1\"\rmodule minimal\rnum answer = 42",
    ];
    let logical = variants.map(|source| {
        let unit = parse_source(source).expect("newline variant should parse");
        let binding = only_binding(&unit);
        (
            unit.module.name.clone(),
            binding.name.clone(),
            binding.value.clone(),
        )
    });
    assert!(logical.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
/// Verifies string escapes and raw Unicode decode before private semantics.
fn parser_decodes_string_scalars_exactly() {
    let source = b"neu \"0.1\"\nmodule scalar\nstring message = \"a\\n\\0\\u{1f642}\xc3\xa9\"\n";
    let unit = parse_source(source).expect("valid escaped string should parse");
    assert_eq!(
        only_binding(&unit).value,
        ParsedValue::String("a\n\0🙂é".to_owned())
    );
}

#[test]
/// Verifies both exact Boolean tokens become typed private values.
fn parser_recognizes_exact_boolean_literals() {
    for (spelling, expected) in [("true", true), ("false", false)] {
        let source = format!("neu \"0.1\"\nmodule scalar\nbool enabled = {spelling}\n");
        let unit = parse_source(source.as_bytes()).expect("Boolean source should parse");
        assert_eq!(only_binding(&unit).value, ParsedValue::Boolean(expected));
    }
}

#[test]
/// Verifies one leading UTF-8 BOM is ignored without shifting original spans.
fn frontend_accepts_one_leading_bom_and_retains_original_offsets() {
    let source = b"\xef\xbb\xbfneu \"0.1\"\nmodule minimal\nnum answer = 42\n";
    let unit = parse_source(source).expect("one leading BOM should be accepted");
    assert_eq!(unit.language_header_span.start(), 3);
    assert_eq!(unit.version_span.start(), 7);
    assert_eq!(unit.module.name, "minimal");
}

#[test]
/// Verifies malformed encoding, NUL, BOM, headers, and numbers fail safely.
fn frontend_rejects_malformed_minimal_inputs_safely() {
    let malformed: [&[u8]; 14] = [
        b"\xff",
        b"neu \"0.1\"\0\nmodule minimal\nnum answer = 42\n",
        b"neu \"0.1\"\n\xef\xbb\xbfmodule minimal\nnum answer = 42\n",
        b"neu 0.1\nmodule minimal\nnum answer = 42\n",
        b"neu \"0.1\"\nmodule\nnum answer = 42\n",
        b"neu \"0.1\"\nmodule minimal\nnum answer = --42\n",
        b"neu \"0.1\"\nuse\nmodule minimal\nnum answer = 42\n",
        b"neu \"0.1\"\nmodule minimal\nrecord\n",
        b"neu \"0.1\"\nmodule minimal\nrecord R {\n123\n}\n",
        b"neu \"0.1\"\nmodule minimal\nnum = 42\n",
        b"neu \"0.1\"\nmodule minimal\n123type x = 1\n",
        b"neu \"0.1\"\nmodule minimal\nnum[] x = [1,\n",
        b"neu \"0.1\"\nmodule minimal\nR x = R {\n",
        b"neu \"0.1\"\nmodule minimal\nnum x = @invalid\n",
    ];
    for (idx, source) in malformed.iter().enumerate() {
        assert!(
            parse_source(source).is_err(),
            "malformed source at index {idx} unexpectedly succeeded"
        );
    }

    // Limit error in parser
    let limits = neutral_core::StructuralLimits::new(4_096, 16)
        .unwrap()
        .with_declarations(1)
        .unwrap();
    let source = b"neu \"0.1\"\nmodule minimal\nnum a = 1\nnum b = 2\n";
    assert!(parse(source, limits).is_err());
}

#[test]
/// Verifies vocabulary use, records, lists, references, and nested values parse in the frontend.
fn parser_handles_all_syntax_shapes() {
    let source = br#"neu "0.1"
module full_syntax

use Fixture

record Point {
    num x,
    num y,
}

num answer = 42
Point pt = {
    x: 10,
    y: 20,
}
List<num> list = [1, 2, 3]
Ref<Point> ptr = ref(pt)
"#;
    let unit = parse_source(source).expect("full syntax fixture should parse");
    assert_eq!(unit.module.name, "full_syntax");
    assert!(unit.vocabulary_use.is_some());
    assert_eq!(unit.declarations.len(), 5);
}
