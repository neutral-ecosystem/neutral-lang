// SPDX-License-Identifier: Apache-2.0

//! Private orchestration for captured source, trivia, layout, and parsing.

use neutral_core::ByteSpan;

pub(crate) mod expression_codes;
mod format_style;
mod graph_syntax;
mod lexer;
mod project_semantics;

pub use project_semantics::{
    ExpressionCompilationCache, ExpressionCompileFailure, ProjectCacheLimits, ProjectCacheStats,
    compile_expression_project, format_expression_source,
};

pub(crate) use graph_syntax::scan_graph_source_bounded;
pub(crate) use graph_syntax::{GraphSourceSyntax, GraphSyntaxErrorKind};

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

impl neutral_core::allocation::TryClone for Token {
    /// Copies owned token spellings fallibly; punctuation and source spans never allocate.
    fn try_clone(&self) -> Result<Self, neutral_core::allocation::AllocationError> {
        let kind = match &self.kind {
            TokenKind::Identifier(s) => TokenKind::Identifier(s.try_clone()?),
            TokenKind::ProtectedName(s) => TokenKind::ProtectedName(s.try_clone()?),
            TokenKind::Number(s) => TokenKind::Number(s.try_clone()?),
            TokenKind::StringLiteral(s) => TokenKind::StringLiteral(DecodedString {
                value: s.value.try_clone()?,
                had_escape: s.had_escape,
            }),
            // All remaining token variants retain only nonallocating punctuation/newline facts.
            kind @ (TokenKind::Neu
            | TokenKind::Module
            | TokenKind::Use
            | TokenKind::Record
            | TokenKind::List
            | TokenKind::RefType
            | TokenKind::RefValue
            | TokenKind::Num
            | TokenKind::StringType
            | TokenKind::BoolType
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
            | TokenKind::Equals
            | TokenKind::Question
            | TokenKind::OpenBrace
            | TokenKind::CloseBrace
            | TokenKind::Colon
            | TokenKind::DoubleColon
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::OpenBracket
            | TokenKind::CloseBracket
            | TokenKind::OpenParen
            | TokenKind::CloseParen
            | TokenKind::Comma
            | TokenKind::PhysicalLineEnd(_)
            | TokenKind::EndOfFile) => kind.clone(),
        };
        Ok(Self {
            kind,
            span: self.span,
        })
    }
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
    /// A symbol cannot be tokenized.
    UnsupportedSymbol,
    /// A block comment reached EOF without closing.
    UnterminatedBlockComment,
    /// A string contains an invalid escape, scalar or raw control.
    InvalidStringLiteral,
    /// A string reached a raw newline or EOF without closing.
    UnterminatedStringLiteral,
    /// Fallible lexical retention or a decoded-string bound failed.
    RecordLimitExceeded,
    /// Malformed encoding or cancellation prevented lexical retention.
    Other,
}

impl FrontendError {
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
