// SPDX-License-Identifier: Apache-2.0

//! Semantic line-end normalization for the minimal frontend slice.

use super::{FrontendError, Token, TokenKind};

/// Replaces physical newlines with semantic line ends and inserts one at EOF.
pub(super) fn normalize(raw: Vec<Token>) -> Result<Vec<Token>, FrontendError> {
    let mut normalized = Vec::with_capacity(raw.len().saturating_add(1));
    let mut construct_start = 0_usize;
    for token in raw {
        match token.kind {
            TokenKind::PhysicalLineEnd(_) => {
                if construct_start == normalized.len() {
                    continue;
                }
                if !is_complete_construct(&normalized[construct_start..]) {
                    return Err(FrontendError::other(token.span));
                }
                normalized.push(Token {
                    kind: TokenKind::LineEnd,
                    span: token.span,
                });
                construct_start = normalized.len();
            }
            TokenKind::EndOfFile => {
                if construct_start < normalized.len() {
                    if !is_complete_construct(&normalized[construct_start..]) {
                        return Err(FrontendError::other(token.span));
                    }
                    normalized.push(Token {
                        kind: TokenKind::LineEnd,
                        span: token.span,
                    });
                }
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
    Ok(normalized)
}

/// Returns whether one physical line is a complete minimal frontend construct.
fn is_complete_construct(tokens: &[Token]) -> bool {
    matches!(
        tokens,
        [
            Token {
                kind: TokenKind::Neu,
                ..
            },
            Token {
                kind: TokenKind::StringLiteral(_),
                ..
            }
        ] | [
            Token {
                kind: TokenKind::Module,
                ..
            },
            Token {
                kind: TokenKind::Identifier(_),
                ..
            }
        ] | [
            Token {
                kind: TokenKind::Num,
                ..
            },
            Token {
                kind: TokenKind::Identifier(_),
                ..
            },
            Token {
                kind: TokenKind::Equals,
                ..
            },
            Token {
                kind: TokenKind::Number(_),
                ..
            }
        ]
    )
}
