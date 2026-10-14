// SPDX-License-Identifier: Apache-2.0

//! Canonical materialized occurrence evidence, independent of supplied source/default assertions.

use super::{
    Budget, E, ExpressionBody, ExpressionProjectIr, ModuleSymbolIdentity, S, V, location, mapping,
};
use neutral_core::allocation::RetainCapacity;
use neutral_ir::{
    composition::{
        CompositionBody as B, ValueOriginKind as K, ValuePathSegment as P,
        project::{CompositionAttribution as A, CompositionOrigin},
    },
    project_interface::ProjectPublicType as T,
};
use neutral_semantics::ValidatedExpressionModel;

/// Borrowed expected path components avoid allocating source-owned field names.
#[derive(Clone, Copy)]
enum Part<'a> {
    /// Canonical record member.
    Field(&'a str),
    /// Ordered list element.
    Element(u64),
    /// Selected tagged payload.
    Payload,
}

/// Checks one-to-one original occurrences for materialized bodies; residual trees use their own paths.
pub(super) fn validate(model: &ValidatedExpressionModel, budget: &mut Budget<'_>) -> Result<(), E> {
    let ir = model.ir();
    let mut origins = ir.origins.iter();
    let mut path = Vec::new();
    for declaration in &ir.declarations {
        if let (S::Binding(ty), Some(ExpressionBody::Value(value))) =
            (&declaration.signature, &declaration.body)
        {
            walk(
                ir,
                &declaration.identity,
                Some(value),
                ty,
                false,
                &mut path,
                &mut origins,
                budget,
            )?;
        }
    }
    if origins.next().is_some() {
        return Err(E::Companion);
    }
    Ok(())
}

/// Resolves already validated nominal contracts without aliases or host acquisition.
pub(super) fn body<'a>(ir: &'a ExpressionProjectIr, ty: &T) -> Result<&'a B, E> {
    match ty {
        T::Nominal(owner) => ir
            .declarations
            .binary_search_by(|d| d.identity.cmp(owner))
            .ok()
            .and_then(|i| match &ir.declarations[i].signature {
                S::Definition(b) => Some(b),
                S::Binding(_) => None,
            })
            .ok_or(E::Companion),
        T::VocabularyNominal {
            identity,
            version,
            name,
        } => ir
            .vocabularies
            .binary_search_by(|b| {
                (b.identity.identity(), b.identity.version())
                    .cmp(&(identity.as_str(), version.as_str()))
            })
            .ok()
            .and_then(|i| {
                ir.vocabularies[i]
                    .definitions
                    .binary_search_by(|d| d.name.cmp(name))
                    .ok()
                    .map(|j| &ir.vocabularies[i].definitions[j].body)
            })
            .ok_or(E::Companion),
        _ => Err(E::Companion),
    }
}

/// Compares a claimed canonical path against independently traversed typed positions.
fn path_matches(actual: &[P], expected: &[Part<'_>]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, b)| {
            matches!((a,b),(P::Field(a),Part::Field(b)) if a==b)
                || matches!((a,b),(P::Element(a),Part::Element(b)) if a==b)
                || matches!((a, b), (P::Payload, Part::Payload))
        })
}

/// Traverses every supplied/null/omitted/default occurrence once in canonical order.
#[allow(clippy::too_many_arguments)]
fn walk<'a>(
    ir: &'a ExpressionProjectIr,
    owner: &'a ModuleSymbolIdentity,
    value: Option<&'a V>,
    ty: &'a T,
    inherited_default: bool,
    path: &mut Vec<Part<'a>>,
    origins: &mut std::slice::Iter<'a, CompositionOrigin>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(path.len() as u64 + 1)?;
    let origin = origins.next().ok_or(E::Companion)?;
    if origin.binding != *owner || !path_matches(&origin.path, path) {
        return Err(E::Companion);
    }
    let defaulted = inherited_default || origin.kind == K::Defaulted;
    if origin.kind == K::Defaulted && path.is_empty() {
        return Err(E::Companion);
    }
    if inherited_default && origin.kind != K::Defaulted {
        return Err(E::Companion);
    }
    if !defaulted
        && origin.kind
            != match value {
                None => K::OmittedOptional,
                Some(V::Null) => K::ExplicitNull,
                Some(_) => K::Supplied,
            }
    {
        return Err(E::Companion);
    }
    attribution(ir, owner, origin, budget)?;
    let Some(value) = value else {
        return Ok(());
    };
    if matches!(value, V::Null) {
        return Ok(());
    }
    let ty = match ty {
        T::Nullable(inner) => inner.as_ref(),
        other => other,
    };
    match (value, ty) {
        (V::List(items), T::List(inner)) => {
            for (index, item) in items.iter().enumerate() {
                path.try_retain(1).map_err(|_| E::Limit)?;
                path.push(Part::Element(index as u64));
                walk(
                    ir,
                    owner,
                    Some(item),
                    inner,
                    defaulted,
                    path,
                    origins,
                    budget,
                )?;
                path.pop();
            }
        }
        (V::Record(values), T::Nominal(_) | T::VocabularyNominal { .. }) => {
            let B::Record(fields) = body(ir, ty)? else {
                return Err(E::Companion);
            };
            for ((name, value), field) in values.iter().zip(fields) {
                path.try_retain(1).map_err(|_| E::Limit)?;
                path.push(Part::Field(name));
                introducing_default(
                    value.as_ref(),
                    field.default.as_ref(),
                    defaulted,
                    origins,
                    budget,
                )?;
                walk(
                    ir,
                    owner,
                    value.as_ref(),
                    &field.ty,
                    defaulted,
                    path,
                    origins,
                    budget,
                )?;
                path.pop();
            }
        }
        (V::Variant { tag, payload }, T::Nominal(_) | T::VocabularyNominal { .. }) => {
            let B::Variant(alts) = body(ir, ty)? else {
                return Err(E::Companion);
            };
            let alt = alts.iter().find(|a| a.tag == *tag).ok_or(E::Companion)?;
            path.try_retain(1).map_err(|_| E::Limit)?;
            path.push(Part::Payload);
            walk(
                ir,
                owner,
                Some(payload),
                &alt.ty,
                defaulted,
                path,
                origins,
                budget,
            )?;
            path.pop();
        }
        _ => {}
    }
    Ok(())
}

/// Only the introducing field can claim its closed default; descendants inherit that evidence.
fn introducing_default(
    value: Option<&V>,
    default: Option<&neutral_ir::composition::ClosedValue>,
    inherited: bool,
    origins: &std::slice::Iter<'_, CompositionOrigin>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    if !inherited
        && origins
            .as_slice()
            .first()
            .is_some_and(|o| o.kind == K::Defaulted)
        && !closed_equal(
            value.ok_or(E::Companion)?,
            default.ok_or(E::Companion)?,
            budget,
        )?
    {
        return Err(E::Companion);
    }
    Ok(())
}

/// Compares a retained value against a closed default without conversion or allocation.
fn closed_equal(
    value: &V,
    closed: &neutral_ir::composition::ClosedValue,
    budget: &mut Budget<'_>,
) -> Result<bool, E> {
    use neutral_ir::composition::CompositionValue as C;
    budget.step(1)?;
    Ok(match (value, closed) {
        (V::Number(a), C::Number(b)) => a == b,
        (V::String(a), C::String(b)) | (V::Url(a), C::Url(b)) | (V::Path(a), C::Path(b)) => {
            budget.step(a.len() as u64 + b.len() as u64 + 1)?;
            a == b
        }
        (V::Bool(a), C::Bool(b)) => a == b,
        (V::Null, C::Null) => true,
        (V::List(a), C::List(b)) if a.len() == b.len() => {
            for (a, b) in a.iter().zip(b) {
                if !closed_equal(a, b, budget)? {
                    return Ok(false);
                }
            }
            true
        }
        (V::Record(a), C::Record(b)) if a.len() == b.len() => {
            for ((an, av), (bn, bv)) in a.iter().zip(b) {
                if an != bn {
                    return Ok(false);
                }
                match (av, bv) {
                    (None, None) => {}
                    (Some(a), Some(b)) if closed_equal(a, b, budget)? => {}
                    _ => return Ok(false),
                }
            }
            true
        }
        (
            V::Variant {
                tag: a,
                payload: av,
            },
            C::Variant {
                tag: b,
                payload: bv,
            },
        ) => a == b && closed_equal(av, bv, budget)?,
        _ => false,
    })
}
/// Checks real source bounds or canonical vocabulary default ownership, never opening a locator.
fn attribution(
    ir: &ExpressionProjectIr,
    owner: &ModuleSymbolIdentity,
    origin: &CompositionOrigin,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    match &origin.attribution {
        None => Ok(()),
        Some(A::Source(loc)) => {
            location(ir, *loc, budget)?;
            if origin.kind == K::Defaulted {
                let Some((T::Nominal(target), _)) = default_owner(ir, owner, origin, budget)?
                else {
                    return Err(E::Companion);
                };
                let map = mapping(ir, target, budget)?;
                return if map.location.source() == loc.source()
                    && loc.span().start() >= map.location.span().start()
                    && loc.span().end() <= map.location.span().end()
                {
                    Ok(())
                } else {
                    Err(E::Companion)
                };
            }
            let map = mapping(ir, owner, budget)?;
            if map.location.source() == loc.source()
                && loc.span().start() >= map.location.span().start()
                && loc.span().end() <= map.location.span().end()
            {
                return Ok(());
            }
            reuse_source(ir, owner, *loc, budget)
        }
        Some(A::Vocabulary {
            identity,
            version,
            type_name,
            field_name,
            span,
        }) => {
            if origin.kind != K::Defaulted {
                return Err(E::Companion);
            }
            let Some((
                T::VocabularyNominal {
                    identity: actual_identity,
                    version: actual_version,
                    name: actual_name,
                },
                actual_field,
            )) = default_owner(ir, owner, origin, budget)?
            else {
                return Err(E::Companion);
            };
            if identity != actual_identity
                || version != actual_version
                || type_name != actual_name
                || field_name != actual_field
            {
                return Err(E::Companion);
            }
            let bundle = ir
                .vocabularies
                .iter()
                .find(|b| b.identity.identity() == identity && b.identity.version() == version)
                .ok_or(E::Companion)?;
            let definition = bundle
                .definitions
                .iter()
                .find(|d| d.name == *type_name)
                .ok_or(E::Companion)?;
            let B::Record(fields) = &definition.body else {
                return Err(E::Companion);
            };
            if !fields
                .iter()
                .any(|f| f.name == *field_name && f.default.is_some())
            {
                return Err(E::Companion);
            }
            let source = ir
                .vocabulary_sources
                .iter()
                .find(|s| s.identity == *identity && s.version == *version)
                .ok_or(E::Companion)?;
            if span.is_some_and(|s| s.is_empty() || s.end() > source.byte_len) {
                return Err(E::Companion);
            }
            Ok(())
        }
    }
}

/// Follows bounded transitive immutable reuse, never unrelated types or guarded output dependencies.
fn reuse_source(
    ir: &ExpressionProjectIr,
    owner: &ModuleSymbolIdentity,
    loc: neutral_core::SourceLocation,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut seen = neutral_core::ordered::OrderedSet::new();
    let mut pending = Vec::new();
    pending.try_retain(1).map_err(|_| E::Limit)?;
    pending.push(owner);
    while let Some(owner) = pending.pop() {
        budget.step(ir.provenance.len() as u64 + seen.len() as u64 + 1)?;
        if !seen.insert(owner).map_err(|_| E::Limit)? {
            continue;
        }
        let map = mapping(ir, owner, budget)?;
        if map.location.source() == loc.source()
            && loc.span().start() >= map.location.span().start()
            && loc.span().end() <= map.location.span().end()
        {
            return Ok(());
        }
        for edge in &ir.provenance {
            if edge.from == *owner
                && edge.kind == neutral_ir::project_interface::ProjectPublicEdgeKind::Value
            {
                pending.try_retain(1).map_err(|_| E::Limit)?;
                pending.push(&edge.to);
            }
        }
    }
    Err(E::Companion)
}

/// Finds the actual field introducing a default, including inherited child occurrences.
fn default_owner<'a>(
    ir: &'a ExpressionProjectIr,
    owner: &ModuleSymbolIdentity,
    origin: &CompositionOrigin,
    budget: &mut Budget<'_>,
) -> Result<Option<(&'a T, &'a str)>, E> {
    let d = ir
        .declarations
        .binary_search_by(|d| d.identity.cmp(owner))
        .ok()
        .map(|i| &ir.declarations[i])
        .ok_or(E::Companion)?;
    let (S::Binding(ty), Some(ExpressionBody::Value(value))) = (&d.signature, &d.body) else {
        return Err(E::Companion);
    };
    let mut ty = ty;
    let mut value = value;
    for (index, segment) in origin.path.iter().enumerate() {
        budget.step(ir.origins.len() as u64 + 1)?;
        if let T::Nullable(inner) = ty {
            ty = inner;
        }
        match (segment, value, ty) {
            (P::Field(name), V::Record(values), T::Nominal(_) | T::VocabularyNominal { .. }) => {
                let B::Record(fields) = body(ir, ty)? else {
                    return Err(E::Companion);
                };
                let field = fields
                    .iter()
                    .find(|f| f.name == *name)
                    .ok_or(E::Companion)?;
                if ir.origins.iter().any(|o| {
                    o.binding == *owner && o.path == origin.path[..=index] && o.kind == K::Defaulted
                }) {
                    if field.default.is_none() {
                        return Err(E::Companion);
                    }
                    return Ok(Some((ty, &field.name)));
                }
                value = values
                    .iter()
                    .find(|(n, _)| n == name)
                    .and_then(|(_, v)| v.as_ref())
                    .ok_or(E::Companion)?;
                ty = &field.ty;
            }
            (P::Element(index), V::List(items), T::List(inner)) => {
                value = items
                    .get(usize::try_from(*index).map_err(|_| E::Companion)?)
                    .ok_or(E::Companion)?;
                ty = inner;
            }
            (
                P::Payload,
                V::Variant { tag, payload },
                T::Nominal(_) | T::VocabularyNominal { .. },
            ) => {
                let B::Variant(alts) = body(ir, ty)? else {
                    return Err(E::Companion);
                };
                ty = &alts.iter().find(|a| a.tag == *tag).ok_or(E::Companion)?.ty;
                value = payload;
            }
            _ => return Err(E::Companion),
        }
    }
    Ok(None)
}
