// SPDX-License-Identifier: Apache-2.0

//! Explicit pure evaluation over independently typed models; no scheduler, callback or acquisition.

use crate::{
    ValidatedExpressionModel,
    budget::{Budget, OperationError},
    validation,
};
use neutral_core::CancellationToken;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::BindingValue as V,
    expression::{
        Expression, ExpressionBody, SelectorSegment,
        profile::{ExpressionLimits, SemanticProfiles},
    },
    project_interface::ProjectPublicType as T,
};
mod bindings;
mod folding;
mod keys;
mod normalization;
mod selectors;
mod walk;
pub use bindings::{Binding, BindingKey, BindingState, OutputAssociation};
pub use normalization::{NormalizedBinding, normalize_binding};

/// Stable safe evaluator failures without raw values, private names or host details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    /// Exact semantic profiles are not supported.
    Unsupported,
    /// Unknown/duplicate key, malformed envelope or association registry.
    InvalidBinding,
    /// Explicit value/type/visibility/result association does not match its contract.
    BindingType,
    /// A real nominal restriction or selector obligation was violated.
    Restriction,
    /// A demanded explicit binding has failed.
    BindingFailed,
    /// Selected subject is absent/private or not an ordinary binding.
    UnknownEntry,
    /// An evaluated denominator is zero.
    DivisionByZero,
    /// Typed operation invariant failed without coercion or partial results.
    ExpressionType,
    /// Independent count, work, storage, numerical growth or allocation exhausted.
    Limit,
    /// Explicit cooperative cancellation.
    Cancelled,
}
impl EvaluationError {
    /// Returns the frozen expression/evaluator diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "NEU-EVAL-001",
            Self::InvalidBinding => "NEU-EVAL-002",
            Self::BindingType => "NEU-EVAL-003",
            Self::Restriction => "NEU-EVAL-004",
            Self::BindingFailed => "NEU-EVAL-005",
            Self::Limit => "NEU-EVAL-006",
            Self::Cancelled => "NEU-EVAL-007",
            Self::UnknownEntry => "NEU-EXP-003",
            Self::ExpressionType => "NEU-EXP-004",
            Self::DivisionByZero => "NEU-EXP-006",
        }
    }
}
impl From<OperationError> for EvaluationError {
    /// Preserves pure-operation errors without exposing their operands.
    fn from(error: OperationError) -> Self {
        match error {
            OperationError::DivisionByZero => Self::DivisionByZero,
            OperationError::Limit => Self::Limit,
            OperationError::Cancelled => Self::Cancelled,
            OperationError::TypeMismatch => Self::ExpressionType,
        }
    }
}

/// One explicit binding generation; omitted keys remain missing, not implicit nulls.
pub struct EvaluationRequest<'a> {
    /// Exact public ordinary binding selected for evaluation.
    pub selected: &'a ModuleSymbolIdentity,
    /// Exact requested expression/evaluator/expansion/decimal tuple.
    pub profiles: &'a SemanticProfiles,
    /// Complete supplied envelope, validated even for skipped branches.
    pub bindings: &'a [Binding<'a>],
    /// Explicit consumer-owned producer/result associations; no inference from field names.
    pub associations: &'a [OutputAssociation<'a>],
    /// Positive independent caller bounds, intersected with model/hard controls.
    pub limits: ExpressionLimits,
}

/// Complete value or demand/failure information; never partial accepted output.
#[derive(Debug, Eq, PartialEq)]
pub enum EvaluationResult<'m> {
    /// A complete calculated value checked against its exact declared contract.
    Value {
        /// Exact independently validated result type.
        ty: &'m T,
        /// Fully owned result data.
        value: V,
    },
    /// Sorted unique keys demanded on selected paths, not scheduling prerequisites.
    RequiredBindings(Vec<BindingKey<'m>>),
    /// A selected path demanded a well-formed unavailable binding.
    Unavailable(BindingKey<'m>),
    /// Safe structured failure with no value or untrusted diagnostic text.
    Failure(EvaluationError),
}

/// Evaluates a public ordinary binding using only explicit validated caller data.
///
/// No input is acquired, template invoked, backend scheduled or source span invented.
/// Model typing is not complete artifact/identity/companion validation.
#[must_use]
pub fn evaluate<'m>(
    model: &'m ValidatedExpressionModel,
    request: &EvaluationRequest<'_>,
    cancellation: &CancellationToken,
) -> EvaluationResult<'m> {
    match run(model, request, cancellation, Mode::Consumer) {
        Ok(value) => value,
        Err(Flow::Failure(error)) => EvaluationResult::Failure(error),
        Err(Flow::Unavailable(key)) => EvaluationResult::Unavailable(key),
    }
}

/// Producer folding can inspect private bindings but cannot accept consumer envelopes.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    /// Consumer calls may select only public entries and must validate output associations.
    Consumer,
    /// Producer folding has no supplied values and cannot expose private binding demands.
    Producer,
}

/// Internal short-circuit control; failures/unavailable states stop ordinary traversal atomically.
enum Flow<'m> {
    /// Safe failure without partially calculated data.
    Failure(EvaluationError),
    /// Exact demanded, known-public unavailable port.
    Unavailable(BindingKey<'m>),
}
impl From<OperationError> for Flow<'_> {
    /// Converts resource/math failures to the closed public result boundary.
    fn from(error: OperationError) -> Self {
        Self::Failure(error.into())
    }
}
impl From<EvaluationError> for Flow<'_> {
    /// Preserves explicit boundary failure classifications.
    fn from(error: EvaluationError) -> Self {
        Self::Failure(error)
    }
}

/// Request-local port and demand state; no persistent cache or cross-generation mutation.
struct State<'m, 'r> {
    /// Separates compiler-only folding from the public consumer trust boundary.
    mode: Mode,
    /// Exact public ordinary owner; source attribution is not invented here.
    owner: &'m ModuleSymbolIdentity,
    /// Independently checked model authority.
    model: &'m ValidatedExpressionModel,
    /// Explicit immutable binding generation.
    request: &'r EvaluationRequest<'r>,
    /// Shared work/storage/numeric controls.
    budget: Budget<'r>,
    /// Known public input/output ports, deduplicated before envelope checking.
    ports: Vec<bindings::Port<'m>>,
    /// Missing keys reached along actually selected paths.
    missing: Vec<BindingKey<'m>>,
    /// Owned match payload values; templates cannot be called by consumers.
    frames: Vec<V>,
}

/// Validates envelopes first, then walks only demanded operands and checks complete results.
fn run<'m>(
    model: &'m ValidatedExpressionModel,
    request: &EvaluationRequest<'_>,
    cancel: &CancellationToken,
    mode: Mode,
) -> Result<EvaluationResult<'m>, Flow<'m>> {
    let (mut state, declaration, ty) = prepare(model, request, cancel, mode)?;
    let value = match declaration
        .body
        .as_ref()
        .ok_or(EvaluationError::ExpressionType)?
    {
        ExpressionBody::Input => state.demand(BindingKey::Input(&declaration.identity))?,
        ExpressionBody::Value(value) => Some(state.budget.copy_value(value)?),
        ExpressionBody::Expression(expression) => state.walk(expression, 1)?,
    };
    if let Some(value) = value {
        state.checked(&value, ty, &declaration.identity)?;
        let cost = neutral_ir::expression::inspect_expression_value_retention(
            &value,
            state.budget.remaining(),
            cancel,
        )
        .map_err(crate::budget::shape_error)?;
        state.budget.step(cost.work)?;
        state.budget.output(cost.retained_bytes)?;
        return Ok(EvaluationResult::Value { ty, value });
    }
    let key_work = state.ports.iter().map(|p| p.cost).max().unwrap_or(1);
    let sorting = key_work
        .checked_mul(state.missing.len() as u64)
        .and_then(|n| n.checked_mul(u64::from(state.missing.len().bit_width()) + 1))
        .ok_or(OperationError::Limit)?;
    state.budget.step(sorting)?;
    state.missing.sort_unstable_by(keys::compare);
    state.missing.dedup();
    state.budget.step(0)?;
    Ok(EvaluationResult::RequiredBindings(state.missing))
}

/// Shares bounded entry selection and envelope preparation between evaluation and folding.
fn prepare<'m, 'r>(
    model: &'m ValidatedExpressionModel,
    request: &'r EvaluationRequest<'r>,
    cancel: &'r CancellationToken,
    mode: Mode,
) -> Result<
    (
        State<'m, 'r>,
        &'m neutral_ir::expression::project::ExpressionDeclaration,
        &'m T,
    ),
    Flow<'m>,
> {
    if !request.profiles.matches_frozen() || request.profiles != &model.ir.semantic_profiles {
        return Err(EvaluationError::Unsupported.into());
    }
    let mut limits = request.limits.intersect(model.limits.expression);
    limits.work = limits.work.min(model.limits.composition.work);
    let mut budget = Budget::new(limits, cancel)?;
    bindings::symbol(request.selected, &mut budget)?;
    budget.step(
        (model.ir.declarations.len() as u64 + 1)
            .checked_mul(request.selected.declaration_name().len() as u64 + 1)
            .ok_or(OperationError::Limit)?,
    )?;
    let index = model
        .ir
        .declarations
        .binary_search_by(|d| d.identity.cmp(request.selected))
        .map_err(|_| EvaluationError::UnknownEntry)?;
    let declaration = &model.ir.declarations[index];
    let neutral_ir::composition::project::CompositionSignature::Binding(ty) =
        &declaration.signature
    else {
        return Err(EvaluationError::UnknownEntry.into());
    };
    if mode == Mode::Consumer && !declaration.public {
        return Err(EvaluationError::UnknownEntry.into());
    }
    if let Some(ExpressionBody::Expression(expression)) = &declaration.body {
        let cost = neutral_ir::expression::inspect_expression_retention(
            expression,
            budget.input_limits(),
            cancel,
        )
        .map_err(crate::budget::shape_error)?;
        budget.step(cost.work)?;
    }
    let mut state = State {
        mode,
        owner: &declaration.identity,
        model,
        request,
        budget,
        ports: Vec::new(),
        missing: Vec::new(),
        frames: Vec::new(),
    };
    state.registry()?;
    state.envelopes()?;
    Ok((state, declaration, ty))
}

impl State<'_, '_> {
    /// Checks actual value types/restrictions using shared semantic rules and aggregate work.
    fn checked(
        &mut self,
        value: &V,
        ty: &T,
        owner: &ModuleSymbolIdentity,
    ) -> Result<(), EvaluationError> {
        let cost = neutral_ir::expression::inspect_expression_value_retention(
            value,
            self.budget.input_limits(),
            self.budget.cancellation(),
        )
        .map_err(crate::budget::shape_error)?;
        self.budget.step(cost.work)?;
        let work = validation::check_materialized_value(
            self.model,
            value,
            ty,
            owner,
            self.mode == Mode::Consumer,
            self.budget.remaining(),
            self.budget.cancellation(),
        )
        .map_err(|error| match error {
            crate::ExpressionValidationError::Cancelled => EvaluationError::Cancelled,
            crate::ExpressionValidationError::Limit => EvaluationError::Limit,
            crate::ExpressionValidationError::Restriction => EvaluationError::Restriction,
            _ => EvaluationError::BindingType,
        })?;
        self.budget.step(work)?;
        Ok(())
    }
}
