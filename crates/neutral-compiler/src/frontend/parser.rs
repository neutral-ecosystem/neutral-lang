// SPDX-License-Identifier: Apache-2.0

//! Recovery-free parser for the Stage 2 minimal document shape.

use super::{
    FrontendError, LexedSource, ParsedBinding, ParsedModule, ParsedUnit, Token, TokenKind, span,
};
use crate::language::names;
use neutral_core::ByteSpan;

/// Parses one exact minimal document or returns no private syntax model.
pub(super) fn parse(source: LexedSource) -> Result<ParsedUnit, FrontendError> {
    let mut parser = Parser {
        tokens: &source.tokens,
        index: 0,
    };
    let mut unit = parser.parse_unit()?;
    unit.trivia = source.trivia;
    Ok(unit)
}

/// Cursor over normalized compiler-private tokens.
struct Parser<'a> {
    /// Complete normalized token sequence ending in EOF.
    tokens: &'a [Token],
    /// Index of the next token to inspect.
    index: usize,
}

impl Parser<'_> {
    /// Parses the language header, module header, and one minimal binding.
    fn parse_unit(&mut self) -> Result<ParsedUnit, FrontendError> {
        let language_start = self.expect_simple(&TokenKind::Neu)?.span.start();
        let version = self.next().ok_or_else(|| self.other_here())?;
        let version_span = version.span;
        match &version.kind {
            TokenKind::StringLiteral(value) if value == "0.1" => {}
            TokenKind::StringLiteral(_) => {
                return Err(FrontendError::unsupported_language_version(version_span));
            }
            _ => return Err(FrontendError::other(version_span)),
        }
        let language_end = self.expect_simple(&TokenKind::LineEnd)?.span.start();
        let language_header_span = ByteSpan::new(language_start, language_end)
            .expect("ordered header tokens must form a valid span");
        let module_insertion = self
            .previous()
            .map_or(language_end, |token| token.span.end());
        self.skip_line_ends();
        if !self.at(&TokenKind::Module) {
            return Err(FrontendError::missing_module_header(
                ByteSpan::new(module_insertion, module_insertion)
                    .expect("equal insertion offsets must form a valid span"),
            ));
        }

        let module = self.parse_module()?;
        self.skip_line_ends();
        let binding = self.parse_binding()?;
        self.skip_line_ends();
        self.expect_simple(&TokenKind::EndOfFile)?;

        Ok(ParsedUnit {
            language_header_span,
            version_span,
            module,
            binding,
            trivia: Vec::new(),
        })
    }

    /// Parses the exact `module identifier LINE_END` header shape.
    fn parse_module(&mut self) -> Result<ParsedModule, FrontendError> {
        let start = self.expect_simple(&TokenKind::Module)?.span.start();
        let name_token = self.next().ok_or_else(|| self.other_here())?;
        let name = identifier_spelling(&name_token)
            .ok_or_else(|| FrontendError::other(name_token.span))?;
        let end = name_token.span.end();
        self.expect_simple(&TokenKind::LineEnd)?;
        Ok(ParsedModule {
            name,
            span: ByteSpan::new(start, end).expect("ordered module tokens must form a valid span"),
            name_span: name_token.span,
        })
    }

    /// Parses the exact `num identifier = digits LINE_END` binding shape.
    fn parse_binding(&mut self) -> Result<ParsedBinding, FrontendError> {
        let type_span = self.expect_simple(&TokenKind::Num)?.span;
        let name_token = self.next().ok_or_else(|| self.other_here())?;
        let name_span = name_token.span;
        let name = identifier_spelling(&name_token)
            .ok_or_else(|| FrontendError::other(name_token.span))?;
        self.expect_simple(&TokenKind::Equals)?;
        let value_token = self.next().ok_or_else(|| self.other_here())?;
        let value_span = value_token.span;
        let number = match &value_token.kind {
            TokenKind::Number(value) => value.clone(),
            _ => return Err(FrontendError::other(value_token.span)),
        };
        let end = value_token.span.end();
        self.expect_simple(&TokenKind::LineEnd)?;
        Ok(ParsedBinding {
            name,
            number,
            span: ByteSpan::new(type_span.start(), end)
                .expect("ordered binding tokens must form a valid span"),
            type_span,
            name_span,
            value_span,
        })
    }

    /// Consumes and returns one token matching a value-free token category.
    fn expect_simple(&mut self, expected: &TokenKind) -> Result<Token, FrontendError> {
        let token = self.next().ok_or_else(|| self.other_here())?;
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(expected) {
            Ok(token)
        } else {
            Err(FrontendError::other(token.span))
        }
    }

    /// Returns whether the next token has the requested value-free category.
    fn at(&self, expected: &TokenKind) -> bool {
        self.peek().is_some_and(|token| {
            std::mem::discriminant(&token.kind) == std::mem::discriminant(expected)
        })
    }

    /// Consumes blank semantic line ends between complete constructs.
    fn skip_line_ends(&mut self) {
        while self.at(&TokenKind::LineEnd) {
            self.index += 1;
        }
    }

    /// Returns the next token without consuming it.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    /// Consumes and returns the next token.
    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    /// Returns the previously consumed token, when one exists.
    fn previous(&self) -> Option<&Token> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    /// Creates a bounded generic failure at the current token or EOF position.
    fn other_here(&self) -> FrontendError {
        let location = self
            .peek()
            .map(|token| token.span)
            .or_else(|| self.tokens.last().map(|token| token.span))
            .unwrap_or_else(|| span(0, 0));
        FrontendError::other(location)
    }
}

/// Returns a spelling from any token that can occur in an identifier position.
fn identifier_spelling(token: &Token) -> Option<String> {
    match &token.kind {
        TokenKind::Identifier(value) | TokenKind::ProtectedName(value) => Some(value.clone()),
        TokenKind::Neu => Some(names::NEU.to_owned()),
        TokenKind::Module => Some(names::MODULE.to_owned()),
        TokenKind::Num => Some(names::NUM.to_owned()),
        _ => None,
    }
}
