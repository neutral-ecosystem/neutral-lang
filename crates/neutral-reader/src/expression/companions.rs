// SPDX-License-Identifier: Apache-2.0

//! Checked original-byte evidence and resource companions; no acquisition or invented coordinates.

use super::ExpressionReadError as E;
use neutral_core::{CancellationToken, SourceLocation};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{BindingValue as V, project::CompositionSignature as S},
    expression::{
        Expression, ExpressionBody, ExpressionNode as N,
        project::{ExpressionProjectIr, inspect_expression_project_resources},
    },
    project::{ProjectResourceFacts, ProjectSourceMap},
};
use neutral_semantics::ValidatedExpressionModel;
mod origins;
mod provenance;

#[cfg(test)]
#[path = "../../tests/expression/companions.rs"]
mod tests;

/// Aggregate checked companion work, shared by source, origin and graph validation.
pub(super) struct Budget<'a> {
    /// Remaining request-local comparison/traversal work.
    pub(super) work: u64,
    /// Cooperative caller signal.
    pub(super) cancel: &'a CancellationToken,
}
impl Budget<'_> {
    /// Charges before proportional traversal and observes cancellation.
    pub(super) fn step(&mut self, n: u64) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        self.work = self.work.checked_sub(n).ok_or(E::Limit)?;
        Ok(())
    }
}

/// Checks every companion collection and independent shared retained counts.
pub(super) fn validate(
    model: &ValidatedExpressionModel,
    cancel: &CancellationToken,
) -> Result<(), E> {
    let ir = model.ir();
    let limits = model.limits();
    let facts = inspect_expression_project_resources(
        ir,
        limits.project,
        limits.composition.policy(),
        cancel,
    )
    .map_err(|e| match e {
        neutral_ir::composition::project::CompositionShapeError::Cancelled => E::Cancelled,
        neutral_ir::composition::project::CompositionShapeError::Limit => E::Limit,
    })?;
    if facts != ir.composition_resources {
        return Err(E::Resources);
    }
    let resources = ProjectResourceFacts {
        source_units: ir.sources.len() as u64,
        source_bytes: ir
            .sources
            .iter()
            .map(|s| s.byte_len)
            .try_fold(0_u64, u64::checked_add)
            .ok_or(E::Limit)?,
        vocabulary_units: ir.vocabulary_sources.len() as u64,
        vocabulary_bytes: ir
            .vocabulary_sources
            .iter()
            .map(|s| s.byte_len)
            .try_fold(0_u64, u64::checked_add)
            .ok_or(E::Limit)?,
        declarations: ir.declarations.len() as u64,
        import_edges: ir
            .modules
            .iter()
            .map(|m| m.imports.len() as u64)
            .try_fold(0_u64, u64::checked_add)
            .ok_or(E::Limit)?,
        value_nodes: facts.retained_value_nodes,
    };
    if resources != ir.resources {
        return Err(E::Resources);
    }
    if resources.source_bytes > limits.project.text_bytes
        || resources.vocabulary_bytes > limits.composition.captured_bytes
    {
        return Err(E::Limit);
    }
    let mut budget = Budget {
        work: limits.expression.work.min(limits.composition.work),
        cancel,
    };
    sources(ir, &mut budget)?;
    attributions(ir, &mut budget)?;
    origins::validate(model, &mut budget)?;
    provenance::validate(model, &mut budget)?;
    budget.step(0)
}

/// Finds an ordinary declaration or public template's original span with paid key work.
pub(super) fn mapping<'a>(
    ir: &'a ExpressionProjectIr,
    owner: &ModuleSymbolIdentity,
    budget: &mut Budget<'_>,
) -> Result<&'a ProjectSourceMap, E> {
    budget.step(
        (owner.declaration_name().len() as u64 + owner.module().module_name().len() as u64 + 1)
            .checked_mul(u64::from(ir.source_maps.len().bit_width()) + 1)
            .ok_or(E::Limit)?,
    )?;
    ir.source_maps
        .binary_search_by(|map| map.declaration.cmp(owner))
        .ok()
        .map(|i| &ir.source_maps[i])
        .ok_or(E::Companion)
}

/// Checks a real retained source digest/range without opening any file or URL.
pub(super) fn location(
    ir: &ExpressionProjectIr,
    loc: SourceLocation,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(ir.sources.len() as u64 + 1)?;
    if loc.span().is_empty()
        || !ir
            .sources
            .iter()
            .any(|s| s.digest == loc.source() && loc.span().end() <= s.byte_len)
    {
        return Err(E::Companion);
    }
    Ok(())
}
/// Requires attribution to belong to the claimed owner's original declaration, not any source.
/// Validated expansion arguments are contained by their application or definition spans.
pub(super) fn owned_location(
    ir: &ExpressionProjectIr,
    owner: &ModuleSymbolIdentity,
    loc: SourceLocation,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    location(ir, loc, budget)?;
    let map = mapping(ir, owner, budget)?;
    if contains(map.location, loc) {
        return Ok(());
    }
    budget.step(ir.expansion_attributions.len() as u64 + 1)?;
    if ir
        .expansion_attributions
        .iter()
        .any(|a| a.owner == *owner && (contains(a.application, loc) || contains(a.definition, loc)))
    {
        Ok(())
    } else {
        Err(E::Companion)
    }
}

/// Checks original-byte containment without interpreting source text.
fn contains(outer: SourceLocation, inner: SourceLocation) -> bool {
    outer.source() == inner.source()
        && inner.span().start() >= outer.span().start()
        && inner.span().end() <= outer.span().end()
}

/// Checks canonical source/catalogue coverage and original declaration/template maps.
fn sources(ir: &ExpressionProjectIr, budget: &mut Budget<'_>) -> Result<(), E> {
    if ir.modules.is_empty()
        || ir.sources.len() != ir.modules.len()
        || ir.vocabulary_sources.len() != ir.vocabularies.len()
        || ir.source_maps.len()
            != ir
                .declarations
                .len()
                .checked_add(ir.public_templates.len())
                .ok_or(E::Limit)?
    {
        return Err(E::Companion);
    }
    let mut ids = neutral_core::ordered::OrderedSet::new();
    for (source, module) in ir.sources.iter().zip(&ir.modules) {
        budget.step(source.source_id.len() as u64 + ids.len() as u64 + 1)?;
        if source.module != module.identity.module_name()
            || source.source_id.is_empty()
            || source.byte_len == 0
            || !ids
                .insert(source.source_id.as_str())
                .map_err(|_| E::Limit)?
        {
            return Err(E::Companion);
        }
    }
    for (source, bundle) in ir.vocabulary_sources.iter().zip(&ir.vocabularies) {
        budget.step(1)?;
        if source.identity != bundle.identity.identity()
            || source.version != bundle.identity.version()
            || source.digest != bundle.identity.content_digest()
            || source.byte_len == 0
        {
            return Err(E::Companion);
        }
    }
    let mut previous = None;
    for map in &ir.source_maps {
        budget.step(map.declaration.declaration_name().len() as u64 + 1)?;
        if previous.is_some_and(|p| p >= &map.declaration) {
            return Err(E::Companion);
        }
        previous = Some(&map.declaration);
        let ordinary = ir
            .declarations
            .binary_search_by(|d| d.identity.cmp(&map.declaration))
            .is_ok();
        let template = ir
            .public_templates
            .binary_search_by(|d| d.identity.cmp(&map.declaration))
            .is_ok();
        if !ordinary && !template {
            return Err(E::Companion);
        }
        let source = ir
            .sources
            .binary_search_by(|s| {
                s.module
                    .as_str()
                    .cmp(map.declaration.module().module_name())
            })
            .ok()
            .map(|i| &ir.sources[i])
            .ok_or(E::Companion)?;
        if map.location.source() != source.digest
            || map.location.span().is_empty()
            || map.location.span().end() > source.byte_len
        {
            return Err(E::Companion);
        }
    }
    Ok(())
}

/// Visits a canonical node path; no path is accepted merely because its owner exists.
fn node<'a>(
    mut root: &'a Expression,
    path: &[u64],
    budget: &mut Budget<'_>,
) -> Result<&'a Expression, E> {
    if path.len() > neutral_ir::expression::profile::MAX_DEPTH {
        return Err(E::Limit);
    }
    for index in path {
        budget.step(1)?;
        let index = usize::try_from(*index).map_err(|_| E::Companion)?;
        root = match &root.node {
            N::Unary { operand, .. } | N::Text(operand) if index == 0 => operand,
            N::Binary { left, right, .. } => match index {
                0 => left,
                1 => right,
                _ => return Err(E::Companion),
            },
            N::If {
                guard,
                then_branch,
                else_branch,
            } => match index {
                0 => guard,
                1 => then_branch,
                2 => else_branch,
                _ => return Err(E::Companion),
            },
            N::Match { scrutinee, .. } if index == 0 => scrutinee,
            N::Match { arms, .. } => &arms.get(index - 1).ok_or(E::Companion)?.body,
            N::List(items) | N::Interpolate(items) => items.get(index).ok_or(E::Companion)?,
            N::Record(fields) => fields
                .get(index)
                .and_then(|(_, e)| e.as_ref())
                .ok_or(E::Companion)?,
            N::Variant { payload, .. } if index == 0 => payload,
            _ => return Err(E::Companion),
        };
    }
    Ok(root)
}
/// Checks paths, canonical evidence order and real expansion lineage.
fn attributions(ir: &ExpressionProjectIr, budget: &mut Budget<'_>) -> Result<(), E> {
    expansion_attributions(ir, budget)?;
    let mut previous = None;
    for attribution in &ir.expression_attributions {
        budget.step(
            (attribution.owner.declaration_name().len() as u64
                + attribution.owner.module().module_name().len() as u64
                + attribution.owner.module().language_behavior_version().len() as u64
                + attribution.node_path.len() as u64
                + 1)
            .checked_mul(u64::from(ir.source_maps.len().bit_width()) + 1)
            .ok_or(E::Limit)?,
        )?;
        let key = (&attribution.owner, &attribution.node_path);
        if previous.is_some_and(|p| p >= key) {
            return Err(E::Companion);
        }
        previous = Some(key);
        let root = ir
            .declarations
            .binary_search_by(|d| d.identity.cmp(&attribution.owner))
            .ok()
            .and_then(|i| match &ir.declarations[i].body {
                Some(ExpressionBody::Expression(e)) => Some(e),
                _ => None,
            })
            .or_else(|| {
                ir.public_templates
                    .binary_search_by(|t| t.identity.cmp(&attribution.owner))
                    .ok()
                    .map(|i| &ir.public_templates[i].body)
            })
            .ok_or(E::Companion)?;
        node(root, &attribution.node_path, budget)?;
        if let Some(loc) = attribution.occurrence {
            owned_location(ir, &attribution.owner, loc, budget)?;
        }
        let mut prior = None;
        for loc in &attribution.related_public_locations {
            if prior.is_some_and(|p| p >= loc) {
                return Err(E::Companion);
            }
            prior = Some(loc);
            location(ir, *loc, budget)?;
            budget.step(ir.source_maps.len() as u64 + 1)?;
            let mut public = false;
            for map in &ir.source_maps {
                if !contains(map.location, *loc) {
                    continue;
                }
                let visible = ir
                    .declarations
                    .binary_search_by(|d| d.identity.cmp(&map.declaration))
                    .is_ok_and(|i| ir.declarations[i].public)
                    || ir
                        .public_templates
                        .binary_search_by(|t| t.identity.cmp(&map.declaration))
                        .is_ok();
                if !visible {
                    return Err(E::Companion);
                }
                public = true;
            }
            if !public {
                return Err(E::Companion);
            }
        }
    }
    Ok(())
}

/// Checks retained lineage before expression/provenance locations may rely on it.
fn expansion_attributions(ir: &ExpressionProjectIr, budget: &mut Budget<'_>) -> Result<(), E> {
    let mut previous = None;
    for attribution in &ir.expansion_attributions {
        budget.step(
            (attribution.owner.declaration_name().len() as u64
                + attribution.owner.module().module_name().len() as u64
                + attribution.owner.module().language_behavior_version().len() as u64
                + attribution.arguments.len() as u64
                + attribution.original_axis_indices.len() as u64
                + 1)
            .checked_mul(u64::from(ir.source_maps.len().bit_width()) + 1)
            .ok_or(E::Limit)?,
        )?;
        let key = (
            &attribution.owner,
            attribution.application,
            attribution.definition,
            &attribution.arguments,
            &attribution.original_axis_indices,
        );
        if previous.is_some_and(|p| p >= key)
            || ir
                .declarations
                .binary_search_by(|d| d.identity.cmp(&attribution.owner))
                .is_err()
        {
            return Err(E::Companion);
        }
        previous = Some(key);
        location(ir, attribution.application, budget)?;
        location(ir, attribution.definition, budget)?;
        let application_map = mapping(ir, &attribution.owner, budget)?;
        if !contains(application_map.location, attribution.application) {
            return Err(E::Companion);
        }
        for template in &ir.public_templates {
            let definition_map = mapping(ir, &template.identity, budget)?;
            if definition_map.location == attribution.definition
                && template.parameters.len() != attribution.arguments.len()
            {
                return Err(E::Companion);
            }
        }
        for loc in &attribution.arguments {
            location(ir, *loc, budget)?;
            if !contains(attribution.application, *loc) && !contains(attribution.definition, *loc) {
                return Err(E::Companion);
            }
        }
        if attribution.arguments.len() as u64 > ir.expansion_limits.parameters
            || attribution.original_axis_indices.len() as u64 > ir.expansion_limits.axes
        {
            return Err(E::Limit);
        }
        if attribution.original_axis_indices.iter().any(|index| {
            *index
                >= ir
                    .expansion_limits
                    .axis_items
                    .min(neutral_ir::expression::templates::ExpansionLimits::hard().axis_items)
        }) {
            return Err(E::Limit);
        }
    }
    Ok(())
}
