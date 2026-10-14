// SPDX-License-Identifier: Apache-2.0

//! Typed pure value operations; caller typing precedes dispatch and no backend formatting is implied.

use crate::{
    budget::{Budget, OperationError as E},
    decimal,
};
use neutral_ir::{
    composition::BindingValue as V,
    expression::{BinaryOperator as O, UnaryOperator as U},
    language,
};
use std::cmp::Ordering;

/// Checks complete literal storage before deep comparisons, including strings and nested values.
fn preflight(value: &V, budget: &mut Budget<'_>) -> Result<(), E> {
    let cost = neutral_ir::expression::inspect_expression_value_retention(
        value,
        budget.input_limits(),
        budget.cancellation(),
    )
    .map_err(crate::budget::shape_error)?;
    budget.step(cost.work)
}
/// Applies a typed prefix operation without coercion.
pub(crate) fn unary(operator: U, value: &V, budget: &mut Budget<'_>) -> Result<V, E> {
    match (operator, value) {
        (U::Not, V::Bool(value)) => {
            budget.step(1)?;
            Ok(V::Bool(!value))
        }
        (U::Negate, V::Number(number)) => Ok(V::Number(decimal::negate(number, budget)?)),
        _ => Err(E::TypeMismatch),
    }
}
/// Applies ordinary operations; Boolean short-circuit selection belongs to the evaluator walker.
pub(crate) fn binary(operator: O, left: &V, right: &V, budget: &mut Budget<'_>) -> Result<V, E> {
    preflight(left, budget)?;
    preflight(right, budget)?;
    match operator {
        O::Add | O::Subtract | O::Multiply | O::Divide => {
            let (V::Number(left), V::Number(right)) = (left, right) else {
                return Err(E::TypeMismatch);
            };
            Ok(V::Number(decimal::binary(operator, left, right, budget)?))
        }
        O::Concat => concatenate(left, right, budget),
        O::And | O::Or => {
            let (V::Bool(left), V::Bool(right)) = (left, right) else {
                return Err(E::TypeMismatch);
            };
            Ok(V::Bool(if operator == O::And {
                *left && *right
            } else {
                *left || *right
            }))
        }
        O::Equal | O::NotEqual => Ok(V::Bool((left == right) == (operator == O::Equal))),
        O::Less | O::LessEqual | O::Greater | O::GreaterEqual => {
            let order = match (left, right) {
                (V::Number(left), V::Number(right)) => decimal::compare(left, right, budget)?,
                (V::String(left), V::String(right)) => left.cmp(right),
                _ => return Err(E::TypeMismatch),
            };
            Ok(V::Bool(match operator {
                O::Less => order == Ordering::Less,
                O::LessEqual => order != Ordering::Greater,
                O::Greater => order == Ordering::Greater,
                _ => order != Ordering::Less,
            }))
        }
    }
}
/// Concatenates homogeneous already-typed values using fallible complete output storage.
fn concatenate(left: &V, right: &V, budget: &mut Budget<'_>) -> Result<V, E> {
    match (left, right) {
        (V::String(left), V::String(right)) => {
            let length = left.len().checked_add(right.len()).ok_or(E::Limit)?;
            budget.output(
                (std::mem::size_of::<V>() as u64)
                    .checked_add(length as u64)
                    .ok_or(E::Limit)?,
            )?;
            let mut bytes = Vec::new();
            budget.reserve(&mut bytes, length)?;
            bytes.extend_from_slice(left.as_bytes());
            bytes.extend_from_slice(right.as_bytes());
            Ok(V::String(
                String::from_utf8(bytes).map_err(|_| E::TypeMismatch)?,
            ))
        }
        (V::List(left), V::List(right)) => {
            let length = left.len().checked_add(right.len()).ok_or(E::Limit)?;
            let bytes = length
                .checked_add(1)
                .and_then(|n| n.checked_mul(std::mem::size_of::<V>()))
                .ok_or(E::Limit)?;
            budget.output(bytes as u64)?;
            let mut values = Vec::new();
            budget.reserve(&mut values, length)?;
            for item in left.iter().chain(right) {
                values.push(budget.copy_value(item)?);
            }
            Ok(V::List(values))
        }
        _ => Err(E::TypeMismatch),
    }
}
/// Converts only scalar text/Boolean/number values to canonical finite decimal text.
pub(crate) fn text(value: &V, budget: &mut Budget<'_>) -> Result<V, E> {
    match value {
        V::String(_) => budget.copy_value(value),
        V::Bool(value) => {
            let text = if *value {
                language::TRUE
            } else {
                language::FALSE
            };
            let mut bytes = Vec::new();
            budget.output((std::mem::size_of::<V>() + text.len()) as u64)?;
            budget.reserve(&mut bytes, text.len())?;
            bytes.extend_from_slice(text.as_bytes());
            Ok(V::String(
                String::from_utf8(bytes).map_err(|_| E::TypeMismatch)?,
            ))
        }
        V::Number(value) => {
            budget.number(value)?;
            let digits = value.coefficient();
            let position = i64::try_from(digits.len())
                .map_err(|_| E::Limit)?
                .checked_add(value.scale())
                .ok_or(E::Limit)?;
            let magnitude = if value.scale() >= 0 {
                position
            } else if position > 0 {
                i64::try_from(digits.len())
                    .map_err(|_| E::Limit)?
                    .checked_add(1)
                    .ok_or(E::Limit)?
            } else {
                2_i64
                    .checked_sub(position)
                    .and_then(|n| n.checked_add(i64::try_from(digits.len()).ok()?))
                    .ok_or(E::Limit)?
            };
            let length = usize::try_from(magnitude)
                .map_err(|_| E::Limit)?
                .checked_add(usize::from(value.is_negative()))
                .ok_or(E::Limit)?;
            budget.output(
                (std::mem::size_of::<V>() as u64)
                    .checked_add(length as u64)
                    .ok_or(E::Limit)?,
            )?;
            let mut bytes = budget.zeros(length)?;
            bytes.fill(b'0');
            let start = usize::from(value.is_negative());
            if value.is_negative() {
                bytes[0] = b'-';
            }
            if value.scale() >= 0 {
                bytes[start..start + digits.len()].copy_from_slice(digits.as_bytes());
            } else if position > 0 {
                let position = usize::try_from(position).map_err(|_| E::Limit)?;
                bytes[start..start + position].copy_from_slice(&digits.as_bytes()[..position]);
                bytes[start + position] = b'.';
                bytes[start + position + 1..].copy_from_slice(&digits.as_bytes()[position..]);
            } else {
                bytes[start + 1] = b'.';
                let offset = length - digits.len();
                bytes[offset..].copy_from_slice(digits.as_bytes());
            }
            Ok(V::String(
                String::from_utf8(bytes).map_err(|_| E::TypeMismatch)?,
            ))
        }
        _ => Err(E::TypeMismatch),
    }
}
