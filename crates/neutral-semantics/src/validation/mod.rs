// SPDX-License-Identifier: Apache-2.0

//! Independent all-branch semantic checks over complete raw data, never source or wire fallback.

use crate::model::{
    AvailabilityObligation, ExpressionValidationError, RestrictionObligation, SemanticLimits,
    ValidatedExpressionModel,
};
use neutral_core::{CancellationToken, allocation::Shared};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{CompositionBody as B, project::CompositionSignature as S},
    expression::{
        ExpressionShapeError,
        profile::{self, ExpressionLimits},
        project::{ExpressionProjectIr, ExpressionResourceFacts},
        templates::ExpansionLimits,
    },
    language::{is_protected_name, is_snake_name},
};
use neutral_vocabulary::composition::{
    CompositionError, CompositionLimits, ValidatedCompositionScope,
};
mod contracts;
mod declarations;
mod matching;
mod operators;
mod selectors;
mod trees;
mod types;
mod values;
use ExpressionValidationError as E;

/// Independently retypes semantic bodies before publishing model authority.
///
/// # Errors
/// Rejects unknown profiles, malformed/private/forged types, stale facts, independent
/// bounds, allocation or cancellation. Public templates are currently unavailable;
/// complete source companions and artifact identities require a separate reader boundary.
pub fn validate_expression_model(
    ir: Shared<ExpressionProjectIr>,
    caller: SemanticLimits,
    cancel: &CancellationToken,
) -> Result<ValidatedExpressionModel, E> {
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    if ir.schema != profile::PROJECT_IR_SCHEMA || !ir.semantic_profiles.matches_frozen() {
        return Err(E::Unsupported);
    }
    if !ir.public_templates.is_empty() {
        return Err(E::Unsupported);
    }
    let limits = SemanticLimits {
        project: caller.project.intersect(ir.limits),
        composition: CompositionLimits::from_policy(
            caller.composition.json,
            caller.composition.policy().intersect(ir.composition_limits),
        ),
        expression: caller
            .expression
            .intersect(ir.expression_limits)
            .intersect(ExpressionLimits::hard()),
        expansion: caller
            .expansion
            .intersect(ir.expansion_limits)
            .intersect(ExpansionLimits::hard()),
    };
    limits.composition.validate().map_err(E::from)?;
    if limits.expression.values().contains(&0)
        || limits.expansion.values().contains(&0)
        || [
            limits.project.modules,
            limits.project.declarations,
            limits.project.import_edges,
            limits.project.nodes,
            limits.project.text_bytes,
            limits.project.artifact_bytes,
        ]
        .contains(&0)
    {
        return Err(E::Limit);
    }
    let mut budget = Budget {
        work: limits.expression.work.min(limits.composition.work),
        bytes: limits
            .expression
            .retained_bytes
            .min(limits.expression.output_bytes),
        cancel,
    };
    budget.step(1)?;
    if ir.declarations.len() as u64 > limits.project.declarations.min(limits.expression.bindings)
        || ir.modules.len() as u64 > limits.project.modules
    {
        return Err(E::Limit);
    }
    owners(&ir, limits.project.import_edges, &mut budget)?;
    let scope = contracts::scope(&ir, &limits, cancel, &mut budget)?;
    let mut state = State {
        ir: &ir,
        scope: &scope,
        limits,
        budget,
        facts: ExpressionResourceFacts::default(),
        availability: Vec::new(),
        restrictions: Vec::new(),
        path: Vec::new(),
        frames: [None; profile::MAX_DEPTH],
        frame_count: 0,
    };
    for d in &ir.declarations {
        state.declaration(d)?;
    }
    if state.facts != ir.expression_resources {
        return Err(E::Resources);
    }
    state.budget.step(1)?;
    let facts = state.facts;
    let availability = state.availability;
    let restrictions = state.restrictions;
    Ok(ValidatedExpressionModel {
        ir,
        scope,
        limits,
        facts,
        availability,
        restrictions,
    })
}

/// Maps shared contract failures without retaining arbitrary diagnostic text.
impl From<CompositionError> for E {
    /// Preserves shared limit/cancellation classes while hiding contract payloads.
    fn from(error: CompositionError) -> Self {
        match error {
            CompositionError::Cancelled => E::Cancelled,
            CompositionError::Limit
            | CompositionError::InvalidLimits
            | CompositionError::Allocation => E::Limit,
            CompositionError::PrivateType | CompositionError::UnknownType => E::UnknownOrPrivate,
            _ => E::Restriction,
        }
    }
}
/// Maps raw preflight failure classes without granting semantic authority.
fn shape_error(error: ExpressionShapeError) -> E {
    match error {
        ExpressionShapeError::Cancelled => E::Cancelled,
        ExpressionShapeError::Limit => E::Limit,
        ExpressionShapeError::Malformed => E::Malformed,
    }
}
/// Checks canonical owners/modules before indexes, comparisons or clone consumers.
fn owners(ir: &ExpressionProjectIr, import_limit: u64, budget: &mut Budget<'_>) -> Result<(), E> {
    let mut previous = None;
    for m in &ir.modules {
        budget.text(m.identity.module_name())?;
        if m.identity.language_behavior_version() != neutral_core::profile::V1_SOURCE_PROFILE
            || !m.identity.module_name().split("::").all(is_snake_name)
            || previous.is_some_and(|p| p >= &m.identity)
        {
            return Err(E::Malformed);
        }
        previous = Some(&m.identity);
    }
    let mut imports = 0_u64;
    for m in &ir.modules {
        imports = imports
            .checked_add(m.imports.len() as u64)
            .ok_or(E::Limit)?;
        if imports > import_limit {
            return Err(E::Limit);
        }
        let mut prior = None;
        for import in &m.imports {
            budget.text(import)?;
            budget.lookup(import.len() as u64 + 1, ir.modules.len())?;
            if prior.is_some_and(|p| p >= import)
                || import == m.identity.module_name()
                || ir
                    .modules
                    .binary_search_by(|target| target.identity.module_name().cmp(import))
                    .is_err()
            {
                return Err(E::Malformed);
            }
            prior = Some(import);
        }
    }
    let mut previous = None;
    for d in &ir.declarations {
        budget.symbol(&d.identity)?;
        budget.lookup(
            (d.identity.module().module_name().len()
                + d.identity.module().language_behavior_version().len()
                + 1) as u64,
            ir.modules.len(),
        )?;
        if previous.is_some_and(|p| p >= &d.identity)
            || ir
                .modules
                .binary_search_by(|m| m.identity.cmp(d.identity.module()))
                .is_err()
        {
            return Err(E::Malformed);
        }
        if matches!(d.signature, S::Binding(_))
            && (!is_snake_name(d.identity.declaration_name())
                || is_protected_name(d.identity.declaration_name()))
        {
            return Err(E::Malformed);
        }
        previous = Some(&d.identity);
    }
    Ok(())
}

/// Aggregate request work and owned expression/obligation storage.
struct Budget<'a> {
    /// Remaining checked work shared by every root.
    work: u64,
    /// Remaining retained expression and obligation bytes.
    bytes: u64,
    /// Explicit cooperative cancellation token.
    cancel: &'a CancellationToken,
}
impl Budget<'_> {
    /// Pays worst-case key comparison work before binary search.
    fn lookup(&mut self, key_bytes: u64, items: usize) -> Result<(), E> {
        self.step(
            key_bytes
                .checked_mul(u64::from(items.bit_width()) + 1)
                .ok_or(E::Limit)?,
        )
    }
    /// Charges before traversal/allocation and at final publication.
    fn step(&mut self, n: u64) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        self.work = self.work.checked_sub(n).ok_or(E::Limit)?;
        Ok(())
    }
    /// Bounds keys before proportional comparison or retention.
    fn text(&mut self, s: &str) -> Result<(), E> {
        self.step(s.len() as u64 + 1)
    }
    /// Bounds complete stable symbol keys before comparison/copy.
    fn symbol(&mut self, s: &ModuleSymbolIdentity) -> Result<(), E> {
        self.text(s.module().language_behavior_version())?;
        self.text(s.module().module_name())?;
        self.text(s.declaration_name())
    }
    /// Charges retained output before fallible reservation/copy.
    fn storage(&mut self, n: u64) -> Result<(), E> {
        self.step(n + 1)?;
        self.bytes = self.bytes.checked_sub(n).ok_or(E::Limit)?;
        Ok(())
    }
}
/// Request-local all-branch retyping state; no supplied graph grants authority.
struct State<'a> {
    /// Complete raw semantic input.
    ir: &'a ExpressionProjectIr,
    /// Independently validated nominal contracts.
    scope: &'a ValidatedCompositionScope,
    /// Effective independent caller/producer controls.
    limits: SemanticLimits,
    /// Shared aggregate work/storage counters.
    budget: Budget<'a>,
    /// Independently accumulated retained facts.
    facts: ExpressionResourceFacts,
    /// Derived availability conditions.
    availability: Vec<AvailabilityObligation>,
    /// Derived unresolved restriction conditions.
    restrictions: Vec<RestrictionObligation>,
    /// Current canonical node occurrence path.
    path: Vec<u64>,
    /// Match payload frames; public templates remain explicitly unsupported for now.
    frames: [Option<&'a neutral_ir::project_interface::ProjectPublicType>; profile::MAX_DEPTH],
    /// Active lexical frames.
    frame_count: usize,
}
