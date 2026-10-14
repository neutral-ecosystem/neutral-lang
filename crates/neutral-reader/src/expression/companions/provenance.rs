// SPDX-License-Identifier: Apache-2.0

//! Independent structural graph evidence; guarded value demand is never a scheduling edge.

use super::{
    Budget, E, ExpressionBody, ExpressionProjectIr, ModuleSymbolIdentity, N, S, V, owned_location,
};
use neutral_core::{allocation::RetainCapacity, ordered::OrderedSet};
use neutral_ir::{
    composition::CompositionBody as B,
    project_interface::{ProjectPublicEdgeKind as K, ProjectPublicType as T},
};
use neutral_semantics::ValidatedExpressionModel;

/// Canonical borrowed structural edges, independent of occurrence-sensitive source spans.
type Edges<'a> = OrderedSet<(&'a ModuleSymbolIdentity, K, &'a ModuleSymbolIdentity)>;

/// Checks derived type/ref edges, source-owned occurrence evidence and immutable reuse cycles.
pub(super) fn validate(model: &ValidatedExpressionModel, budget: &mut Budget<'_>) -> Result<(), E> {
    let ir = model.ir();
    let mut expected = Edges::new();
    for d in &ir.declarations {
        if let Some(ExpressionBody::Expression(e)) = &d.body {
            ports(ir, &d.identity, e, budget)?;
        }
        match &d.signature {
            S::Binding(ty) => types(&d.identity, ty, false, &mut expected, budget)?,
            S::Definition(B::Record(fields)) => {
                for field in fields {
                    types(&d.identity, &field.ty, false, &mut expected, budget)?;
                }
            }
            S::Definition(B::Variant(alts)) => {
                for alt in alts {
                    types(&d.identity, &alt.ty, false, &mut expected, budget)?;
                }
            }
        }
        match &d.body {
            Some(ExpressionBody::Value(v)) => values(&d.identity, v, &mut expected, budget)?,
            Some(ExpressionBody::Expression(e)) => trees(&d.identity, e, &mut expected, budget)?,
            _ => {}
        }
    }
    for template in &ir.public_templates {
        ports(ir, &template.identity, &template.body, budget)?;
        for parameter in &template.parameters {
            if let Some(e) = &parameter.default {
                ports(ir, &template.identity, e, budget)?;
            }
        }
        types(
            &template.identity,
            &template.result_type,
            false,
            &mut expected,
            budget,
        )?;
        trees(&template.identity, &template.body, &mut expected, budget)?;
        for parameter in &template.parameters {
            types(
                &template.identity,
                &parameter.ty,
                false,
                &mut expected,
                budget,
            )?;
            if let Some(default) = &parameter.default {
                trees(&template.identity, default, &mut expected, budget)?;
            }
        }
    }
    let mut actual = Edges::new();
    let mut previous = None;
    for edge in &ir.provenance {
        owned_location(ir, &edge.from, edge.location, budget)?;
        let key = (&edge.from, edge.kind, &edge.to, edge.location);
        if previous.is_some_and(|p| p >= key) {
            return Err(E::Companion);
        }
        previous = Some(key);
        let target = ir
            .declarations
            .binary_search_by(|d| d.identity.cmp(&edge.to))
            .ok()
            .map(|i| &ir.declarations[i])
            .ok_or(E::Companion)?;
        if edge.from.module() != edge.to.module()
            && (!target.public
                || !reachable(
                    ir,
                    edge.from.module().module_name(),
                    edge.to.module().module_name(),
                    budget,
                )?)
        {
            return Err(E::Semantic);
        }
        if edge.kind == K::Value {
            reuse(ir, &edge.from, &edge.to, budget)?;
        }
        budget.step(actual.len() as u64 + 1)?;
        actual
            .insert((&edge.from, edge.kind, &edge.to))
            .map_err(|_| E::Limit)?;
    }
    for edge in &expected {
        budget.step(actual.len() as u64 + 1)?;
        if !actual.contains(edge) {
            return Err(E::Companion);
        }
    }
    for edge in &actual {
        budget.step(expected.len() as u64 + 1)?;
        if edge.1 != K::Value && !expected.contains(edge) {
            return Err(E::Companion);
        }
    }
    reuse_cycles(&actual, budget)
}

/// Value reuse is acyclic; structural references and guarded ports do not enter this graph.
fn reuse_cycles(actual: &Edges<'_>, budget: &mut Budget<'_>) -> Result<(), E> {
    for (from, kind, to) in actual {
        if *kind != K::Value {
            continue;
        }
        let mut seen = OrderedSet::new();
        let mut pending = Vec::new();
        pending.try_retain(1).map_err(|_| E::Limit)?;
        pending.push(*to);
        while let Some(owner) = pending.pop() {
            budget.step(actual.len() as u64 + seen.len() as u64 + 1)?;
            if owner == *from {
                return Err(E::Semantic);
            }
            if !seen.insert(owner).map_err(|_| E::Limit)? {
                continue;
            }
            for (a, k, b) in actual {
                if *a == owner && *k == K::Value {
                    pending.try_retain(1).map_err(|_| E::Limit)?;
                    pending.push(*b);
                }
            }
        }
    }
    Ok(())
}

/// Requires logical import closure for guarded ports without turning them into structural reuse.
fn ports(
    ir: &ExpressionProjectIr,
    owner: &ModuleSymbolIdentity,
    e: &neutral_ir::expression::Expression,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(1)?;
    match &e.node {
        N::Input(target) | N::Output { target, .. } => {
            if target.module() != owner.module()
                && !reachable(
                    ir,
                    owner.module().module_name(),
                    target.module().module_name(),
                    budget,
                )?
            {
                return Err(E::Semantic);
            }
        }
        N::Unary { operand, .. } | N::Text(operand) => ports(ir, owner, operand, budget)?,
        N::Binary { left, right, .. } => {
            ports(ir, owner, left, budget)?;
            ports(ir, owner, right, budget)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            ports(ir, owner, guard, budget)?;
            ports(ir, owner, then_branch, budget)?;
            ports(ir, owner, else_branch, budget)?;
        }
        N::Match { scrutinee, arms } => {
            ports(ir, owner, scrutinee, budget)?;
            for arm in arms {
                ports(ir, owner, &arm.body, budget)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                ports(ir, owner, item, budget)?;
            }
        }
        N::Record(fields) => {
            for (_, e) in fields {
                if let Some(e) = e {
                    ports(ir, owner, e, budget)?;
                }
            }
        }
        N::Variant { payload, .. } => ports(ir, owner, payload, budget)?,
        N::Value(_) | N::Bound { .. } => {}
    }
    Ok(())
}

/// Registers a nominal source dependency without following reference targets as values.
fn types<'a>(
    owner: &'a ModuleSymbolIdentity,
    ty: &'a T,
    reference: bool,
    edges: &mut Edges<'a>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(1)?;
    match ty {
        T::Nominal(target) => {
            budget.step(edges.len() as u64 + 1)?;
            edges
                .insert((
                    owner,
                    if reference { K::ReferenceType } else { K::Type },
                    target,
                ))
                .map_err(|_| E::Limit)?;
        }
        T::Ref(inner) => types(owner, inner, true, edges, budget)?,
        T::Nullable(inner) | T::List(inner) => types(owner, inner, reference, edges, budget)?,
        _ => {}
    }
    Ok(())
}
/// Registers actual immutable reference keys inside materialized values.
fn values<'a>(
    owner: &'a ModuleSymbolIdentity,
    value: &'a V,
    edges: &mut Edges<'a>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(1)?;
    match value {
        V::Reference(target) => {
            budget.step(edges.len() as u64 + 1)?;
            edges
                .insert((owner, K::Reference, target))
                .map_err(|_| E::Limit)?;
        }
        V::List(items) => {
            for item in items {
                values(owner, item, edges, budget)?;
            }
        }
        V::Record(fields) => {
            for (_, item) in fields {
                if let Some(item) = item {
                    values(owner, item, edges, budget)?;
                }
            }
        }
        V::Variant { payload, .. } => values(owner, payload, edges, budget)?,
        _ => {}
    }
    Ok(())
}
/// Retains typed-tree structural contracts without interpreting input/output as prerequisites.
fn trees<'a>(
    owner: &'a ModuleSymbolIdentity,
    e: &'a neutral_ir::expression::Expression,
    edges: &mut Edges<'a>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(1)?;
    types(owner, &e.ty, false, edges, budget)?;
    match &e.node {
        N::Value(v) => values(owner, v, edges, budget)?,
        N::Output { result_type, .. } => types(owner, result_type, false, edges, budget)?,
        N::Unary { operand, .. } | N::Text(operand) => trees(owner, operand, edges, budget)?,
        N::Binary { left, right, .. } => {
            trees(owner, left, edges, budget)?;
            trees(owner, right, edges, budget)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            trees(owner, guard, edges, budget)?;
            trees(owner, then_branch, edges, budget)?;
            trees(owner, else_branch, edges, budget)?;
        }
        N::Match { scrutinee, arms } => {
            trees(owner, scrutinee, edges, budget)?;
            for arm in arms {
                types(owner, &arm.payload_type, false, edges, budget)?;
                trees(owner, &arm.body, edges, budget)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                trees(owner, item, edges, budget)?;
            }
        }
        N::Record(fields) => {
            for (_, e) in fields {
                if let Some(e) = e {
                    trees(owner, e, edges, budget)?;
                }
            }
        }
        N::Variant { payload, .. } => trees(owner, payload, edges, budget)?,
        N::Input(_) | N::Bound { .. } => {}
    }
    Ok(())
}
/// Proves public cross-unit dependencies belong to the retained logical import closure.
fn reachable(
    ir: &ExpressionProjectIr,
    start: &str,
    goal: &str,
    budget: &mut Budget<'_>,
) -> Result<bool, E> {
    let mut seen = OrderedSet::new();
    let mut pending = Vec::new();
    pending.try_retain(1).map_err(|_| E::Limit)?;
    pending.push(start);
    while let Some(module) = pending.pop() {
        budget.step(ir.modules.len() as u64 + seen.len() as u64 + 1)?;
        if module == goal {
            return Ok(true);
        }
        if !seen.insert(module).map_err(|_| E::Limit)? {
            continue;
        }
        let i = ir
            .modules
            .binary_search_by(|m| m.identity.module_name().cmp(module))
            .map_err(|_| E::Semantic)?;
        for target in &ir.modules[i].imports {
            pending.try_retain(1).map_err(|_| E::Limit)?;
            pending.push(target);
        }
    }
    Ok(false)
}
/// Requires a claimed immutable reuse target to occur with its invariant nominal type.
fn reuse(
    ir: &ExpressionProjectIr,
    from: &ModuleSymbolIdentity,
    to: &ModuleSymbolIdentity,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let binding = |owner| {
        ir.declarations
            .binary_search_by(|d| d.identity.cmp(owner))
            .ok()
            .map(|i| &ir.declarations[i])
    };
    let left = binding(from).ok_or(E::Companion)?;
    let right = binding(to).ok_or(E::Companion)?;
    let (Some(ExpressionBody::Value(value)), Some(ExpressionBody::Value(target))) =
        (&left.body, &right.body)
    else {
        return Err(E::Semantic);
    };
    let (S::Binding(ty), S::Binding(target_ty)) = (&left.signature, &right.signature) else {
        return Err(E::Semantic);
    };
    if contains_reuse(ir, value, ty, target, target_ty, budget)? {
        Ok(())
    } else {
        Err(E::Semantic)
    }
}

/// Traverses independently preflighted materialized occurrences, paying before deep equality.
fn contains_reuse(
    ir: &ExpressionProjectIr,
    value: &V,
    ty: &T,
    target: &V,
    target_ty: &T,
    budget: &mut Budget<'_>,
) -> Result<bool, E> {
    use neutral_ir::expression::{
        inspect_expression_type_retention, inspect_expression_value_retention,
        profile::ExpressionLimits,
    };
    let limits = ExpressionLimits {
        work: budget.work,
        ..ExpressionLimits::hard()
    };
    let cost = inspect_expression_type_retention(target_ty, limits, budget.cancel).map_err(
        |e| match e {
            neutral_ir::expression::ExpressionShapeError::Cancelled => E::Cancelled,
            _ => E::Limit,
        },
    )?;
    budget.step(cost.work)?;
    if ty == target_ty {
        let cost = inspect_expression_value_retention(target, limits, budget.cancel).map_err(
            |e| match e {
                neutral_ir::expression::ExpressionShapeError::Cancelled => E::Cancelled,
                _ => E::Limit,
            },
        )?;
        budget.step(cost.work)?;
        if value == target {
            return Ok(true);
        }
    }
    let ty = if let T::Nullable(inner) = ty {
        inner.as_ref()
    } else {
        ty
    };
    match (ty, value) {
        (T::List(inner), V::List(values)) => {
            for value in values {
                if contains_reuse(ir, value, inner, target, target_ty, budget)? {
                    return Ok(true);
                }
            }
        }
        (T::Nominal(_) | T::VocabularyNominal { .. }, V::Record(values)) => {
            budget.step(ir.declarations.len() as u64 + ir.vocabularies.len() as u64 + 1)?;
            let B::Record(fields) = super::origins::body(ir, ty)? else {
                return Err(E::Semantic);
            };
            for ((_, value), field) in values.iter().zip(fields) {
                if let Some(value) = value
                    && contains_reuse(ir, value, &field.ty, target, target_ty, budget)?
                {
                    return Ok(true);
                }
            }
        }
        (T::Nominal(_) | T::VocabularyNominal { .. }, V::Variant { tag, payload }) => {
            budget.step(ir.declarations.len() as u64 + ir.vocabularies.len() as u64 + 1)?;
            let B::Variant(alternatives) = super::origins::body(ir, ty)? else {
                return Err(E::Semantic);
            };
            budget.step(alternatives.len() as u64 + tag.len() as u64 + 1)?;
            let alternative = alternatives
                .iter()
                .find(|a| a.tag == *tag)
                .ok_or(E::Semantic)?;
            return contains_reuse(ir, payload, &alternative.ty, target, target_ty, budget);
        }
        _ => {}
    }
    Ok(false)
}
