// SPDX-License-Identifier: Apache-2.0

//! Bounded big-endian base-ten scratch arithmetic; no floats, exponent expansion or ambient state.

use super::{Budget, E};
use std::cmp::Ordering;

/// Copies exact ASCII digits with optional finite alignment zeroes.
pub(super) fn digits(text: &str, zeroes: usize, budget: &mut Budget<'_>) -> Result<Vec<u8>, E> {
    if text == "0" {
        return budget.zeros(1);
    }
    let length = text.len().checked_add(zeroes).ok_or(E::Limit)?;
    let mut result = budget.zeros(length)?;
    budget.step(text.len() as u64)?;
    for (target, byte) in result.iter_mut().zip(text.bytes()) {
        *target = byte - b'0';
    }
    Ok(result)
}
/// Removes leading scratch zeroes while preserving a canonical zero slot.
pub(super) fn trim(value: &mut Vec<u8>, budget: &mut Budget<'_>) -> Result<(), E> {
    budget.step(value.len() as u64 + 1)?;
    let leading = value
        .iter()
        .position(|d| *d != 0)
        .unwrap_or(value.len().saturating_sub(1));
    value.drain(..leading);
    Ok(())
}
/// Tests normalized scratch zero without inspecting an unbounded tail.
pub(super) fn zero(value: &[u8]) -> bool {
    value == [0]
}
/// Compares normalized magnitudes after paying complete key work.
pub(super) fn compare(a: &[u8], b: &[u8], budget: &mut Budget<'_>) -> Result<Ordering, E> {
    budget.step(a.len().max(b.len()) as u64 + 1)?;
    Ok(a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
}
/// Adds magnitudes with one carry slot, then removes the unused slot.
pub(super) fn add(a: &[u8], b: &[u8], budget: &mut Budget<'_>) -> Result<Vec<u8>, E> {
    let length = a.len().max(b.len());
    let mut result = budget.zeros(length.checked_add(1).ok_or(E::Limit)?)?;
    budget.step(length as u64 + 1)?;
    let mut carry = 0;
    for offset in 0..length {
        let digit = a.len().checked_sub(offset + 1).map_or(0, |i| a[i])
            + b.len().checked_sub(offset + 1).map_or(0, |i| b[i])
            + carry;
        result[length - offset] = digit % 10;
        carry = digit / 10;
    }
    result[0] = carry;
    trim(&mut result, budget)?;
    Ok(result)
}
/// Subtracts a smaller magnitude in-place without allocating per division digit.
pub(super) fn subtract(a: &mut Vec<u8>, b: &[u8], budget: &mut Budget<'_>) -> Result<(), E> {
    if compare(a, b, budget)? == Ordering::Less {
        return Err(E::TypeMismatch);
    }
    budget.step(a.len() as u64 + 1)?;
    let mut borrow = 0_i16;
    for offset in 0..a.len() {
        let index = a.len() - offset - 1;
        let right = b.len().checked_sub(offset + 1).map_or(0, |i| b[i]);
        let digit = i16::from(a[index]) - i16::from(right) - borrow;
        borrow = i16::from(digit < 0);
        a[index] =
            u8::try_from(if digit < 0 { digit + 10 } else { digit }).map_err(|_| E::Limit)?;
    }
    trim(a, budget)
}
/// Copies a magnitude under the same fallible request storage policy.
pub(super) fn copy(value: &[u8], budget: &mut Budget<'_>) -> Result<Vec<u8>, E> {
    let mut result = budget.zeros(value.len())?;
    budget.step(value.len() as u64)?;
    result.copy_from_slice(value);
    Ok(result)
}
/// Multiplies finite coefficients, checking cancellation at every schoolbook row.
pub(super) fn multiply(a: &[u8], b: &[u8], budget: &mut Budget<'_>) -> Result<Vec<u8>, E> {
    let mut result = budget.zeros(a.len().checked_add(b.len()).ok_or(E::Limit)?)?;
    for i in (0..a.len()).rev() {
        budget.step(b.len() as u64 + 1)?;
        let mut carry = 0_u16;
        for j in (0..b.len()).rev() {
            let digit = u16::from(result[i + j + 1]) + u16::from(a[i]) * u16::from(b[j]) + carry;
            result[i + j + 1] = u8::try_from(digit % 10).map_err(|_| E::Limit)?;
            carry = digit / 10;
        }
        result[i] = u8::try_from(carry).map_err(|_| E::Limit)?;
    }
    trim(&mut result, budget)?;
    Ok(result)
}
/// Appends one radix digit to a normalized remainder with pre-reserved capacity.
pub(super) fn append(value: &mut Vec<u8>, digit: u8, budget: &mut Budget<'_>) -> Result<(), E> {
    if zero(value) {
        value[0] = digit;
        budget.step(1)
    } else {
        budget.push(value, digit)
    }
}
/// Extracts one quotient digit; the caller ensures the remainder is below ten denominators.
pub(super) fn quotient_digit(
    remainder: &mut Vec<u8>,
    denominator: &[u8],
    budget: &mut Budget<'_>,
) -> Result<u8, E> {
    let mut digit = 0;
    while compare(remainder, denominator, budget)? != Ordering::Less {
        subtract(remainder, denominator, budget)?;
        digit += 1;
        if digit > 9 {
            return Err(E::TypeMismatch);
        }
    }
    Ok(digit)
}
/// Divides normalized magnitudes with one reusable remainder buffer.
pub(super) fn div_rem(
    a: &[u8],
    b: &[u8],
    budget: &mut Budget<'_>,
) -> Result<(Vec<u8>, Vec<u8>), E> {
    if zero(b) {
        return Err(E::DivisionByZero);
    }
    let mut quotient = budget.zeros(a.len())?;
    let mut remainder = budget.zeros(1)?;
    budget.reserve(&mut remainder, b.len())?;
    for (index, digit) in a.iter().enumerate() {
        append(&mut remainder, *digit, budget)?;
        quotient[index] = quotient_digit(&mut remainder, b, budget)?;
    }
    trim(&mut quotient, budget)?;
    Ok((quotient, remainder))
}
/// Computes a small-factor remainder without mutation or allocation.
pub(super) fn small_remainder(value: &[u8], divisor: u8, budget: &mut Budget<'_>) -> Result<u8, E> {
    budget.step(value.len() as u64 + 1)?;
    let mut remainder = 0;
    for digit in value {
        remainder = (remainder * 10 + digit) % divisor;
    }
    Ok(remainder)
}
/// Removes one known exact factor of two or five in-place.
pub(super) fn divide_small(
    value: &mut Vec<u8>,
    divisor: u8,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(value.len() as u64 + 1)?;
    let mut remainder = 0;
    for digit in value.iter_mut() {
        let current = remainder * 10 + *digit;
        *digit = current / divisor;
        remainder = current % divisor;
    }
    trim(value, budget)
}
