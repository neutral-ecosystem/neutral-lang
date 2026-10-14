// SPDX-License-Identifier: Apache-2.0

//! Selected-path pure traversal; missing guards never demand either guarded branch.

use super::{BindingKey, EvaluationError as E, Expression, Flow, State, V};
use neutral_ir::expression::{BinaryOperator as O, ExpressionNode as N};

impl<'m> State<'m, '_> {
    /// Evaluates one typed node or reports demand, never a partial accepted result.
    pub(super) fn walk(
        &mut self,
        expression: &'m Expression,
        depth: u64,
    ) -> Result<Option<V>, Flow<'m>> {
        self.budget.step(1)?;
        if depth > self.budget.limits.evaluation_depth {
            return Err(E::Limit.into());
        }
        match &expression.node {
            N::Value(value) => Ok(Some(self.budget.copy_value(value)?)),
            N::Input(owner) => self.demand(BindingKey::Input(owner)),
            N::Output {
                target,
                result_type,
                selector,
            } => self.demand(BindingKey::Output {
                target,
                result_type,
                selector,
            }),
            N::Unary { operator, operand } => match self.walk(operand, depth + 1)? {
                Some(value) => Ok(Some(crate::values::unary(
                    *operator,
                    &value,
                    &mut self.budget,
                )?)),
                None => Ok(None),
            },
            N::Binary {
                operator,
                left,
                right,
            } => self.binary(*operator, left, right, depth),
            N::If {
                guard,
                then_branch,
                else_branch,
            } => match self.walk(guard, depth + 1)? {
                Some(V::Bool(guard)) => {
                    self.walk(if guard { then_branch } else { else_branch }, depth + 1)
                }
                None => Ok(None),
                _ => Err(E::ExpressionType.into()),
            },
            N::Match { scrutinee, arms } => {
                let Some(value) = self.walk(scrutinee, depth + 1)? else {
                    return Ok(None);
                };
                let V::Variant { tag, payload } = value else {
                    return Err(E::ExpressionType.into());
                };
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
                let result = self.walk(&arm.body, depth + 1);
                self.frames.pop();
                result
            }
            N::Text(operand) => match self.walk(operand, depth + 1)? {
                Some(value) => Ok(Some(crate::values::text(&value, &mut self.budget)?)),
                None => Ok(None),
            },
            N::List(items) | N::Interpolate(items) => self.sequence(expression, items, depth),
            N::Record(fields) => self.record(expression, fields, depth),
            N::Variant { tag, payload } => {
                let Some(payload) = self.walk(payload, depth + 1)? else {
                    return Ok(None);
                };
                let value = V::Variant {
                    tag: self.budget.copy_text(tag)?,
                    payload: self.budget.boxed(payload)?,
                };
                let owner = self.owner;
                self.checked(&value, &expression.ty, owner)?;
                Ok(Some(value))
            }
            N::Bound { depth, slot } => {
                if *slot != 0 {
                    return Err(E::ExpressionType.into());
                }
                let depth = usize::try_from(*depth).map_err(|_| E::ExpressionType)?;
                let index = self
                    .frames
                    .len()
                    .checked_sub(depth.checked_add(1).ok_or(E::ExpressionType)?)
                    .ok_or(E::ExpressionType)?;
                Ok(Some(self.budget.copy_value(&self.frames[index])?))
            }
        }
    }
    /// Checks nominal field restrictions as soon as a complete computed record exists.
    fn record(
        &mut self,
        expression: &'m Expression,
        fields: &'m [(String, Option<Expression>)],
        depth: u64,
    ) -> Result<Option<V>, Flow<'m>> {
        let mut values = Vec::new();
        self.budget.reserve(&mut values, fields.len())?;
        let mut missing = false;
        for (name, expression) in fields {
            let value = match expression {
                Some(expression) => {
                    let value = self.walk(expression, depth + 1)?;
                    missing |= value.is_none();
                    value
                }
                None => None,
            };
            values.push((self.budget.copy_text(name)?, value));
        }
        if missing {
            return Ok(None);
        }
        let value = V::Record(values);
        let owner = self.owner;
        self.checked(&value, &expression.ty, owner)?;
        Ok(Some(value))
    }
    /// Ordinary operands are independent, but Boolean guards select the right operand.
    fn binary(
        &mut self,
        operator: O,
        left: &'m Expression,
        right: &'m Expression,
        depth: u64,
    ) -> Result<Option<V>, Flow<'m>> {
        let left = self.walk(left, depth + 1)?;
        if matches!(operator, O::And | O::Or) {
            let Some(V::Bool(value)) = &left else {
                return if left.is_none() {
                    Ok(None)
                } else {
                    Err(E::ExpressionType.into())
                };
            };
            if (operator == O::And && !value) || (operator == O::Or && *value) {
                return Ok(left);
            }
        }
        let right = self.walk(right, depth + 1)?;
        match (left, right) {
            (Some(left), Some(right)) => Ok(Some(crate::values::binary(
                operator,
                &left,
                &right,
                &mut self.budget,
            )?)),
            _ => Ok(None),
        }
    }
    /// Collects operand values before construction; missing is never a synthesized list element.
    fn sequence(
        &mut self,
        expression: &'m Expression,
        items: &'m [Expression],
        depth: u64,
    ) -> Result<Option<V>, Flow<'m>> {
        let mut values = Vec::new();
        self.budget.reserve(&mut values, items.len())?;
        let mut missing = false;
        for item in items {
            match self.walk(item, depth + 1)? {
                Some(value) => values.push(value),
                None => missing = true,
            }
        }
        if missing {
            return Ok(None);
        }
        if matches!(expression.node, N::List(_)) {
            return Ok(Some(V::List(values)));
        }
        let mut length = 0_usize;
        for value in &values {
            let V::String(value) = value else {
                return Err(E::ExpressionType.into());
            };
            length = length.checked_add(value.len()).ok_or(E::Limit)?;
        }
        self.budget.output(
            (std::mem::size_of::<V>() as u64)
                .checked_add(length as u64)
                .ok_or(E::Limit)?,
        )?;
        let mut bytes = Vec::new();
        self.budget.reserve(&mut bytes, length)?;
        for value in values {
            let V::String(value) = value else {
                return Err(E::ExpressionType.into());
            };
            bytes.extend_from_slice(value.as_bytes());
        }
        Ok(Some(V::String(
            String::from_utf8(bytes).map_err(|_| E::ExpressionType)?,
        )))
    }
}
