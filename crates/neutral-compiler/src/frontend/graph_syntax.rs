// SPDX-License-Identifier: Apache-2.0

//! Bounded v1 module/import scanner over the existing exact-byte lexer.

use super::{Token, TokenKind, lexer};
use crate::language::{graph_names, names};
use neutral_core::allocation::RetainCapacity;
use neutral_core::{ByteSpan, CancellationToken, allocation::text, profile::V1_SOURCE_PROFILE};

#[cfg(test)]
#[path = "../../tests/frontend/graph_syntax.rs"]
mod tests;

/// One parsed, source-accounted logical import.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GraphImport {
    /// Exact target module ID.
    pub(crate) target: String,
    /// Required local alias.
    pub(crate) alias: String,
    /// Original-byte span of the complete import statement.
    pub(crate) span: ByteSpan,
}

/// One syntax scan result before graph validation.
pub(crate) struct GraphSourceSyntax {
    /// Exact module header span.
    pub(crate) header_span: ByteSpan,
    /// Imported modules in original source order.
    pub(crate) imports: Vec<GraphImport>,
    /// Names already occupied by captured vocabulary requirements.
    pub(crate) vocabulary_aliases: Vec<String>,
}

/// Classification of a source syntax failure at the graph boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GraphSyntaxErrorKind {
    /// The required source grammar or placement is malformed.
    InvalidSyntax,
    /// An import attempts a forbidden form, including re-export or acquisition.
    ForbiddenImport,
    /// One vocabulary alias collides with another alias or the local module.
    AliasCollision,
    /// The per-module import count exceeded its explicit ceiling.
    LimitExceeded,
}

/// One bounded source syntax error with an original-byte location.
pub(crate) struct GraphSyntaxError {
    /// Stable failure classification.
    pub(crate) kind: GraphSyntaxErrorKind,
    /// Exact original-byte location.
    pub(crate) span: ByteSpan,
}

/// Scans one captured unit without consulting host locations or resolvers.
pub(crate) fn scan_graph_source(
    bytes: &[u8],
    expected_module: &str,
    import_limit: u64,
) -> Result<GraphSourceSyntax, GraphSyntaxError> {
    scan_graph_source_bounded(bytes, expected_module, import_limit, usize::MAX, None)
}

/// Scans graph grammar with cancellation-aware fallible lexical and syntax retention.
pub(crate) fn scan_graph_source_bounded(
    bytes: &[u8],
    expected_module: &str,
    import_limit: u64,
    string_bytes: usize,
    cancel: Option<&CancellationToken>,
) -> Result<GraphSourceSyntax, GraphSyntaxError> {
    let lexed =
        lexer::lex_bounded(bytes, string_bytes, cancel).map_err(|error| GraphSyntaxError {
            kind: if matches!(error.kind, super::FrontendErrorKind::RecordLimitExceeded) {
                GraphSyntaxErrorKind::LimitExceeded
            } else if forbidden_line_at(bytes, error.span.start()) {
                GraphSyntaxErrorKind::ForbiddenImport
            } else {
                GraphSyntaxErrorKind::InvalidSyntax
            },
            span: error.span,
        })?;
    let mut result = GraphSourceSyntax {
        header_span: span(0, 0),
        imports: Vec::new(),
        vocabulary_aliases: Vec::new(),
    };
    let mut phase = 0_u8;
    let mut start = 0_usize;
    for (index, token) in lexed.tokens.iter().enumerate() {
        if cancel.is_some_and(CancellationToken::is_cancelled) {
            return Err(retention(token.span));
        }
        if !matches!(
            token.kind,
            TokenKind::PhysicalLineEnd(_) | TokenKind::EndOfFile
        ) {
            continue;
        }
        let line = &lexed.tokens[start..index];
        if !line.is_empty() {
            scan_line(line, expected_module, import_limit, &mut phase, &mut result)?;
        }
        start = index + 1;
    }
    if phase < 2 {
        return Err(GraphSyntaxError {
            kind: GraphSyntaxErrorKind::InvalidSyntax,
            span: span(bytes.len(), bytes.len()),
        });
    }
    Ok(result)
}

/// Advances the header/import/declaration phases for one physical source line.
fn scan_line(
    line: &[Token],
    expected_module: &str,
    import_limit: u64,
    phase: &mut u8,
    result: &mut GraphSourceSyntax,
) -> Result<(), GraphSyntaxError> {
    let line_span = token_span(line);
    if *phase == 0 {
        if !matches!(line, [Token { kind: TokenKind::Neu, .. }, Token { kind: TokenKind::StringLiteral(value), .. }] if !value.had_escape && value.value == V1_SOURCE_PROFILE)
        {
            return Err(invalid(line_span));
        }
        *phase = 1;
        return Ok(());
    }
    if *phase == 1 {
        if !matches!(line[0].kind, TokenKind::Module)
            || parse_module_name(&line[1..])?.as_deref() != Some(expected_module)
        {
            return Err(invalid(line_span));
        }
        result.header_span = line_span;
        *phase = 2;
        return Ok(());
    }
    if matches!(line[0].kind, TokenKind::Use) {
        if *phase > 2 {
            return Err(invalid(line_span));
        }
        let [
            _,
            Token {
                kind: TokenKind::Identifier(identity),
                ..
            },
            Token {
                kind: TokenKind::Identifier(as_word),
                ..
            },
            Token {
                kind: TokenKind::Identifier(alias),
                ..
            },
        ] = line
        else {
            return Err(invalid(line_span));
        };
        if !identity.starts_with(|character: char| character.is_ascii_uppercase())
            || as_word != graph_names::AS
            || !valid_graph_name(alias)
        {
            return Err(invalid(line_span));
        }
        if alias == expected_module.rsplit("::").next().unwrap_or("")
            || result.vocabulary_aliases.contains(alias)
        {
            return Err(GraphSyntaxError {
                kind: GraphSyntaxErrorKind::AliasCollision,
                span: line_span,
            });
        }
        result
            .vocabulary_aliases
            .try_retain(1)
            .map_err(|_| retention(line_span))?;
        result
            .vocabulary_aliases
            .push(text(alias).map_err(|_| retention(line_span))?);
        return Ok(());
    }
    if matches!(&line[0].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC)
        && line
            .get(1)
            .is_some_and(|token| matches!(&token.kind, TokenKind::Identifier(word) if word == graph_names::IMPORT))
    {
        return Err(GraphSyntaxError {
            kind: GraphSyntaxErrorKind::ForbiddenImport,
            span: line_span,
        });
    }
    if matches!(&line[0].kind, TokenKind::Identifier(word) if word == graph_names::IMPORT) {
        if *phase > 3 {
            return Err(invalid(line_span));
        }
        *phase = 3;
        let parsed = parse_import(line)?;
        if u64::try_from(result.imports.len()).unwrap_or(u64::MAX) >= import_limit {
            return Err(GraphSyntaxError {
                kind: GraphSyntaxErrorKind::LimitExceeded,
                span: line_span,
            });
        }
        result
            .imports
            .try_retain(1)
            .map_err(|_| retention(line_span))?;
        result.imports.push(parsed);
        return Ok(());
    }
    if matches!(line[0].kind, TokenKind::Module | TokenKind::Neu) {
        return Err(invalid(line_span));
    }
    *phase = 4;
    Ok(())
}

/// Parses one exact aliased import statement from already lexed tokens.
fn parse_import(line: &[Token]) -> Result<GraphImport, GraphSyntaxError> {
    let line_span = token_span(line);
    if line.len() < 4 {
        return Err(invalid(line_span));
    }
    if line.get(1).is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::DoubleColon | TokenKind::StringLiteral(_)
        )
    }) {
        return Err(GraphSyntaxError {
            kind: GraphSyntaxErrorKind::ForbiddenImport,
            span: line_span,
        });
    }
    let mut end = 2_usize;
    while end + 1 < line.len() && matches!(line[end].kind, TokenKind::DoubleColon) {
        end += 2;
    }
    let target = parse_module_name(&line[1..end])?.ok_or_else(|| invalid(line_span))?;
    let [
        Token {
            kind: TokenKind::Identifier(as_word),
            ..
        },
        Token {
            kind: TokenKind::Identifier(alias),
            ..
        },
    ] = &line[end..]
    else {
        return Err(invalid(line_span));
    };
    if as_word != graph_names::AS || !valid_graph_name(alias) {
        return Err(invalid(line_span));
    }
    Ok(GraphImport {
        target,
        alias: text(alias).map_err(|_| retention(line_span))?,
        span: line_span,
    })
}

/// Reads a qualified module ID from alternating name and `::` tokens.
fn parse_module_name(tokens: &[Token]) -> Result<Option<String>, GraphSyntaxError> {
    if tokens.is_empty() || tokens.len().is_multiple_of(2) {
        return Ok(None);
    }
    let mut result = String::new();
    for (index, token) in tokens.iter().enumerate() {
        if index % 2 == 0 {
            let TokenKind::Identifier(name) = &token.kind else {
                return Ok(None);
            };
            if !valid_graph_name(name) {
                return Ok(None);
            }
            result
                .try_retain(name.len())
                .map_err(|_| retention(token.span))?;
            result.push_str(name);
        } else if !matches!(token.kind, TokenKind::DoubleColon) {
            return Ok(None);
        } else {
            result.try_retain(2).map_err(|_| retention(token.span))?;
            result.push_str("::");
        }
    }
    Ok(Some(result))
}

/// Applies the exact ASCII snake-name grammar and excludes language keywords.
fn valid_graph_name(value: &str) -> bool {
    value.split('_').all(|part| {
        let mut bytes = part.bytes();
        bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
            && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }) && !names::is_protected_name(value)
        && !matches!(
            value,
            graph_names::IMPORT | graph_names::AS | graph_names::PUBLIC
        )
}

/// Classifies an unsupported symbol on an import line as a forbidden form.
fn forbidden_line_at(bytes: &[u8], offset: u64) -> bool {
    let end = usize::try_from(offset)
        .unwrap_or(bytes.len())
        .min(bytes.len());
    let start = bytes[..end]
        .iter()
        .rposition(|byte| matches!(byte, b'\n' | b'\r'))
        .map_or(0, |index| index + 1);
    let prefix = &bytes[start..end];
    let first = prefix
        .iter()
        .position(|byte| !matches!(byte, b' ' | b'\t'))
        .unwrap_or(prefix.len());
    let prefix = &prefix[first..];
    prefix.starts_with(graph_names::IMPORT.as_bytes())
        || prefix
            .strip_prefix(graph_names::PUBLIC.as_bytes())
            .and_then(|tail| tail.strip_prefix(b" "))
            .is_some_and(|tail| tail.starts_with(graph_names::IMPORT.as_bytes()))
}
/// Classifies failed private graph retention without allocating diagnostic text.
fn retention(span: ByteSpan) -> GraphSyntaxError {
    GraphSyntaxError {
        kind: GraphSyntaxErrorKind::LimitExceeded,
        span,
    }
}

/// Creates one stable invalid-syntax error at the original-byte location.
fn invalid(span: ByteSpan) -> GraphSyntaxError {
    GraphSyntaxError {
        kind: GraphSyntaxErrorKind::InvalidSyntax,
        span,
    }
}

/// Returns the exact original-byte span of a nonempty token line.
fn token_span(line: &[Token]) -> ByteSpan {
    ByteSpan::new(line[0].span.start(), line[line.len() - 1].span.end())
        .expect("ordered token offsets must form a span")
}

/// Creates a checked original-byte half-open span from lexer offsets.
fn span(start: usize, end: usize) -> ByteSpan {
    ByteSpan::new(start as u64, end as u64).expect("ordered lexer offsets must form a span")
}
