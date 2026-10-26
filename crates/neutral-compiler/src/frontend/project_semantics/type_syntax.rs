// SPDX-License-Identifier: Apache-2.0

//! Shared bounded type and header syntax for current captured-project compilers.
//!
//! Consumes an entire already-lexed type: scalars, contextual inert locations,
//! local/qualified nominal names and invariant list/reference/nullable wrappers.
//! A parsed name retains the original occurrence for later resolution diagnostics;
//! parsing does not determine whether its owner exists, is public or is the right kind.
//!
//! `Ok(None)` means malformed/incomplete grammar or excessive syntactic depth;
//! allocation failure remains an explicit `Err`. This distinction prevents a
//! failed retention from masquerading as an ordinary unrecognized spelling.
//! Recursion is bounded before allocating wrappers; semantic wrapper constraints
//! are enforced after complete syntax has been accepted.

use super::TypeExpr;
use crate::frontend::{Token, TokenKind};
use crate::language::graph_names;
use neutral_ir::project_interface::MAX_PROJECT_INTERFACE_TYPE_DEPTH;

/// Separates malformed grammar from allocation failure while retaining a complete type.
pub(crate) fn parse_complete_type_fallible(
    tokens: &[Token],
) -> Result<Option<TypeExpr>, neutral_core::allocation::AllocationError> {
    let mut index = 0;
    let ty = parse_type_fallible(tokens, &mut index, 0)?;
    // A valid prefix is insufficient: trailing tokens must not turn into a second implicit type.
    Ok(ty.filter(|_| index == tokens.len()))
}

/// Bounds recursive grammar before fallibly retaining spelling or wrapper storage.
fn parse_type_fallible(
    tokens: &[Token],
    index: &mut usize,
    depth: usize,
) -> Result<Option<TypeExpr>, neutral_core::allocation::AllocationError> {
    use neutral_core::allocation::{boxed, text};
    if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        return Ok(None);
    }
    let Some(first) = tokens.get(*index) else {
        return Ok(None);
    };
    *index += 1;
    let mut ty = match &first.kind {
        TokenKind::Num => TypeExpr::Num,
        TokenKind::StringType => TypeExpr::String,
        TokenKind::BoolType => TypeExpr::Bool,
        TokenKind::Identifier(name) if name == graph_names::URL => TypeExpr::Url,
        TokenKind::Identifier(name) if name == graph_names::PATH => TypeExpr::Path,
        TokenKind::Identifier(name) | TokenKind::ProtectedName(name) => {
            if matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::DoubleColon)
            ) {
                *index += 1;
                let Some(second) = tokens.get(*index) else {
                    return Ok(None);
                };
                let TokenKind::Identifier(target) = &second.kind else {
                    return Ok(None);
                };
                *index += 1;
                TypeExpr::Nominal(Some(text(name)?), text(target)?, second.span)
            } else {
                TypeExpr::Nominal(None, text(name)?, first.span)
            }
        }
        TokenKind::List | TokenKind::RefType => {
            if !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::Less)
            ) {
                return Ok(None);
            }
            *index += 1;
            let Some(inner) = parse_type_fallible(tokens, index, depth + 1)? else {
                return Ok(None);
            };
            if !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::Greater)
            ) {
                return Ok(None);
            }
            *index += 1;
            let inner = boxed(inner)?;
            if matches!(first.kind, TokenKind::List) {
                TypeExpr::List(inner)
            } else {
                TypeExpr::Ref(inner)
            }
        }
        _ => return Ok(None),
    };
    if matches!(
        tokens.get(*index).map(|token| &token.kind),
        Some(TokenKind::Question)
    ) {
        *index += 1;
        ty = TypeExpr::Nullable(boxed(ty)?);
    }
    Ok(Some(ty))
}
