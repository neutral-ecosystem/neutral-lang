// SPDX-License-Identifier: Apache-2.0

//! Producer-only whole-binding folding reuses demand semantics without external bindings.

use super::{EvaluationError, EvaluationRequest, Flow, Mode, prepare};
use crate::ValidatedExpressionModel;
use neutral_core::CancellationToken;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::BindingValue,
    expression::{ExpressionBody, profile::ExpressionLimits},
};

/// A complete static value or the original deferred body, never a partial accepted value.
#[derive(Debug, Eq, PartialEq)]
pub enum NormalizedBinding {
    /// Fully static selected path calculated by the same kernel as explicit evaluation.
    Value(BindingValue),
    /// Owned normalized residual; unresolved guarded branches are not evaluated.
    Residual(ExpressionBody),
}

/// Folds one producer binding without acquiring or supplying any external value.
///
/// Private bindings are allowed here, but private demand keys never leave this API.
/// Unconditional static operands fold; unresolved guarded bodies remain unevaluated.
/// The result is not a published project and retained facts must be recomputed.
///
/// # Errors
/// Returns selected-path arithmetic, resource/cancellation or absent/non-binding errors.
pub fn normalize_binding(
    model: &ValidatedExpressionModel,
    selected: &ModuleSymbolIdentity,
    limits: ExpressionLimits,
    cancellation: &CancellationToken,
) -> Result<NormalizedBinding, EvaluationError> {
    let request = EvaluationRequest {
        selected,
        profiles: &model.ir().semantic_profiles,
        bindings: &[],
        associations: &[],
        limits,
    };
    let result = (|| {
        let (mut state, declaration, ty) = prepare(model, &request, cancellation, Mode::Producer)?;
        let result = match declaration
            .body
            .as_ref()
            .ok_or(EvaluationError::ExpressionType)?
        {
            ExpressionBody::Input => NormalizedBinding::Residual(ExpressionBody::Input),
            ExpressionBody::Value(value) => {
                NormalizedBinding::Value(state.budget.copy_value(value)?)
            }
            ExpressionBody::Expression(expression) => {
                let expression = state.fold(expression, 1)?;
                match expression.node {
                    neutral_ir::expression::ExpressionNode::Value(value) => {
                        NormalizedBinding::Value(value)
                    }
                    _ => NormalizedBinding::Residual(ExpressionBody::Expression(expression)),
                }
            }
        };
        let cost = match &result {
            NormalizedBinding::Value(value) => {
                state.checked(value, ty, &declaration.identity)?;
                neutral_ir::expression::inspect_expression_value_retention(
                    value,
                    state.budget.remaining(),
                    cancellation,
                )
            }
            NormalizedBinding::Residual(ExpressionBody::Expression(expression)) => {
                neutral_ir::expression::inspect_expression_retention(
                    expression,
                    state.budget.remaining(),
                    cancellation,
                )
            }
            NormalizedBinding::Residual(ExpressionBody::Input) => {
                state.budget.step(0)?;
                return Ok(result);
            }
            NormalizedBinding::Residual(ExpressionBody::Value(_)) => {
                return Err(EvaluationError::ExpressionType.into());
            }
        }
        .map_err(crate::budget::shape_error)?;
        state.budget.step(cost.work)?;
        state.budget.output(cost.retained_bytes)?;
        Ok(result)
    })();
    result.map_err(|flow| match flow {
        Flow::Failure(error) => error,
        Flow::Unavailable(_) => EvaluationError::InvalidBinding,
    })
}
