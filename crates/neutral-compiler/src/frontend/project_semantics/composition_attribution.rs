// SPDX-License-Identifier: Apache-2.0

//! Fine-grained checked source occurrences and canonical vocabulary default owners.

use super::{
    B, E, PROJECT_MAX_DEPTH, Resolver, Root, T, Token, TokenKind, codes, fail, parse_name, profile,
    split,
};
use neutral_core::{ByteSpan, SourceLocation};
use neutral_ir::composition::{
    ValueOriginKind as K, ValuePathSegment as P, project::CompositionAttribution as A,
};

/// Traces one materialized path without fabricating source bytes for vocabulary defaults.
pub(super) fn origin(
    resolver: &mut Resolver<'_>,
    root: &Root,
    ty: &T,
    path: &[P],
    kind: K,
) -> Result<Option<A>, E> {
    let attribution = trace(resolver, root, ty, &root.tokens, path, 0)?;
    // A copied materialized vocabulary default is not a newly applied default.
    // Its original facts remain on the reuse target; unavailable is safer than relabelling supplied data.
    if kind != K::Defaulted && matches!(attribution, Some(A::Vocabulary { .. })) {
        return Ok(None);
    }
    Ok(attribution)
}

/// Builds a checked original-byte location for a complete initializer or selected subexpression.
fn location(root: &Root, tokens: &[Token]) -> Result<Option<A>, E> {
    let (Some(first), Some(last)) = (tokens.first(), tokens.last()) else {
        return Ok(None);
    };
    let span = ByteSpan::new(first.span.start(), last.span.end())
        .map_err(|_| fail(codes::INVALID_SOURCE, Some(root.location)))?;
    Ok(Some(A::Source(SourceLocation::new(
        root.location.source(),
        span,
    ))))
}

/// Selects one object member without confusing member-name tokens with its payload.
fn member<'a>(tokens: &'a [Token], name: &str) -> Result<Option<&'a [Token]>, E> {
    if tokens.len() < 2 {
        return Ok(None);
    }
    for part in split(&tokens[1..tokens.len() - 1])? {
        if matches!(part.first().map(|t| &t.kind), Some(TokenKind::Identifier(n)) if n == name) {
            return part
                .get(2..)
                .map(Some)
                .ok_or_else(|| fail(codes::INVALID_SOURCE, None));
        }
    }
    Ok(None)
}

/// Follows bounded source reuse, lists, selected variants and closed default definitions.
fn trace(
    resolver: &mut Resolver<'_>,
    root: &Root,
    ty: &T,
    tokens: &[Token],
    path: &[P],
    depth: usize,
) -> Result<Option<A>, E> {
    resolver.step(
        tokens.len() as u64 + path.len() as u64 + 1,
        Some(root.location),
    )?;
    if depth > PROJECT_MAX_DEPTH {
        return Err(fail(codes::LIMIT, Some(root.location)));
    }
    if let Some((name, end)) = parse_name(tokens, 0)
        && end == tokens.len()
    {
        let owner = resolver.name(root, name.alias.as_deref(), &name.name, name.span, false)?;
        let target = resolver.roots[&owner].clone();
        return trace(resolver, &target, ty, &target.tokens, path, depth + 1);
    }
    let Some((segment, rest)) = path.split_first() else {
        return location(root, tokens);
    };
    let mut ty = ty;
    while let T::Nullable(inner) = ty {
        ty = inner;
    }
    match segment {
        P::Element(index) => {
            let T::List(inner) = ty else {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(root.location)));
            };
            let interior = tokens
                .get(1..tokens.len().saturating_sub(1))
                .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(root.location)))?;
            let items = split(interior)?;
            let item = usize::try_from(*index)
                .ok()
                .and_then(|i| items.get(i))
                .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(root.location)))?;
            trace(resolver, root, inner, item, rest, depth + 1)
        }
        P::Payload => {
            let B::Variant(alternatives) = resolver.body(ty)? else {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(root.location)));
            };
            let tag = member(tokens, profile::TAG)?
                .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(root.location)))?;
            let Some(Token {
                kind: TokenKind::StringLiteral(tag),
                ..
            }) = tag.first()
            else {
                return Err(fail(codes::INVALID_SOURCE, Some(root.location)));
            };
            let selected = alternatives
                .iter()
                .find(|a| a.tag == tag.value)
                .ok_or_else(|| fail(codes::UNKNOWN_TAG, Some(root.location)))?;
            let payload = member(tokens, profile::PAYLOAD)?
                .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(root.location)))?;
            trace(resolver, root, &selected.ty, payload, rest, depth + 1)
        }
        P::Field(name) => field(resolver, root, ty, tokens, name, rest, depth),
    }
}

/// Attributes supplied fields to their source subexpression and omitted defaults to their real contract owner.
fn field(
    resolver: &mut Resolver<'_>,
    root: &Root,
    ty: &T,
    tokens: &[Token],
    name: &str,
    rest: &[P],
    depth: usize,
) -> Result<Option<A>, E> {
    let B::Record(fields) = resolver.body(ty)? else {
        return Err(fail(codes::INCOMPATIBLE_VALUE, Some(root.location)));
    };
    let field = fields
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(root.location)))?;
    if let Some(value) = member(tokens, name)? {
        return trace(resolver, root, &field.ty, value, rest, depth + 1);
    }
    if field.default.is_none() {
        return location(root, tokens.last().map_or(&[], std::slice::from_ref));
    }
    match ty {
        T::VocabularyNominal {
            identity,
            version,
            name: type_name,
        } => Ok(Some(A::Vocabulary {
            identity: identity.clone(),
            version: version.clone(),
            type_name: type_name.clone(),
            field_name: name.to_owned(),
            span: None,
        })),
        T::Nominal(owner) => {
            let definition = resolver.roots[owner].clone();
            for part in split(&definition.tokens)? {
                let Some(eq) = part
                    .iter()
                    .position(|t| matches!(t.kind, TokenKind::Equals))
                else {
                    continue;
                };
                if matches!(eq.checked_sub(1).and_then(|i| part.get(i)).map(|t| &t.kind), Some(TokenKind::Identifier(n)) if n == name)
                {
                    return trace(
                        resolver,
                        &definition,
                        &field.ty,
                        &part[eq + 1..],
                        rest,
                        depth + 1,
                    );
                }
            }
            Ok(None)
        }
        _ => Err(fail(codes::INCOMPATIBLE_VALUE, Some(root.location))),
    }
}
