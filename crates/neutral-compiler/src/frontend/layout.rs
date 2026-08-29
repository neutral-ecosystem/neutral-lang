// SPDX-License-Identifier: Apache-2.0

//! Semantic line-end normalization for active scalar frontend slices.

use super::{FrontendError, LexedSource, Token, TokenKind};

/// Replaces physical newlines with semantic line ends and inserts one at EOF.
pub(super) fn normalize(raw: LexedSource) -> Result<LexedSource, FrontendError> {
    let mut normalized = Vec::with_capacity(raw.tokens.len().saturating_add(1));
    let mut construct_start = 0_usize;
    let mut brace_depth = 0_u64;
    for token in raw.tokens {
        match token.kind {
            TokenKind::PhysicalLineEnd(_) => {
                if brace_depth > 0 {
                    continue;
                }
                if construct_start == normalized.len() {
                    continue;
                }
                if !is_complete_construct(&normalized[construct_start..]) {
                    return Err(FrontendError::malformed_boundary(token.span));
                }
                normalized.push(Token {
                    kind: TokenKind::LineEnd,
                    span: token.span,
                });
                construct_start = normalized.len();
            }
            TokenKind::EndOfFile => {
                if brace_depth != 0 {
                    return Err(FrontendError::malformed_boundary(token.span));
                }
                if construct_start < normalized.len() {
                    if !is_complete_construct(&normalized[construct_start..]) {
                        return Err(FrontendError::malformed_boundary(token.span));
                    }
                    normalized.push(Token {
                        kind: TokenKind::LineEnd,
                        span: token.span,
                    });
                }
                normalized.push(token);
            }
            TokenKind::OpenBrace => {
                brace_depth = brace_depth.saturating_add(1);
                normalized.push(token);
            }
            TokenKind::CloseBrace => {
                brace_depth = brace_depth
                    .checked_sub(1)
                    .ok_or_else(|| FrontendError::malformed_boundary(token.span))?;
                normalized.push(token);
            }
            _ => normalized.push(token),
        }
    }
    if !normalized
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::EndOfFile))
    {
        let span = normalized
            .last()
            .map(|token| token.span)
            .ok_or_else(|| FrontendError::other(super::span(0, 0)))?;
        return Err(FrontendError::other(span));
    }
    Ok(LexedSource {
        tokens: normalized,
        trivia: raw.trivia,
    })
}

/// Returns whether one top-level physical construct is complete.
fn is_complete_construct(tokens: &[Token]) -> bool {
    match tokens {
        [first, second] => {
            matches!(first.kind, TokenKind::Neu)
                && matches!(second.kind, TokenKind::StringLiteral(_))
                || matches!(first.kind, TokenKind::Module) && is_name_token(&second.kind)
        }
        [first, .., last] if matches!(first.kind, TokenKind::Record) => {
            matches!(last.kind, TokenKind::CloseBrace)
        }
        [first, .., last] if is_type_start(&first.kind) => {
            tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Equals))
                && (is_scalar_value(&last.kind) || matches!(last.kind, TokenKind::CloseBrace))
        }
        _ => false,
    }
}

/// Returns whether a token can begin an active scalar or nominal type.
fn is_type_start(kind: &TokenKind) -> bool {
    is_scalar_type(kind) || matches!(kind, TokenKind::Identifier(_))
}

/// Returns whether a token is an active explicit scalar type.
fn is_scalar_type(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Num | TokenKind::StringType | TokenKind::BoolType
    )
}

/// Returns whether a token is an active explicit scalar literal.
fn is_scalar_value(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Number(_)
            | TokenKind::StringLiteral(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
    )
}

/// Returns whether a token spelling can be diagnosed in an identifier position.
fn is_name_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier(_)
            | TokenKind::ProtectedName(_)
            | TokenKind::Neu
            | TokenKind::Module
            | TokenKind::Record
            | TokenKind::Num
            | TokenKind::StringType
            | TokenKind::BoolType
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
    )
}
