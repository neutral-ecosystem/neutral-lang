// SPDX-License-Identifier: Apache-2.0

//! Exact terminating division and nearest/ties-even nonterminating decimal rounding.

use super::{Budget, E, coefficient as c, finish};
use neutral_ir::{ExactNumber, expression::profile::DECIMAL_SIGNIFICANT_DIGITS};
use std::cmp::Ordering;

#[cfg(test)]
#[path = "../../tests/unit/decimal_rounding.rs"]
mod tests;

/// Detects termination after removing decimal prime factors, without a large rational GCD.
fn terminating(numerator: &[u8], denominator: &[u8], budget: &mut Budget<'_>) -> Result<bool, E> {
    let mut odd = c::copy(denominator, budget)?;
    for factor in [2, 5] {
        while c::small_remainder(&odd, factor, budget)? == 0 {
            c::divide_small(&mut odd, factor, budget)?;
        }
    }
    if odd == [1] {
        return Ok(true);
    }
    Ok(c::zero(&c::div_rem(numerator, &odd, budget)?.1))
}
/// Uses quotient/remainder directly; decimal scales never become floating-point operands.
pub(super) fn divide(
    left: &ExactNumber,
    right: &ExactNumber,
    budget: &mut Budget<'_>,
) -> Result<ExactNumber, E> {
    if right.coefficient() == "0" {
        return Err(E::DivisionByZero);
    }
    if left.coefficient() == "0" {
        return super::parts(false, "0", 0, budget);
    }
    let numerator = c::digits(left.coefficient(), 0, budget)?;
    let denominator = c::digits(right.coefficient(), 0, budget)?;
    let exact = terminating(&numerator, &denominator, budget)?;
    let (mut quotient, mut remainder) = c::div_rem(&numerator, &denominator, budget)?;
    let scale = left.scale().checked_sub(right.scale()).ok_or(E::Limit)?;
    let negative = left.is_negative() != right.is_negative();
    if exact {
        let mut fractional = 0_i64;
        while !c::zero(&remainder) {
            c::append(&mut remainder, 0, budget)?;
            let digit = c::quotient_digit(&mut remainder, &denominator, budget)?;
            budget.push(&mut quotient, digit)?;
            fractional = fractional.checked_add(1).ok_or(E::Limit)?;
        }
        return finish(
            quotient,
            negative,
            scale.checked_sub(fractional).ok_or(E::Limit)?,
            budget,
        );
    }
    rounded(quotient, remainder, &denominator, negative, scale, budget)
}
/// Produces the fixed number of significant digits without retaining exponent-sized zero prefixes.
fn rounded(
    mut quotient: Vec<u8>,
    mut remainder: Vec<u8>,
    denominator: &[u8],
    negative: bool,
    scale: i64,
    budget: &mut Budget<'_>,
) -> Result<ExactNumber, E> {
    let precision = DECIMAL_SIGNIFICANT_DIGITS;
    if !c::zero(&quotient) && quotient.len() > precision {
        budget.step(quotient.len() as u64 + 1)?;
        let discarded = quotient.len() - precision;
        let first = quotient[precision];
        let nonzero_tail =
            quotient[precision + 1..].iter().any(|d| *d != 0) || !c::zero(&remainder);
        let up = first > 5
            || (first == 5 && (nonzero_tail || !quotient[precision - 1].is_multiple_of(2)));
        quotient.truncate(precision);
        if up {
            increment(&mut quotient, budget)?;
        }
        return finish(
            quotient,
            negative,
            scale
                .checked_add(i64::try_from(discarded).map_err(|_| E::Limit)?)
                .ok_or(E::Limit)?,
            budget,
        );
    }
    if c::zero(&quotient) {
        quotient.clear();
    }
    let mut fractional = 0_i64;
    while quotient.len() < precision {
        c::append(&mut remainder, 0, budget)?;
        let digit = c::quotient_digit(&mut remainder, denominator, budget)?;
        fractional = fractional.checked_add(1).ok_or(E::Limit)?;
        if digit != 0 || !quotient.is_empty() {
            budget.push(&mut quotient, digit)?;
        }
    }
    if round_up(&remainder, denominator, quotient[precision - 1], budget)? {
        increment(&mut quotient, budget)?;
    }
    finish(
        quotient,
        negative,
        scale.checked_sub(fractional).ok_or(E::Limit)?,
        budget,
    )
}
/// Compares twice the exact remainder; an exact midpoint increments only an odd retained digit.
pub(super) fn round_up(
    remainder: &[u8],
    denominator: &[u8],
    last: u8,
    budget: &mut Budget<'_>,
) -> Result<bool, E> {
    let twice = c::add(remainder, remainder, budget)?;
    Ok(match c::compare(&twice, denominator, budget)? {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => !last.is_multiple_of(2),
    })
}
/// Rounds with a bounded carry slot; normalization later moves trailing zeroes into scale.
fn increment(value: &mut Vec<u8>, budget: &mut Budget<'_>) -> Result<(), E> {
    budget.step(value.len() as u64 + 1)?;
    for digit in value.iter_mut().rev() {
        if *digit != 9 {
            *digit += 1;
            return Ok(());
        }
        *digit = 0;
    }
    budget.push(value, 0)?;
    value[0] = 1;
    Ok(())
}
