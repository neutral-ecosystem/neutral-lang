// SPDX-License-Identifier: Apache-2.0

//! Selected static folding and capture-free retention of unresolved guarded branches.

use super::{EvaluationError as E, Expression, Flow, State, V};
use neutral_ir::expression::{BinaryOperator as O, ExpressionNode as N};

impl<'m> State<'m, '_> {
    /// Reuses pure evaluation before rewriting; no alternate arithmetic implementation.
    pub(super) fn fold(
        &mut self,
        expression: &'m Expression,
        depth: u64,
    ) -> Result<Expression, Flow<'m>> {
        if let Some(value) = self.walk(expression, depth)? {
            let owner = self.owner;
            self.checked(&value, &expression.ty, owner)?;
            return Ok(Expression {
                ty: self.budget.copy_type(&expression.ty)?,
                node: N::Value(value),
            });
        }
        match &expression.node {
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                if let Some(V::Bool(value)) = self.walk(guard, depth + 1)? {
                    return self.fold(if value { then_branch } else { else_branch }, depth + 1);
                }
            }
            N::Binary {
                operator: O::And | O::Or,
                left,
                right,
            } => {
                if let Some(V::Bool(_)) = self.walk(left, depth + 1)? {
                    return self.fold(right, depth + 1);
                }
            }
            N::Match { scrutinee, arms } => {
                if let Some(V::Variant { tag, payload }) = self.walk(scrutinee, depth + 1)? {
                    self.budget.step(
                        (tag.len() as u64 + 1)
                            .checked_mul(arms.len() as u64 + 1)
                            .ok_or(E::Limit)?,
                    )?;
                    let arm = arms
                        .iter()
                        .find(|arm| arm.tag == tag)
                        .ok_or(E::ExpressionType)?;
                    self.budget.reserve(&mut self.frames, 1)?;
                    self.frames.push(*payload);
                    let result = self.fold(&arm.body, depth + 1);
                    self.frames.pop();
                    return result;
                }
            }
            _ => {}
        }
        let mut result = self.retain_guarded(expression)?;
        self.fold_children(expression, &mut result, depth)?;
        self.budget.step(0)?;
        Ok(result)
    }

    /// Folds unconditional children only; guarded branches retain their original semantics.
    fn fold_children(
        &mut self,
        expression: &'m Expression,
        result: &mut Expression,
        depth: u64,
    ) -> Result<(), Flow<'m>> {
        match (&expression.node, &mut result.node) {
            (
                N::Unary { operand, .. },
                N::Unary {
                    operand: retained, ..
                },
            )
            | (N::Text(operand), N::Text(retained)) => {
                **retained = self.fold(operand, depth + 1)?;
            }
            (
                N::Binary {
                    operator,
                    left,
                    right,
                },
                N::Binary {
                    left: a, right: b, ..
                },
            ) => {
                **a = self.fold(left, depth + 1)?;
                if !matches!(operator, O::And | O::Or) {
                    **b = self.fold(right, depth + 1)?;
                }
            }
            (
                N::If { guard, .. },
                N::If {
                    guard: retained, ..
                },
            ) => **retained = self.fold(guard, depth + 1)?,
            (
                N::Match { scrutinee, .. },
                N::Match {
                    scrutinee: retained,
                    ..
                },
            ) => **retained = self.fold(scrutinee, depth + 1)?,
            (N::List(items), N::List(retained))
            | (N::Interpolate(items), N::Interpolate(retained)) => {
                for (item, retained) in items.iter().zip(retained) {
                    *retained = self.fold(item, depth + 1)?;
                }
            }
            (N::Record(fields), N::Record(retained)) => {
                for ((_, value), (_, retained)) in fields.iter().zip(retained) {
                    if let Some(value) = value {
                        *retained = Some(self.fold(value, depth + 1)?);
                    }
                }
            }
            (
                N::Variant { payload, .. },
                N::Variant {
                    payload: retained, ..
                },
            ) => **retained = self.fold(payload, depth + 1)?,
            _ => {}
        }
        self.budget.step(0)?;
        Ok(())
    }

    /// Copies guarded data without evaluating it, substituting only eliminated outer frames.
    fn retain_guarded(&mut self, expression: &Expression) -> Result<Expression, Flow<'m>> {
        let mut result = self.budget.copy_expression(expression)?;
        if !self.frames.is_empty() {
            self.substitute(&mut result, 0)?;
        }
        Ok(result)
    }

    /// Preserves retained match-local binders while closing references to folded-away matches.
    fn substitute(
        &mut self,
        expression: &mut Expression,
        local_frames: u64,
    ) -> Result<(), Flow<'m>> {
        self.budget.step(1)?;
        match &mut expression.node {
            N::Bound { depth, slot } if *depth >= local_frames => {
                if *slot != 0 {
                    return Err(E::ExpressionType.into());
                }
                let outer = usize::try_from(*depth - local_frames).map_err(|_| E::Limit)?;
                let index = self
                    .frames
                    .len()
                    .checked_sub(outer.checked_add(1).ok_or(E::Limit)?)
                    .ok_or(E::ExpressionType)?;
                expression.node = N::Value(self.budget.copy_value(&self.frames[index])?);
            }
            N::Unary { operand, .. } | N::Text(operand) => {
                self.substitute(operand, local_frames)?;
            }
            N::Binary { left, right, .. } => {
                self.substitute(left, local_frames)?;
                self.substitute(right, local_frames)?;
            }
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                self.substitute(guard, local_frames)?;
                self.substitute(then_branch, local_frames)?;
                self.substitute(else_branch, local_frames)?;
            }
            N::Match { scrutinee, arms } => {
                self.substitute(scrutinee, local_frames)?;
                for arm in arms {
                    self.substitute(&mut arm.body, local_frames + 1)?;
                }
            }
            N::List(items) | N::Interpolate(items) => {
                for item in items {
                    self.substitute(item, local_frames)?;
                }
            }
            N::Record(fields) => {
                for (_, value) in fields {
                    if let Some(value) = value {
                        self.substitute(value, local_frames)?;
                    }
                }
            }
            N::Variant { payload, .. } => self.substitute(payload, local_frames)?,
            _ => {}
        }
        Ok(())
    }
}
