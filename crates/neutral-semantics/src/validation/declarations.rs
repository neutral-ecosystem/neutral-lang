// SPDX-License-Identifier: Apache-2.0

//! Retypes each tagged binding body under shared aggregate expression limits.

use super::{E, S, State, shape_error};
use neutral_ir::expression::{
    ExpressionBody, profile::ExpressionLimits, project::ExpressionDeclaration,
};

impl<'a> State<'a> {
    /// Checks one real declaration; definition bodies never masquerade as external inputs.
    pub(super) fn declaration(&mut self, d: &'a ExpressionDeclaration) -> Result<(), E> {
        self.budget.step(1)?;
        d.validate_body_shape().map_err(shape_error)?;
        if let S::Binding(ty) = &d.signature {
            self.ty(ty, d.public, &d.identity, 0)?;
            match d.body.as_ref().ok_or(E::Malformed)? {
                ExpressionBody::Input => {
                    self.facts.input_declarations = self
                        .facts
                        .input_declarations
                        .checked_add(1)
                        .ok_or(E::Limit)?;
                }
                ExpressionBody::Value(v) => {
                    let cost = neutral_ir::expression::inspect_expression_value_retention(
                        v,
                        self.retention_limits(),
                        self.budget.cancel,
                    )
                    .map_err(shape_error)?;
                    self.budget.step(cost.work)?;
                    self.budget.storage(cost.retained_bytes)?;
                    self.value(v, ty, d.public, &d.identity, false, 0)?;
                }
                ExpressionBody::Expression(e) => {
                    let cost = neutral_ir::expression::inspect_expression_retention(
                        e,
                        self.retention_limits(),
                        self.budget.cancel,
                    )
                    .map_err(shape_error)?;
                    self.budget.step(cost.work)?;
                    self.budget.storage(cost.retained_bytes)?;
                    self.tree(e, d.public, &d.identity, 1)?;
                    if &e.ty != ty {
                        return Err(E::TypeMismatch);
                    }
                }
            }
        }

        Ok(())
    }

    /// Every body shares the remaining request work and retention budget.
    pub(super) fn retention_limits(&self) -> ExpressionLimits {
        ExpressionLimits {
            work: self.budget.work,
            retained_bytes: self.budget.bytes,
            output_bytes: self.budget.bytes,
            ..self.limits.expression
        }
    }
}
