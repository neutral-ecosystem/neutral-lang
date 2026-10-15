// SPDX-License-Identifier: Apache-2.0

//! Parse composition declaration roots from already captured source bytes.
//!
//! Retain record/variant shapes, bindings, type expressions and source spans for
//! later resolution. Token splitting respects nested delimiters and caller
//! limits; allocation and cancellation failures propagate through the pipeline's
//! failure envelope. Parsing does not resolve names or materialize defaults.

use super::{
    BorrowedOccurrence, ByteSpan, CancellationToken, CapturedCompositionProject, E,
    LogicalModuleIdentity, ModuleSymbolIdentity, PROJECT_MAX_DEPTH, RetainCapacity, Root,
    SourceLocation, Token, TokenKind, TypeExpr, V1_SOURCE_PROFILE, codes, fail, is_protected_name,
    is_requirement_or_import, is_snake_name, is_upper_name, lexer, parse_complete_type_fallible,
    profile,
};

/// Recognizes a local or qualified name without copying token storage.
pub(super) fn parse_name(
    tokens: &[Token],
    index: usize,
) -> Option<(BorrowedOccurrence<'_>, usize)> {
    let first = tokens.get(index)?;
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if matches!(
        tokens.get(index + 1).map(|t| &t.kind),
        Some(TokenKind::DoubleColon)
    ) {
        let second = tokens.get(index + 2)?;
        let TokenKind::Identifier(target) = &second.kind else {
            return None;
        };
        Some((
            BorrowedOccurrence {
                alias: Some(name),
                name: target,
                span: second.span,
            },
            index + 3,
        ))
    } else {
        Some((
            BorrowedOccurrence {
                alias: None,
                name,
                span: first.span,
            },
            index + 1,
        ))
    }
}
/// Recognizes a reference's borrowed target without retaining an intermediate name.
pub(super) fn parse_reference(
    tokens: &[Token],
    index: usize,
) -> Option<(BorrowedOccurrence<'_>, usize)> {
    if !matches!(tokens.get(index + 1)?.kind, TokenKind::OpenParen) {
        return None;
    }
    let (name, end) = parse_name(tokens, index + 2)?;
    matches!(tokens.get(end)?.kind, TokenKind::CloseParen).then_some((name, end + 1))
}

/// Parses declarations once while preserving physical statement and original-byte boundaries.
pub(super) fn parse(
    source: &crate::CapturedProjectSource,
    captured: &CapturedCompositionProject,
    cancel: &CancellationToken,
) -> Result<Vec<Root>, E> {
    let string_limit =
        usize::try_from(captured.composition_limits().json.string_bytes()).unwrap_or(usize::MAX);
    let tokens = lexer::lex_bounded(source.bytes(), string_limit, Some(cancel))
        .map_err(|e| {
            fail(
                if cancel.is_cancelled() {
                    codes::CANCELLED
                } else if matches!(
                    e.kind,
                    crate::frontend::FrontendErrorKind::RecordLimitExceeded
                ) {
                    codes::LIMIT
                } else {
                    codes::INVALID_SOURCE
                },
                Some(SourceLocation::new(source.digest(), e.span)),
            )
        })?
        .tokens;
    let mut statement = Vec::new();
    let mut roots = Vec::new();
    let mut depth = 0;
    let mut headers = 0;
    for token in tokens {
        if cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        if matches!(
            token.kind,
            TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
        ) {
            if depth == 0 && !statement.is_empty() {
                if headers < 2 {
                    headers += 1;
                } else if !is_requirement_or_import(&statement) {
                    if roots.len() as u64 >= captured.limits().values().declarations {
                        return Err(fail(codes::LIMIT, None));
                    }
                    roots.try_retain(1).map_err(|_| fail(codes::LIMIT, None))?;
                    roots.push(parse_declaration(
                        source,
                        &statement,
                        captured.composition_limits(),
                    )?);
                }
                statement.clear();
            }
            continue;
        }
        match token.kind {
            TokenKind::OpenBrace | TokenKind::OpenBracket | TokenKind::OpenParen => depth += 1,
            TokenKind::CloseBrace | TokenKind::CloseBracket | TokenKind::CloseParen => {
                if depth == 0 {
                    return Err(fail(
                        codes::INVALID_SOURCE,
                        Some(SourceLocation::new(source.digest(), token.span)),
                    ));
                }
                depth -= 1;
            }
            _ => {}
        }
        if depth > PROJECT_MAX_DEPTH {
            return Err(fail(
                codes::LIMIT,
                Some(SourceLocation::new(source.digest(), token.span)),
            ));
        }
        statement
            .try_retain(1)
            .map_err(|_| fail(codes::LIMIT, None))?;
        statement.push(token);
    }
    if !statement.is_empty() {
        return Err(fail(
            codes::INVALID_SOURCE,
            Some(SourceLocation::new(source.digest(), statement[0].span)),
        ));
    }
    Ok(roots)
}

/// Adds contextual variant syntax without reserving `variant` as a binding identifier.
pub(super) fn parse_declaration(
    source: &crate::CapturedProjectSource,
    tokens: &[Token],
    limits: neutral_vocabulary::composition::CompositionLimits,
) -> Result<Root, E> {
    let public = matches!(&tokens[0].kind,TokenKind::Identifier(s) if s==crate::language::graph_names::PUBLIC);
    let start = usize::from(public);
    let variant = matches!(tokens.get(start).map(|t|&t.kind),Some(TokenKind::Identifier(s)) if s==profile::VARIANT);
    let span = ByteSpan::new(tokens[0].span.start(), tokens[tokens.len() - 1].span.end())
        .map_err(|_| fail(codes::INVALID_SOURCE, None))?;
    let location = SourceLocation::new(source.digest(), span);
    if variant || matches!(tokens.get(start).map(|t| &t.kind), Some(TokenKind::Record)) {
        let Some(Token {
            kind: TokenKind::Identifier(name),
            ..
        }) = tokens.get(start + 1)
        else {
            return Err(fail(codes::INVALID_SOURCE, Some(location)));
        };
        if !is_upper_name(name)
            || is_protected_name(name)
            || !matches!(
                tokens.get(start + 2).map(|t| &t.kind),
                Some(TokenKind::OpenBrace)
            )
            || !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::CloseBrace))
        {
            return Err(fail(codes::INVALID_SOURCE, Some(location)));
        }
        let body = &tokens[start + 3..tokens.len() - 1];
        if variant && body.iter().any(|t| matches!(t.kind, TokenKind::Equals)) {
            return Err(fail(codes::INVALID_SOURCE, Some(location)));
        }
        let fields = parsed_fields(
            body,
            if variant {
                limits.alternatives_per_type
            } else {
                limits.json.fields()
            },
        )?;
        if variant && fields.is_empty() {
            return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
        }
        return Ok(Root {
            owner: ModuleSymbolIdentity::new(
                LogicalModuleIdentity::new(
                    neutral_core::allocation::text(V1_SOURCE_PROFILE)
                        .map_err(|_| fail(codes::LIMIT, None))?,
                    neutral_core::allocation::text(source.module_id())
                        .map_err(|_| fail(codes::LIMIT, None))?,
                ),
                neutral_core::allocation::text(name).map_err(|_| fail(codes::LIMIT, None))?,
            ),
            public,
            location,
            ty: None,
            fields,
            tokens: neutral_core::allocation::copy_slice(body)
                .map_err(|_| fail(codes::LIMIT, None))?,
            variant,
        });
    }
    let eq = tokens
        .iter()
        .position(|t| matches!(t.kind, TokenKind::Equals))
        .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(location)))?;
    if eq <= start + 1 || eq + 1 >= tokens.len() {
        return Err(fail(codes::INVALID_SOURCE, Some(location)));
    }
    let TokenKind::Identifier(name) = &tokens[eq - 1].kind else {
        return Err(fail(codes::INVALID_SOURCE, Some(location)));
    };
    if !is_snake_name(name) || is_protected_name(name) {
        return Err(fail(codes::INVALID_SOURCE, Some(location)));
    }
    let ty = parse_complete_type_fallible(&tokens[start..eq - 1])
        .map_err(|_| fail(codes::LIMIT, None))?
        .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(location)))?;
    Ok(Root {
        owner: ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(
                neutral_core::allocation::text(V1_SOURCE_PROFILE)
                    .map_err(|_| fail(codes::LIMIT, None))?,
                neutral_core::allocation::text(source.module_id())
                    .map_err(|_| fail(codes::LIMIT, None))?,
            ),
            neutral_core::allocation::text(name).map_err(|_| fail(codes::LIMIT, None))?,
        ),
        public,
        location,
        ty: Some(ty),
        fields: Vec::new(),
        tokens: neutral_core::allocation::copy_slice(&tokens[eq + 1..])
            .map_err(|_| fail(codes::LIMIT, None))?,
        variant: false,
    })
}

/// Parses complete source member types without requiring old-profile value terminators.
fn parsed_fields(tokens: &[Token], limit: u64) -> Result<Vec<(String, TypeExpr)>, E> {
    let mut result = Vec::new();
    let mut names = neutral_core::ordered::OrderedSet::new();
    for member in split(tokens)? {
        let end = member
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Equals))
            .unwrap_or(member.len());
        if end < 2 {
            return Err(fail(codes::INVALID_SOURCE, None));
        }
        let TokenKind::Identifier(name) = &member[end - 1].kind else {
            return Err(fail(codes::INVALID_SOURCE, None));
        };
        if result.len() as u64 >= limit {
            return Err(fail(codes::LIMIT, None));
        }
        if !is_snake_name(name)
            || is_protected_name(name)
            || !names
                .insert(name.as_str())
                .map_err(|_| fail(codes::LIMIT, None))?
        {
            return Err(fail(codes::DUPLICATE_MEMBER, None));
        }
        let ty = parse_complete_type_fallible(&member[..end - 1])
            .map_err(|_| fail(codes::LIMIT, None))?
            .ok_or_else(|| fail(codes::INVALID_SOURCE, None))?;
        if end < member.len() && end + 1 == member.len() {
            return Err(fail(codes::INVALID_SOURCE, None));
        }
        result.try_retain(1).map_err(|_| fail(codes::LIMIT, None))?;
        result.push((
            neutral_core::allocation::text(name).map_err(|_| fail(codes::LIMIT, None))?,
            ty,
        ));
    }
    Ok(result)
}

/// Splits top-level members only, permitting one final comma and rejecting empty interior entries.
pub(super) fn split(tokens: &[Token]) -> Result<Vec<&[Token]>, E> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, t) in tokens.iter().enumerate() {
        match t.kind {
            TokenKind::OpenBrace
            | TokenKind::OpenBracket
            | TokenKind::OpenParen
            | TokenKind::Less => depth += 1,
            TokenKind::CloseBrace
            | TokenKind::CloseBracket
            | TokenKind::CloseParen
            | TokenKind::Greater => {
                if depth == 0 {
                    return Err(fail(codes::INVALID_SOURCE, None));
                }
                depth -= 1;
            }
            _ => {}
        }
        if depth == 0 && matches!(t.kind, TokenKind::Comma) {
            if start == i {
                return Err(fail(codes::INVALID_SOURCE, None));
            }
            result.try_retain(1).map_err(|_| fail(codes::LIMIT, None))?;
            result.push(&tokens[start..i]);
            start = i + 1;
        }
    }
    if depth != 0 {
        return Err(fail(codes::INVALID_SOURCE, None));
    }
    if start < tokens.len() {
        result.try_retain(1).map_err(|_| fail(codes::LIMIT, None))?;
        result.push(&tokens[start..]);
    }
    Ok(result)
}

#[cfg(test)]
#[path = "../../../../tests/project_cache/composition_syntax.rs"]
mod tests;
