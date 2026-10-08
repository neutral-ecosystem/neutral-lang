// SPDX-License-Identifier: Apache-2.0

//! Fallible retained occurrence-key copies; raw-data bounds remain the caller's responsibility.

use super::CompositionError as E;
use neutral_core::allocation::RetainCapacity;
use neutral_ir::composition::ValuePathSegment as P;

/// Copies one previously bounded UTF-8 string without an infallible growth operation.
pub(super) fn text(value: &str) -> Result<String, E> {
    text_observed(value, &mut |_| Ok(()))
}
/// Copies a whole previously bounded path, including each owned field string.
pub(super) fn path(value: &[P]) -> Result<Vec<P>, E> {
    path_observed(value, &mut |_| Ok(()))
}
/// Observes an allocation boundary privately before performing the same fallible text copy.
fn text_observed(
    value: &str,
    observer: &mut impl FnMut(usize) -> Result<(), E>,
) -> Result<String, E> {
    observer(value.len())?;
    let mut result = String::new();
    result
        .try_retain_exact(value.len())
        .map_err(|_| E::Allocation)?;
    result.push_str(value);
    Ok(result)
}
/// Makes path publication atomic even when a later owned field string cannot be copied.
fn path_observed(
    value: &[P],
    observer: &mut impl FnMut(usize) -> Result<(), E>,
) -> Result<Vec<P>, E> {
    observer(value.len())?;
    let mut result = Vec::new();
    result
        .try_retain_exact(value.len())
        .map_err(|_| E::Allocation)?;
    for segment in value {
        result.push(match segment {
            P::Field(name) => P::Field(text_observed(name, observer)?),
            P::Element(index) => P::Element(*index),
            P::Payload => P::Payload,
        });
    }
    Ok(result)
}

#[cfg(test)]
#[path = "../../tests/composition/copy.rs"]
mod tests;
