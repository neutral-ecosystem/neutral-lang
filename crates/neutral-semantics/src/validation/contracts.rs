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
    ValidatedCompositionScope, validate_composition_model, validate_composition_scope,
};

/// Reuses the independent vocabulary/default/embedding validator without fake binding values.
pub(super) fn scope(
    ir: &ExpressionProjectIr,
    limits: &SemanticLimits,
    cancel: &CancellationToken,
    budget: &mut Budget<'_>,
) -> Result<Shared<ValidatedCompositionScope>, E> {
    let inspection = inspect_composition_bodies(
        ir.declarations.iter().filter_map(|d| {
            if let S::Definition(b) = &d.signature {
                Some(b)
            } else {
                None
            }
        }),
        limits.project,
        limits.composition.policy(),
        cancel,
    )
    .map_err(|e| match e {
        neutral_ir::composition::project::CompositionShapeError::Cancelled => E::Cancelled,
        neutral_ir::composition::project::CompositionShapeError::Limit => E::Limit,
    })?;
    // Pay preflight, guarded copy and canonical equality before their proportional work.
    budget.step(inspection.work.checked_mul(3).ok_or(E::Limit)?)?;
    let catalogue = Shared::try_new(
        validate_composition_model(&ir.vocabularies, limits.composition, cancel)
            .map_err(E::from)?,
    )
    .map_err(|_| E::Limit)?;
    let mut sources = Vec::new();
    for d in &ir.declarations {
        if let S::Definition(body) = &d.signature {
            budget.step(1)?;
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
    let scope = Shared::try_new(
        validate_composition_scope(catalogue, sources, limits.composition, cancel)
            .map_err(E::from)?,
    )
    .map_err(|_| E::Limit)?;
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
