// SPDX-License-Identifier: Apache-2.0

//! Private orchestration for captured source, trivia, layout, and parsing.

use crate::diagnostics;
use neutral_core::{
    ByteSpan, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity, SourceContentDigest,
    SourceLocation, StructuralLimits,
};

mod layout;
mod lexer;
mod parser;

/// Raw lexing result with nonsemantic trivia kept separate from parser tokens.
#[derive(Clone, Debug, Eq, PartialEq)]
struct LexedSource {
    /// Semantic and physical-line tokens in original byte order.
    tokens: Vec<Token>,
    /// Exact source trivia retained privately for later formatter work.
    trivia: Vec<Trivia>,
}

/// One exact nonsemantic source region.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Trivia {
    /// Trivia category.
    kind: TriviaKind,
    /// Exact half-open span in captured source bytes.
    span: ByteSpan,
}

/// Compiler-private source trivia categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TriviaKind {
    /// One or more spaces or horizontal tabs.
    HorizontalWhitespace,
    /// A `//` comment excluding its physical line ending.
    LineComment,
    /// A non-nesting `/* ... */` comment.
    BlockComment,
}

/// A raw or normalized token retained only inside the compiler frontend.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    /// Token category and safe decoded value, when applicable.
    kind: TokenKind,
    /// Exact half-open span in the original captured bytes.
    span: ByteSpan,
}

/// Token categories required by the active source and scalar slices.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TokenKind {
    /// The `neu` document-header keyword.
    Neu,
    /// The `module` header keyword.
    Module,
    /// The `use` vocabulary-requirement keyword.
    Use,
    /// The `record` nominal-type declaration keyword.
    Record,
    /// The `List` invariant generic type constructor.
    List,
    /// The `Ref` invariant identity-reference type constructor.
    RefType,
    /// The `ref` identity-reference value constructor.
    RefValue,
    /// The `num` core-type keyword.
    Num,
    /// The `string` core-type keyword.
    StringType,
    /// The `bool` core-type keyword.
    BoolType,
    /// The `true` Boolean literal.
    True,
    /// The `false` Boolean literal.
    False,
    /// The `null` explicit null literal.
    Null,
    /// A protected core spelling appearing where an identifier can be diagnosed.
    ProtectedName(String),
    /// An ASCII identifier retained for later semantic validation.
    Identifier(String),
    /// A raw, unescaped string body used only for the version header.
    StringLiteral(DecodedString),
    /// A digits-only minimal numeric spelling retained for later semantics.
    Number(String),
    /// The binding initializer delimiter.
    Equals,
    /// Postfix outer-nullability delimiter.
    Question,
    /// Record declaration or contextual-value opening delimiter.
    OpenBrace,
    /// Record declaration or contextual-value closing delimiter.
    CloseBrace,
    /// Record value field-name separator.
    Colon,
    /// Vocabulary qualification delimiter.
    DoubleColon,
    /// Generic type-argument opening delimiter.
    Less,
    /// Generic type-argument closing delimiter.
    Greater,
    /// List value opening delimiter.
    OpenBracket,
    /// List value closing delimiter.
    CloseBracket,
    /// Identity-reference target opening delimiter.
    OpenParen,
    /// Identity-reference target closing delimiter.
    CloseParen,
    /// Required record field terminator.
    Comma,
    /// An original physical newline before layout normalization.
    PhysicalLineEnd(PhysicalLineEnd),
    /// A semantic declaration/header terminator.
    LineEnd,
    /// End of captured source.
    EndOfFile,
}

/// A decoded string token plus spelling information needed by canonical headers.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedString {
    /// Exact decoded Unicode scalar sequence.
    value: String,
    /// Whether the source spelling contained any escape.
    had_escape: bool,
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
    /// Optional captured vocabulary requirement.
    pub(super) vocabulary_use: Option<ParsedVocabularyUse>,
    /// Parsed root declarations in source order.
    pub(super) declarations: Vec<ParsedDeclaration>,
    /// Exact compiler-private trivia, never lowered into logical IR.
    trivia: Vec<Trivia>,
}

/// One compiler-private captured vocabulary requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedVocabularyUse {
    /// Required uppercase vocabulary identity.
    pub(super) name: String,
    /// Complete `use` declaration span.
    pub(super) span: ByteSpan,
    /// Exact vocabulary-name span.
    pub(super) name_span: ByteSpan,
}

/// One compiler-private root declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ParsedDeclaration {
    /// One nominal record schema declaration.
    Record(ParsedRecord),
    /// One immutable typed value binding.
    Binding(ParsedBinding),
}

impl ParsedDeclaration {
    /// Returns the declared root name.
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Record(record) => &record.name,
            Self::Binding(binding) => &binding.name,
        }
    }

    /// Returns the exact root-name span.
    pub(super) const fn name_span(&self) -> ByteSpan {
        match self {
            Self::Record(record) => record.name_span,
            Self::Binding(binding) => binding.name_span,
        }
    }
}

/// A compiler-private nominal record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedRecord {
    /// Uppercase-leading nominal type name.
    pub(super) name: String,
    /// Required fields in source order.
    pub(super) fields: Vec<ParsedRecordField>,
    /// Complete declaration span.
    pub(super) span: ByteSpan,
    /// Exact record-name span.
    pub(super) name_span: ByteSpan,
    /// Exact braces-and-fields span.
    pub(super) body_span: ByteSpan,
}

/// One compiler-private required record field contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedRecordField {
    /// Explicit parsed field type.
    pub(super) declared_type: ParsedType,
    /// Required field name.
    pub(super) name: String,
    /// Optional parsed closed-default candidate.
    pub(super) default_value: Option<ParsedValue>,
    /// Complete field span.
    pub(super) span: ByteSpan,
    /// Exact type span.
    pub(super) type_span: ByteSpan,
    /// Exact field-name span.
    pub(super) name_span: ByteSpan,
    /// Exact default value span, when present.
    pub(super) default_span: Option<ByteSpan>,
}

impl ParsedUnit {
    /// Returns the number of privately retained trivia regions.
    pub(super) fn trivia_count(&self) -> usize {
        self.trivia.len()
    }
}

/// A compiler-private parsed module header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedModule {
    /// Module name spelling retained for Step 4 semantic validation.
    pub(super) name: String,
    /// Exact span of the complete module header.
    pub(super) span: ByteSpan,
    /// Exact span of the module name.
    pub(super) name_span: ByteSpan,
}

/// A compiler-private parsed scalar binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedBinding {
    /// Binding name spelling retained for Step 4 semantic validation.
    pub(super) name: String,
    /// Explicit parsed scalar type.
    pub(super) declared_type: ParsedType,
    /// Explicit parsed scalar value.
    pub(super) value: ParsedValue,
    /// Exact span of the complete binding.
    pub(super) span: ByteSpan,
    /// Exact span of the explicit `num` type.
    pub(super) type_span: ByteSpan,
    /// Exact span of the binding name.
    pub(super) name_span: ByteSpan,
    /// Exact span of the numeric source value.
    pub(super) value_span: ByteSpan,
}

/// Compiler-private type syntax active through Slice 5.2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ParsedType {
    /// Exact numeric type.
    Num,
    /// Unicode scalar-sequence string type.
    String,
    /// Boolean type.
    Bool,
    /// Unresolved user nominal record name.
    Record(String),
    /// Qualified nominal type owned by the captured vocabulary.
    VocabularyRecord {
        /// Required vocabulary namespace.
        namespace: String,
        /// Required vocabulary-owned nominal type.
        name: String,
    },
    /// Exactly one outer nullable layer around a supported type.
    Nullable(Box<ParsedType>),
    /// Invariant ordered list element type.
    List(Box<ParsedType>),
    /// Typed document-local identity reference target type.
    Ref(Box<ParsedType>),
}

/// Compiler-private contextual value syntax active through Slice 5.2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ParsedValue {
    /// Minimal digits-only number spelling.
    Number(String),
    /// Decoded Unicode scalar sequence.
    String(String),
    /// Exact Boolean value.
    Boolean(bool),
    /// Explicit null value.
    Null,
    /// Contextual record value with explicit fields.
    Record(Vec<ParsedValueField>),
    /// Ordered list value with contextually typed items.
    List(Vec<ParsedListItem>),
    /// Unqualified name candidate, accepted only for later semantic rejection.
    Name(String),
    /// Identity-only edge to one unqualified binding name.
    Reference {
        /// Unresolved target binding name.
        target: String,
        /// Exact target-name source span.
        target_span: ByteSpan,
    },
}

/// One compiler-private list item with its exact source ownership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedListItem {
    /// Recursively parsed contextual item value.
    pub(super) value: ParsedValue,
    /// Exact item value span.
    pub(super) span: ByteSpan,
}

/// One compiler-private explicit contextual-record value field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedValueField {
    /// Explicit field name.
    pub(super) name: String,
    /// Recursively parsed contextual value.
    pub(super) value: ParsedValue,
    /// Complete `name: value` span.
    pub(super) span: ByteSpan,
    /// Exact field-name span.
    pub(super) name_span: ByteSpan,
    /// Exact field-value span.
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

/// Private categories for active frontend failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrontendErrorKind {
    /// The required module header was absent.
    MissingModuleHeader,
    /// The quoted language version was not exactly `0.1`.
    UnsupportedLanguageVersion,
    /// A malformed token or raw-newline boundary.
    MalformedBoundary,
    /// A symbol excluded by the frozen v0 grammar.
    UnsupportedSymbol,
    /// A block comment reached EOF without closing.
    UnterminatedBlockComment,
    /// A string contained an invalid escape, scalar, or raw control.
    InvalidStringLiteral,
    /// A string reached a raw newline or EOF without closing.
    UnterminatedStringLiteral,
    /// A record/declaration/depth limit was exceeded during bounded parsing.
    RecordLimitExceeded,
    /// A list item/depth/traversal limit was exceeded during bounded parsing.
    ListLimitExceeded,
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

    /// Creates a stable malformed-boundary failure.
    fn malformed_boundary(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::MalformedBoundary,
            span,
        }
    }

    /// Creates a stable unsupported-symbol failure.
    fn unsupported_symbol(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::UnsupportedSymbol,
            span,
        }
    }

    /// Creates a stable unterminated-block-comment failure.
    fn unterminated_block_comment(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::UnterminatedBlockComment,
            span,
        }
    }

    /// Creates a stable invalid-string-literal failure.
    fn invalid_string_literal(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::InvalidStringLiteral,
            span,
        }
    }

    /// Creates a stable unterminated-string-literal failure.
    fn unterminated_string_literal(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::UnterminatedStringLiteral,
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

    /// Creates a stable record-structure resource-limit failure.
    fn record_limit_exceeded(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::RecordLimitExceeded,
            span,
        }
    }

    /// Creates a stable list-structure resource-limit failure.
    fn list_limit_exceeded(span: ByteSpan) -> Self {
        Self {
            kind: FrontendErrorKind::ListLimitExceeded,
            span,
        }
    }

    /// Returns the broad public failure class.
    pub(super) const fn class(&self) -> neutral_core::ResultClass {
        match self.kind {
            FrontendErrorKind::RecordLimitExceeded | FrontendErrorKind::ListLimitExceeded => {
                neutral_core::ResultClass::Resource
            }
            _ => neutral_core::ResultClass::Syntax,
        }
    }

    /// Returns the public failure detail without exposing private token state.
    pub(super) const fn detail(&self) -> crate::CompilationFailureDetail {
        match self.kind {
            FrontendErrorKind::MissingModuleHeader
            | FrontendErrorKind::UnsupportedLanguageVersion
            | FrontendErrorKind::MalformedBoundary
            | FrontendErrorKind::UnsupportedSymbol
            | FrontendErrorKind::UnterminatedBlockComment
            | FrontendErrorKind::InvalidStringLiteral
            | FrontendErrorKind::UnterminatedStringLiteral => {
                crate::CompilationFailureDetail::SyntaxRejected
            }
            FrontendErrorKind::RecordLimitExceeded | FrontendErrorKind::ListLimitExceeded => {
                crate::CompilationFailureDetail::ResourceLimitExceeded
            }
            FrontendErrorKind::Other => crate::CompilationFailureDetail::FrontendUnavailable,
        }
    }

    /// Converts only frozen frontend failures into stable public diagnostics.
    pub(super) fn into_diagnostics(self, source: SourceContentDigest) -> Vec<Diagnostic> {
        let code = match self.kind {
            FrontendErrorKind::MissingModuleHeader => diagnostics::MISSING_MODULE_HEADER,
            FrontendErrorKind::UnsupportedLanguageVersion => {
                diagnostics::UNSUPPORTED_LANGUAGE_VERSION
            }
            FrontendErrorKind::MalformedBoundary => diagnostics::MALFORMED_BOUNDARY,
            FrontendErrorKind::UnsupportedSymbol => diagnostics::UNSUPPORTED_SYMBOL,
            FrontendErrorKind::UnterminatedBlockComment => diagnostics::UNTERMINATED_BLOCK_COMMENT,
            FrontendErrorKind::InvalidStringLiteral => diagnostics::INVALID_STRING_LITERAL,
            FrontendErrorKind::UnterminatedStringLiteral => {
                diagnostics::UNTERMINATED_STRING_LITERAL
            }
            FrontendErrorKind::RecordLimitExceeded => diagnostics::RECORD_LIMIT_EXCEEDED,
            FrontendErrorKind::ListLimitExceeded => diagnostics::LIST_LIMIT_EXCEEDED,
            FrontendErrorKind::Other => return Vec::new(),
        };
        let code = DiagnosticCode::new(code).expect("frozen diagnostic code must be valid ASCII");
        vec![Diagnostic::new(
            code,
            if matches!(
                self.kind,
                FrontendErrorKind::RecordLimitExceeded | FrontendErrorKind::ListLimitExceeded
            ) {
                DiagnosticLayer::Resource
            } else {
                DiagnosticLayer::Syntax
            },
            DiagnosticSeverity::Error,
            SourceLocation::new(source, self.span),
            Vec::new(),
            false,
        )]
    }
}

/// Runs raw lexing, layout normalization, and parsing without ambient authority.
pub(super) fn parse(source: &[u8], limits: StructuralLimits) -> Result<ParsedUnit, FrontendError> {
    let raw = lexer::lex(source)?;
    let normalized = layout::normalize(raw)?;
    parser::parse(normalized, limits)
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
/// Tests for the complete private active frontend pipeline.
mod tests {
    use super::{
        FrontendErrorKind, ParsedValue, PhysicalLineEnd, TokenKind, TriviaKind, layout, lexer,
        parse,
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
            "../../../../portable/spec/v0/fixtures/positive/syntax/minimal-core.neu"
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
    /// Verifies the positive frozen fixture produces its expected private spans.
    fn parser_matches_the_minimal_frozen_oracle() {
        let source = include_bytes!(
            "../../../../portable/spec/v0/fixtures/positive/syntax/minimal-core.neu"
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
            "../../../../portable/spec/v0/fixtures/negative/syntax/missing-module-header.neu"
        );
        let error = parse_source(source).expect_err("missing module must fail");
        assert_eq!(error.kind, FrontendErrorKind::MissingModuleHeader);
        assert_eq!((error.span.start(), error.span.end()), (10, 10));
    }

    #[test]
    /// Verifies the unsupported-version fixture matches its frozen diagnostic span.
    fn parser_matches_the_unsupported_version_frozen_oracle() {
        let source = include_bytes!(
            "../../../../portable/spec/v0/fixtures/negative/syntax/unsupported-language-version.neu"
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
        let source =
            b"neu \"0.1\"\nmodule scalar\nstring message = \"a\\n\\0\\u{1f642}\xc3\xa9\"\n";
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
        let malformed: [&[u8]; 6] = [
            b"\xff",
            b"neu \"0.1\"\0\nmodule minimal\nnum answer = 42\n",
            b"neu \"0.1\"\n\xef\xbb\xbfmodule minimal\nnum answer = 42\n",
            b"neu 0.1\nmodule minimal\nnum answer = 42\n",
            b"neu \"0.1\"\nmodule\nnum answer = 42\n",
            b"neu \"0.1\"\nmodule minimal\nnum answer = --42\n",
        ];
        for source in malformed {
            assert!(parse_source(source).is_err());
        }
    }
}
