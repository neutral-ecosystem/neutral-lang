// SPDX-License-Identifier: Apache-2.0

//! Borrowed nominal preflight precedes every copied source contract.

use super::{Budget, E};
use crate::SemanticLimits;
use neutral_core::{
    CancellationToken,
    allocation::{RetainCapacity, Shared, TryClone, text},
};
use neutral_ir::{
    composition::{
        CompositionDefinition, SourceCompositionDefinition,
        project::{CompositionSignature as S, inspect_composition_bodies},
    },
    expression::project::ExpressionProjectIr,
};
use neutral_vocabulary::composition::{
    ValidatedCompositionScope, validate_composition_model_with_budget,
    validate_composition_scope_with_budget,
};

/// Reuses the independent vocabulary/default/embedding validator without fake binding values.
#[allow(clippy::too_many_lines)] // Preflight, canonicalization and comparison share one atomic budget.
pub(super) fn scope(
    ir: &ExpressionProjectIr,
    limits: &SemanticLimits,
    cancel: &CancellationToken,
    budget: &mut Budget<'_>,
) -> Result<Shared<ValidatedCompositionScope>, E> {
    let mut policy = limits.composition.policy();
    policy.work = policy.work.min(budget.work);
    let inspection = inspect_composition_bodies(
        ir.declarations.iter().filter_map(|d| {
            if let S::Definition(b) = &d.signature {
                Some(b)
            } else {
                None
            }
        }),
        limits.project,
        policy,
        cancel,
    )
    .map_err(|e| match e {
        neutral_ir::composition::project::CompositionShapeError::Cancelled => E::Cancelled,
        neutral_ir::composition::project::CompositionShapeError::Limit => E::Limit,
    })?;
    // Pay preflight, guarded copy and canonical equality before their proportional work.
    budget.step(inspection.work.checked_mul(3).ok_or(E::Limit)?)?;
    budget.storage(inspection.retained_bytes.checked_mul(3).ok_or(E::Limit)?)?;
    budget.storage(
        (std::mem::size_of::<ValidatedCompositionScope>()
            + std::mem::size_of::<neutral_vocabulary::composition::ValidatedComposition>())
            as u64,
    )?;
    policy.work = policy.work.min(budget.work);
    let catalogue_copy = neutral_ir::composition::project::inspect_composition_catalogue(
        &ir.vocabularies,
        limits.project,
        policy,
        cancel,
    )
    .map_err(|_| {
        if cancel.is_cancelled() {
            E::Cancelled
        } else {
            E::Limit
        }
    })?;
    budget.step(catalogue_copy.work.checked_mul(3).ok_or(E::Limit)?)?;
    budget.storage(
        catalogue_copy
            .retained_bytes
            .checked_mul(3)
            .ok_or(E::Limit)?,
    )?;
    let mut remaining = budget.work;
    let before = remaining;
    let validated_catalogue = validate_composition_model_with_budget(
        &ir.vocabularies,
        limits.composition,
        &mut remaining,
        cancel,
    )
    .map_err(E::from)?;
    budget.step(before - remaining)?;
    let catalogue = Shared::try_new(validated_catalogue).map_err(|_| E::Limit)?;
    let mut sources = Vec::new();
    for d in &ir.declarations {
        if let S::Definition(body) = &d.signature {
            budget.step(1)?;
            budget.symbol(&d.identity)?;
            budget.storage(
                std::mem::size_of::<SourceCompositionDefinition>() as u64
                    + d.identity.declaration_name().len() as u64,
            )?;
            sources.try_retain_exact(1).map_err(|_| E::Limit)?;
            sources.push(SourceCompositionDefinition {
                owner: d.identity.try_clone().map_err(|_| E::Limit)?,
                definition: CompositionDefinition {
                    name: text(d.identity.declaration_name()).map_err(|_| E::Limit)?,
                    public: d.public,
                    body: body.try_clone().map_err(|_| E::Limit)?,
                },
            });
        }
    }
    let mut remaining = budget.work;
    let before = remaining;
    let validated_scope = validate_composition_scope_with_budget(
        catalogue,
        sources,
        limits.composition,
        &mut remaining,
        cancel,
    )
    .map_err(E::from)?;
    budget.step(before - remaining)?;
    let scope = Shared::try_new(validated_scope).map_err(|_| E::Limit)?;
    for (raw, validated) in ir
        .declarations
        .iter()
        .filter_map(|d| {
            if let S::Definition(body) = &d.signature {
                Some(body)
            } else {
                None
            }
        })
        .zip(scope.sources())
    {
        budget.step(1)?;
        if raw != &validated.definition.body {
            return Err(E::Malformed);
        }
    }
    Ok(scope)
}
