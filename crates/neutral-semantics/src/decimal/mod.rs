// SPDX-License-Identifier: Apache-2.0

//! Shared exact decimal operations with frozen nonterminating-division rounding.

use crate::budget::{Budget, OperationError as E};
use neutral_ir::{ExactNumber, expression::BinaryOperator as O};
use std::cmp::Ordering;
mod coefficient;
mod division;

/// Dispatches arithmetic only; callers independently validate operand/result types.
pub(crate) fn binary(
    operator: O,
    left: &ExactNumber,
    right: &ExactNumber,
    budget: &mut Budget<'_>,
) -> Result<ExactNumber, E> {
    budget.number(left)?;
    budget.number(right)?;
    match operator {
        O::Add | O::Subtract => sum(left, right, operator == O::Subtract, budget),
        O::Multiply => {
            let a = coefficient::digits(left.coefficient(), 0, budget)?;
            let b = coefficient::digits(right.coefficient(), 0, budget)?;
            let value = coefficient::multiply(&a, &b, budget)?;
            finish(
                value,
                left.is_negative() != right.is_negative(),
                left.scale().checked_add(right.scale()).ok_or(E::Limit)?,
                budget,
            )
        }
        O::Divide => division::divide(left, right, budget),
        _ => Err(E::TypeMismatch),
    }
}
/// Negates without producing negative zero or changing coefficient identity.
pub(crate) fn negate(value: &ExactNumber, budget: &mut Budget<'_>) -> Result<ExactNumber, E> {
    budget.number(value)?;
    parts(
        !value.is_negative() && value.coefficient() != "0",
        value.coefficient(),
        value.scale(),
        budget,
    )
}
/// Reuses IR's allocation-free decimal comparison after bounding coefficient work.
pub(crate) fn compare(
    left: &ExactNumber,
    right: &ExactNumber,
    budget: &mut Budget<'_>,
) -> Result<Ordering, E> {
    budget.number(left)?;
    budget.number(right)?;
    budget.step(left.coefficient().len().max(right.coefficient().len()) as u64 + 1)?;
    Ok(neutral_ir::composition::compare_exact_numbers(left, right))
}
/// Aligns finite powers of ten and combines signed magnitudes without host rounding.
fn sum(
    left: &ExactNumber,
    right: &ExactNumber,
    subtract: bool,
    budget: &mut Budget<'_>,
) -> Result<ExactNumber, E> {
    let scale = left.scale().min(right.scale());
    let a = coefficient::digits(
        left.coefficient(),
        usize::try_from(left.scale() - scale).map_err(|_| E::Limit)?,
        budget,
    )?;
    let b = coefficient::digits(
        right.coefficient(),
        usize::try_from(right.scale() - scale).map_err(|_| E::Limit)?,
        budget,
    )?;
    let right_negative = right.is_negative() != subtract;
    let (value, negative) = if left.is_negative() == right_negative {
        (coefficient::add(&a, &b, budget)?, left.is_negative())
    } else if coefficient::compare(&a, &b, budget)? == Ordering::Less {
        let mut value = b;
        coefficient::subtract(&mut value, &a, budget)?;
        (value, right_negative)
    } else {
        let mut value = a;
        coefficient::subtract(&mut value, &b, budget)?;
        (value, left.is_negative())
    };
    finish(value, negative, scale, budget)
}
/// Normalizes owned scratch once, then reconstructs the unchanged public exact-number type.
fn finish(
    mut value: Vec<u8>,
    negative: bool,
    mut scale: i64,
    budget: &mut Budget<'_>,
) -> Result<ExactNumber, E> {
    coefficient::trim(&mut value, budget)?;
    if coefficient::zero(&value) {
        return parts(false, "0", 0, budget);
    }
    while value.last() == Some(&0) {
        budget.step(1)?;
        value.pop();
        scale = scale.checked_add(1).ok_or(E::Limit)?;
    }
    budget.step(value.len() as u64 + 1)?;
    for digit in &mut value {
        *digit += b'0';
    }
    let text = String::from_utf8(value).map_err(|_| E::TypeMismatch)?;
    parts(negative, &text, scale, budget)
}
/// Checks final normalized bounds/output bytes before a fallible owned coefficient copy.
fn parts(
    negative: bool,
    coefficient: &str,
    scale: i64,
    budget: &mut Budget<'_>,
) -> Result<ExactNumber, E> {
    if coefficient.len() as u64 > budget.limits.coefficient_digits
        || scale.unsigned_abs() > budget.limits.absolute_scale
        || (std::mem::size_of::<ExactNumber>() as u64)
            .checked_add(coefficient.len() as u64)
            .ok_or(E::Limit)?
            > budget.limits.output_bytes
    {
        return Err(E::Limit);
    }
    budget.storage(coefficient.len() as u64)?;
    let result = ExactNumber::from_normalized_parts(
        negative,
        coefficient,
        scale,
        budget.limits.coefficient_digits,
        budget.limits.absolute_scale,
    );
    budget.step(0)?;
    result.map_err(|_| E::Limit)
}
