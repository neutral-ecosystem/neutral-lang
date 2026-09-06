// SPDX-License-Identifier: Apache-2.0

//! Private orchestration for captured source, trivia, layout, and parsing.

use crate::diagnostics;
use neutral_core::{
    ByteSpan, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity, SourceContentDigest,
    SourceLocation, StructuralLimits,
};

mod format_style;
mod formatter;
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

/// Formats one successfully parsed unit without exposing private syntax types.
pub(super) fn format_source(source: &[u8], unit: &ParsedUnit) -> Vec<u8> {
    formatter::format_source(source, unit)
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
#[path = "../../tests/frontend/mod.rs"]
mod tests;
