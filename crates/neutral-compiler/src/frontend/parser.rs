// SPDX-License-Identifier: Apache-2.0

//! Recovery-free parser for active scalar, record, default, and list shapes.

use super::{
    FrontendError, LexedSource, ParsedBinding, ParsedDeclaration, ParsedListItem, ParsedModule,
    ParsedRecord, ParsedRecordField, ParsedType, ParsedUnit, ParsedValue, ParsedValueField,
    ParsedVocabularyUse, Token, TokenKind, span,
};
use crate::language::names;
use neutral_core::{
    ByteSpan, StructuralLimits,
    profile::{LanguageProfile, ProfileAvailability, language_profile},
};

#[cfg(test)]
#[path = "../../tests/parser/mod.rs"]
mod tests;

/// Parses one active document or returns no private syntax model.
pub(super) fn parse(
    source: LexedSource,
    limits: StructuralLimits,
) -> Result<ParsedUnit, FrontendError> {
    let mut parser = Parser {
        tokens: &source.tokens,
        index: 0,
        limits,
        value_nodes: 0,
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
    /// Captured deterministic structural limits.
    limits: StructuralLimits,
    /// Total recursively parsed values for traversal bounding.
    value_nodes: u64,
}

impl Parser<'_> {
    /// Parses headers followed by all root declarations.
    fn parse_unit(&mut self) -> Result<ParsedUnit, FrontendError> {
        let language_start = self.expect_simple(&TokenKind::Neu)?.span.start();
        let version = self.next().ok_or_else(|| self.other_here())?;
        let version_span = version.span;
        match &version.kind {
            TokenKind::StringLiteral(value) if !value.had_escape => {
                let Some(profile) = LanguageProfile::from_source_version(&value.value) else {
                    return Err(FrontendError::unsupported_language_version(version_span));
                };
                if language_profile(profile).availability() == ProfileAvailability::Unavailable {
                    return Err(FrontendError::unavailable_language_profile(version_span));
                }
                debug_assert_eq!(profile, LanguageProfile::V0_1);
                debug_assert_eq!(value.value, names::SOURCE_LANGUAGE_VERSION);
            }
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
        let vocabulary_use = if self.at(&TokenKind::Use) {
            let parsed = Some(self.parse_vocabulary_use()?);
            self.skip_line_ends();
            parsed
        } else {
            None
        };
        let mut declarations = Vec::new();
        while !self.at(&TokenKind::EndOfFile) {
            Self::ensure_capacity(
                declarations.len(),
                self.limits.declarations(),
                self.peek().map_or_else(|| span(0, 0), |token| token.span),
            )?;
            let declaration = if self.at(&TokenKind::Record) {
                ParsedDeclaration::Record(self.parse_record()?)
            } else {
                ParsedDeclaration::Binding(self.parse_binding()?)
            };
            declarations.push(declaration);
            self.skip_line_ends();
        }
        self.expect_simple(&TokenKind::EndOfFile)?;

        Ok(ParsedUnit {
            language_header_span,
            version_span,
            module,
            vocabulary_use,
            declarations,
            trivia: Vec::new(),
        })
    }

    /// Parses the exact `use Vocabulary LINE_END` requirement.
    fn parse_vocabulary_use(&mut self) -> Result<ParsedVocabularyUse, FrontendError> {
        let start = self.expect_simple(&TokenKind::Use)?.span.start();
        let name_token = self.next().ok_or_else(|| self.other_here())?;
        let name = identifier_spelling(&name_token)
            .ok_or_else(|| FrontendError::other(name_token.span))?;
        let end = name_token.span.end();
        self.expect_simple(&TokenKind::LineEnd)?;
        Ok(ParsedVocabularyUse {
            name,
            span: ByteSpan::new(start, end).expect("ordered use tokens must form a valid span"),
            name_span: name_token.span,
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

    /// Parses one nominal record declaration with required fields.
    fn parse_record(&mut self) -> Result<ParsedRecord, FrontendError> {
        let start = self.expect_simple(&TokenKind::Record)?.span.start();
        let name_token = self.next().ok_or_else(|| self.other_here())?;
        let name = identifier_spelling(&name_token)
            .ok_or_else(|| FrontendError::other(name_token.span))?;
        let open = self.expect_simple(&TokenKind::OpenBrace)?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::CloseBrace) {
            Self::ensure_capacity(
                fields.len(),
                self.limits.record_fields(),
                self.peek().map_or(open.span, |token| token.span),
            )?;
            fields.push(self.parse_record_field()?);
        }
        let close = self.expect_simple(&TokenKind::CloseBrace)?;
        self.expect_simple(&TokenKind::LineEnd)?;
        Ok(ParsedRecord {
            name,
            fields,
            span: ByteSpan::new(start, close.span.end())
                .expect("ordered record tokens must form a valid span"),
            name_span: name_token.span,
            body_span: ByteSpan::new(open.span.start(), close.span.end())
                .expect("ordered record body tokens must form a valid span"),
        })
    }

    /// Parses one required or closed-defaulted record field.
    fn parse_record_field(&mut self) -> Result<ParsedRecordField, FrontendError> {
        let (declared_type, type_span) = self.parse_type()?;
        let name_token = self.next().ok_or_else(|| self.other_here())?;
        let name = identifier_spelling(&name_token)
            .ok_or_else(|| FrontendError::malformed_boundary(name_token.span))?;
        let (default_value, default_span) = if self.at(&TokenKind::Equals) {
            self.next().expect("looked-ahead equals token must exist");
            let start = self.peek().ok_or_else(|| self.other_here())?.span.start();
            let value = self.parse_value(0)?;
            let end = self
                .previous()
                .expect("a parsed default consumes at least one token")
                .span
                .end();
            (
                Some(value),
                Some(
                    ByteSpan::new(start, end)
                        .expect("ordered default tokens must form a valid span"),
                ),
            )
        } else {
            (None, None)
        };
        let comma = self.expect_field_delimiter(&TokenKind::Comma)?;
        Ok(ParsedRecordField {
            declared_type,
            name,
            default_value,
            span: ByteSpan::new(type_span.start(), comma.span.end())
                .expect("ordered field tokens must form a valid span"),
            type_span,
            name_span: name_token.span,
            default_span,
        })
    }

    /// Parses one immutable typed binding and contextual value.
    fn parse_binding(&mut self) -> Result<ParsedBinding, FrontendError> {
        let (declared_type, type_span) = self.parse_type()?;
        let name_token = self.next().ok_or_else(|| self.other_here())?;
        let name_span = name_token.span;
        let name = identifier_spelling(&name_token)
            .ok_or_else(|| FrontendError::other(name_token.span))?;
        self.expect_simple(&TokenKind::Equals)?;
        let value_start = self.peek().ok_or_else(|| self.other_here())?.span.start();
        let value = self.parse_value(0)?;
        let value_end = self
            .previous()
            .expect("a parsed value consumes at least one token")
            .span
            .end();
        let value_span = ByteSpan::new(value_start, value_end)
            .expect("ordered value tokens must form a valid span");
        self.expect_simple(&TokenKind::LineEnd)?;
        Ok(ParsedBinding {
            name,
            declared_type,
            value,
            span: ByteSpan::new(type_span.start(), value_end)
                .expect("ordered binding tokens must form a valid span"),
            type_span,
            name_span,
            value_span,
        })
    }

    /// Parses one scalar or nominal type with optional outer nullability.
    fn parse_type(&mut self) -> Result<(ParsedType, ByteSpan), FrontendError> {
        let type_token = self.next().ok_or_else(|| self.other_here())?;
        let mut type_span = type_token.span;
        let mut declared_type = match type_token.kind {
            TokenKind::Num => ParsedType::Num,
            TokenKind::StringType => ParsedType::String,
            TokenKind::BoolType => ParsedType::Bool,
            TokenKind::List => {
                self.expect_simple(&TokenKind::Less)?;
                let (inner, _) = self.parse_type()?;
                let close = self.expect_simple(&TokenKind::Greater)?;
                type_span = ByteSpan::new(type_span.start(), close.span.end())
                    .expect("ordered list type tokens must form a valid span");
                ParsedType::List(Box::new(inner))
            }
            TokenKind::RefType => {
                self.expect_simple(&TokenKind::Less)?;
                let (inner, _) = self.parse_type()?;
                let close = self.expect_simple(&TokenKind::Greater)?;
                type_span = ByteSpan::new(type_span.start(), close.span.end())
                    .expect("identity-reference type tokens must form a valid span");
                ParsedType::Ref(Box::new(inner))
            }
            TokenKind::Identifier(name) | TokenKind::ProtectedName(name) => {
                if self.at(&TokenKind::DoubleColon) {
                    self.next().expect("looked-ahead qualifier must exist");
                    let target = self.next().ok_or_else(|| self.other_here())?;
                    let target_name = identifier_spelling(&target)
                        .ok_or_else(|| FrontendError::other(target.span))?;
                    type_span = ByteSpan::new(type_span.start(), target.span.end())
                        .expect("ordered qualified type tokens must form a valid span");
                    ParsedType::VocabularyRecord {
                        namespace: name,
                        name: target_name,
                    }
                } else {
                    ParsedType::Record(name)
                }
            }
            _ => return Err(FrontendError::other(type_span)),
        };
        if self.at(&TokenKind::Question) {
            let question = self
                .next()
                .expect("looked-ahead nullable delimiter must exist");
            type_span = ByteSpan::new(type_span.start(), question.span.end())
                .expect("adjacent type tokens must form a valid span");
            declared_type = ParsedType::Nullable(Box::new(declared_type));
            if self.at(&TokenKind::Question) {
                let duplicate = self
                    .next()
                    .expect("looked-ahead duplicate nullable delimiter must exist");
                return Err(FrontendError::malformed_boundary(duplicate.span));
            }
        }
        Ok((declared_type, type_span))
    }

    /// Parses one scalar or recursively contextual record value.
    fn parse_value(&mut self, depth: u64) -> Result<ParsedValue, FrontendError> {
        let token = self.next().ok_or_else(|| self.other_here())?;
        self.value_nodes = self.value_nodes.saturating_add(1);
        if self.value_nodes > self.limits.traversal_nodes() {
            return Err(FrontendError::list_limit_exceeded(token.span));
        }
        match token.kind {
            TokenKind::Number(value) => Ok(ParsedValue::Number(value)),
            TokenKind::StringLiteral(value) => Ok(ParsedValue::String(value.value)),
            TokenKind::True => Ok(ParsedValue::Boolean(true)),
            TokenKind::False => Ok(ParsedValue::Boolean(false)),
            TokenKind::Null => Ok(ParsedValue::Null),
            TokenKind::OpenBrace => self.parse_record_value(token.span, depth),
            TokenKind::OpenBracket => self.parse_list_value(token.span, depth),
            TokenKind::RefValue => self.parse_reference_value(),
            TokenKind::Identifier(value) | TokenKind::ProtectedName(value) => {
                Ok(ParsedValue::Name(value))
            }
            _ => Err(FrontendError::other(token.span)),
        }
    }

    /// Parses the only identity-reference constructor, `ref(name)`.
    fn parse_reference_value(&mut self) -> Result<ParsedValue, FrontendError> {
        self.expect_simple(&TokenKind::OpenParen)?;
        let target_token = self.next().ok_or_else(|| self.other_here())?;
        let target = identifier_spelling(&target_token)
            .ok_or_else(|| FrontendError::malformed_boundary(target_token.span))?;
        self.expect_simple(&TokenKind::CloseParen)?;
        Ok(ParsedValue::Reference {
            target,
            target_span: target_token.span,
        })
    }

    /// Parses an ordered, bounded list value with an optional trailing comma.
    fn parse_list_value(
        &mut self,
        open_span: ByteSpan,
        depth: u64,
    ) -> Result<ParsedValue, FrontendError> {
        let nested_depth = depth.saturating_add(1);
        if nested_depth > self.limits.nesting_depth() {
            return Err(FrontendError::list_limit_exceeded(open_span));
        }
        let mut items = Vec::new();
        while !self.at(&TokenKind::CloseBracket) {
            if u64::try_from(items.len()).unwrap_or(u64::MAX) >= self.limits.list_items() {
                return Err(FrontendError::list_limit_exceeded(
                    self.peek().map_or(open_span, |token| token.span),
                ));
            }
            let start = self.peek().ok_or_else(|| self.other_here())?.span.start();
            let value = self.parse_value(nested_depth)?;
            let end = self
                .previous()
                .expect("a parsed list item consumes at least one token")
                .span
                .end();
            items.push(ParsedListItem {
                value,
                span: ByteSpan::new(start, end)
                    .expect("ordered list item tokens must form a valid span"),
            });
            if self.at(&TokenKind::Comma) {
                self.next().expect("looked-ahead list comma must exist");
                if self.at(&TokenKind::CloseBracket) {
                    break;
                }
            } else if !self.at(&TokenKind::CloseBracket) {
                return Err(FrontendError::malformed_boundary(
                    self.peek().map_or(open_span, |token| token.span),
                ));
            }
        }
        self.expect_simple(&TokenKind::CloseBracket)?;
        Ok(ParsedValue::List(items))
    }

    /// Parses explicit fields in one contextual record value.
    fn parse_record_value(
        &mut self,
        open_span: ByteSpan,
        depth: u64,
    ) -> Result<ParsedValue, FrontendError> {
        let nested_depth = depth.saturating_add(1);
        if nested_depth > self.limits.nesting_depth() {
            return Err(FrontendError::record_limit_exceeded(open_span));
        }
        let mut fields = Vec::new();
        self.skip_line_ends();
        while !self.at(&TokenKind::CloseBrace) {
            Self::ensure_capacity(
                fields.len(),
                self.limits.record_fields(),
                self.peek().map_or(open_span, |token| token.span),
            )?;
            let name_token = self.next().ok_or_else(|| self.other_here())?;
            let name = identifier_spelling(&name_token)
                .ok_or_else(|| FrontendError::malformed_boundary(name_token.span))?;
            self.expect_field_delimiter(&TokenKind::Colon)?;
            let value_start = self.peek().ok_or_else(|| self.other_here())?.span.start();
            let value = self.parse_value(nested_depth)?;
            let value_end = self
                .previous()
                .expect("a parsed field value consumes at least one token")
                .span
                .end();
            let value_span = ByteSpan::new(value_start, value_end)
                .expect("ordered field value tokens must form a valid span");
            self.expect_field_delimiter(&TokenKind::Comma)?;
            self.skip_line_ends();
            fields.push(ParsedValueField {
                name,
                value,
                span: ByteSpan::new(name_token.span.start(), value_end)
                    .expect("ordered value-field tokens must form a valid span"),
                name_span: name_token.span,
                value_span,
            });
        }
        self.expect_simple(&TokenKind::CloseBrace)?;
        Ok(ParsedValue::Record(fields))
    }

    /// Fails before growing a bounded declaration or field vector past its limit.
    fn ensure_capacity(current: usize, limit: u64, span: ByteSpan) -> Result<(), FrontendError> {
        if u64::try_from(current).unwrap_or(u64::MAX) >= limit {
            Err(FrontendError::record_limit_exceeded(span))
        } else {
            Ok(())
        }
    }

    /// Consumes a required record field delimiter with a stable owned failure.
    fn expect_field_delimiter(&mut self, expected: &TokenKind) -> Result<Token, FrontendError> {
        let token = self.next().ok_or_else(|| self.other_here())?;
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(expected) {
            Ok(token)
        } else {
            Err(FrontendError::malformed_boundary(token.span))
        }
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
        TokenKind::Use => Some(names::USE.to_owned()),
        TokenKind::Record => Some(names::RECORD.to_owned()),
        TokenKind::List => Some(names::LIST.to_owned()),
        TokenKind::Num => Some(names::NUM.to_owned()),
        TokenKind::StringType => Some(names::STRING.to_owned()),
        TokenKind::BoolType => Some(names::BOOL.to_owned()),
        TokenKind::True => Some(names::TRUE.to_owned()),
        TokenKind::False => Some(names::FALSE.to_owned()),
        TokenKind::Null => Some(names::NULL.to_owned()),
        TokenKind::RefType => Some(names::REF_TYPE.to_owned()),
        TokenKind::RefValue => Some(names::REF.to_owned()),
        _ => None,
    }
}
