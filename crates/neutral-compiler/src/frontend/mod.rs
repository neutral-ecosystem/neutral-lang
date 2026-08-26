// SPDX-License-Identifier: Apache-2.0

//! Private orchestration for the minimal captured-source frontend.

use neutral_core::{
    ByteSpan, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity, SourceContentDigest,
    SourceLocation,
};

mod layout;
mod lexer;
mod parser;

/// Frozen diagnostic code for a missing module header.
const MISSING_MODULE_HEADER: &str = "NEU-SYN-001";
/// Frozen diagnostic code for an unsupported language version.
const UNSUPPORTED_LANGUAGE_VERSION: &str = "NEU-SYN-002";

/// A raw or normalized token retained only inside the compiler frontend.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    /// Token category and safe decoded value, when applicable.
    kind: TokenKind,
    /// Exact half-open span in the original captured bytes.
    span: ByteSpan,
}

/// Token categories required by the Stage 2 minimal source slice.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenKind {
    /// The `neu` document-header keyword.
    Neu,
    /// The `module` header keyword.
    Module,
    /// The `num` core-type keyword.
    Num,
    /// An ASCII identifier retained for later semantic validation.
    Identifier(String),
    /// A raw, unescaped string body used only for the version header.
    StringLiteral(String),
    /// A digits-only minimal numeric spelling retained for later semantics.
    Number(String),
    /// The binding initializer delimiter.
    Equals,
    /// An original physical newline before layout normalization.
    PhysicalLineEnd(PhysicalLineEnd),
    /// A semantic declaration/header terminator.
    LineEnd,
    /// End of captured source.
    EndOfFile,
}

/// Original physical newline spellings retained by the raw lexer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PhysicalLineEnd {
    /// One line-feed byte.
    LineFeed,
    /// One carriage-return byte not followed by line feed.
    CarriageReturn,
    /// A carriage-return and line-feed pair.
    CarriageReturnLineFeed,
}

/// A private successfully parsed minimal source unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedUnit {
    /// Exact span of the language header.
    pub(super) language_header_span: ByteSpan,
    /// Exact span of the quoted language version.
    pub(super) version_span: ByteSpan,
    /// Parsed module header.
    pub(super) module: ParsedModule,
    /// Parsed minimal numeric binding.
    pub(super) binding: ParsedBinding,
}

/// A compiler-private parsed module header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedModule {
    /// Module name spelling retained for Step 4 semantic validation.
    pub(super) name: String,
    /// Exact span of the complete module header.
    pub(super) span: ByteSpan,
}

/// A compiler-private parsed minimal numeric binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedBinding {
    /// Binding name spelling retained for Step 4 semantic validation.
    pub(super) name: String,
    /// Minimal numeric spelling retained without host numeric conversion.
    pub(super) number: String,
    /// Exact span of the complete binding.
    pub(super) span: ByteSpan,
    /// Exact span of the explicit `num` type.
    pub(super) type_span: ByteSpan,
    /// Exact span of the binding name.
    pub(super) name_span: ByteSpan,
    /// Exact span of the numeric source value.
    pub(super) value_span: ByteSpan,
}

/// A private frontend failure with an optional frozen public diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FrontendError {
    /// Failure kind used to classify frozen and not-yet-active syntax.
    kind: FrontendErrorKind,
    /// Exact primary source span.
    span: ByteSpan,
}

/// Private categories for minimal frontend failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrontendErrorKind {
    /// The required module header was absent.
    MissingModuleHeader,
    /// The quoted language version was not exactly `0.1`.
    UnsupportedLanguageVersion,
    /// Source is invalid or beyond the currently active minimal slice.
    Other,
}

impl FrontendError {
    /// Creates a missing-module-header failure at its insertion position.
    fn missing_module_header(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::MissingModuleHeader,
            span,
        }
    }

    /// Creates an unsupported-language-version failure over the quoted spelling.
    fn unsupported_language_version(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::UnsupportedLanguageVersion,
            span,
        }
    }

    /// Creates a bounded non-frozen failure for inactive or malformed syntax.
    fn other(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::Other,
            span,
        }
    }

    /// Returns the public failure detail without exposing private token state.
    pub(super) const fn detail(&self) -> crate::CompilationFailureDetail {
        match self.kind {
            FrontendErrorKind::MissingModuleHeader
            | FrontendErrorKind::UnsupportedLanguageVersion => {
                crate::CompilationFailureDetail::SyntaxRejected
            }
            FrontendErrorKind::Other => crate::CompilationFailureDetail::FrontendUnavailable,
        }
    }

    /// Converts only frozen frontend failures into stable public diagnostics.
    pub(super) fn into_diagnostics(self, source: SourceContentDigest) -> Vec<Diagnostic> {
        let code = match self.kind {
            FrontendErrorKind::MissingModuleHeader => MISSING_MODULE_HEADER,
            FrontendErrorKind::UnsupportedLanguageVersion => UNSUPPORTED_LANGUAGE_VERSION,
            FrontendErrorKind::Other => return Vec::new(),
        };
        let code = DiagnosticCode::new(code).expect("frozen diagnostic code must be valid ASCII");
        vec![Diagnostic::new(
            code,
            DiagnosticLayer::Syntax,
            DiagnosticSeverity::Error,
            SourceLocation::new(source, self.span),
            Vec::new(),
            false,
        )]
    }
}

/// Runs raw lexing, layout normalization, and parsing without ambient authority.
pub(super) fn parse(source: &[u8]) -> Result<ParsedUnit, FrontendError> {
    let raw = lexer::lex(source)?;
    let normalized = layout::normalize(raw)?;
    parser::parse(&normalized)
}

/// Creates a checked source span from validated in-memory indexes.
fn span(start: usize, end: usize) -> ByteSpan {
    ByteSpan::new(
        u64::try_from(start).unwrap_or(u64::MAX),
        u64::try_from(end).unwrap_or(u64::MAX),
    )
    .expect("frontend indexes must form an ordered span")
}

#[cfg(test)]
/// Tests for the complete private minimal frontend pipeline.
mod tests {
    use super::{FrontendErrorKind, TokenKind, layout, lexer, parse};

    /// Parses one exact captured source byte sequence.
    fn parse_source(source: &[u8]) -> Result<super::ParsedUnit, super::FrontendError> {
        parse(source)
    }

    #[test]
    /// Verifies raw tokens retain original spans and physical newline spellings.
    fn lexer_retains_minimal_fixture_spans_and_physical_newlines() {
        let source =
            include_bytes!("../../../../portable/spec/v0/fixtures/positive/minimal-core.neu");
        let tokens = lexer::lex(source).expect("frozen minimal fixture should lex");
        assert!(matches!(tokens[0].kind, TokenKind::Neu));
        assert_eq!(tokens[0].span.start(), 0);
        assert_eq!(tokens[0].span.end(), 3);
        assert!(tokens.iter().any(|token| matches!(
            token.kind,
            TokenKind::PhysicalLineEnd(super::PhysicalLineEnd::LineFeed)
        )));
    }

    #[test]
    /// Verifies layout creates semantic terminators and preserves EOF.
    fn layout_normalizes_minimal_fixture_line_ends() {
        let source = b"neu \"0.1\"\r\nmodule minimal\rnum answer = 42";
        let raw = lexer::lex(source).expect("source should lex");
        let tokens = layout::normalize(raw).expect("layout should normalize");
        assert_eq!(
            tokens
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::LineEnd))
                .count(),
            3
        );
        assert!(matches!(
            tokens.last().map(|token| &token.kind),
            Some(TokenKind::EndOfFile)
        ));
    }

    #[test]
    /// Verifies the positive frozen fixture produces its expected private spans.
    fn parser_matches_the_minimal_frozen_oracle() {
        let source =
            include_bytes!("../../../../portable/spec/v0/fixtures/positive/minimal-core.neu");
        let unit = parse_source(source).expect("frozen minimal fixture should parse");
        assert_eq!((unit.module.span.start(), unit.module.span.end()), (10, 24));
        assert_eq!(
            (unit.binding.span.start(), unit.binding.span.end()),
            (26, 41)
        );
        assert_eq!(
            (unit.binding.type_span.start(), unit.binding.type_span.end()),
            (26, 29)
        );
        assert_eq!(
            (unit.binding.name_span.start(), unit.binding.name_span.end()),
            (30, 36)
        );
        assert_eq!(
            (
                unit.binding.value_span.start(),
                unit.binding.value_span.end()
            ),
            (39, 41)
        );
        assert_eq!(unit.module.name, "minimal");
        assert_eq!(unit.binding.name, "answer");
        assert_eq!(unit.binding.number, "42");
    }

    #[test]
    /// Verifies the missing-module fixture matches its frozen diagnostic span.
    fn parser_matches_the_missing_module_frozen_oracle() {
        let source = include_bytes!(
            "../../../../portable/spec/v0/fixtures/negative/missing-module-header.neu"
        );
        let error = parse_source(source).expect_err("missing module must fail");
        assert_eq!(error.kind, FrontendErrorKind::MissingModuleHeader);
        assert_eq!((error.span.start(), error.span.end()), (10, 10));
    }

    #[test]
    /// Verifies the unsupported-version fixture matches its frozen diagnostic span.
    fn parser_matches_the_unsupported_version_frozen_oracle() {
        let source = include_bytes!(
            "../../../../portable/spec/v0/fixtures/negative/unsupported-language-version.neu"
        );
        let error = parse_source(source).expect_err("unsupported version must fail");
        assert_eq!(error.kind, FrontendErrorKind::UnsupportedLanguageVersion);
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
            (unit.module.name, unit.binding.name, unit.binding.number)
        });
        assert!(logical.windows(2).all(|pair| pair[0] == pair[1]));
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
        let malformed: [&[u8]; 7] = [
            b"\xff",
            b"neu \"0.1\"\0\nmodule minimal\nnum answer = 42\n",
            b"neu \"0.1\"\n\xef\xbb\xbfmodule minimal\nnum answer = 42\n",
            b"neu 0.1\nmodule minimal\nnum answer = 42\n",
            b"neu \"0.1\"\nmodule\nnum answer = 42\n",
            b"neu \"0.1\"\nmodule minimal\nnum answer = 4.2\n",
            b"neu \"0.1\"\nmodule minimal\nnum answer = --42\n",
        ];
        for source in malformed {
            assert!(parse_source(source).is_err());
        }
    }
}
