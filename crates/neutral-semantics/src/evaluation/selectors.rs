// SPDX-License-Identifier: Apache-2.0

//! Value-mode selector obligations against an independently typed complete output root.

use super::{Budget, EvaluationError as E, SelectorSegment, V};

/// Selects explicit fields/indices/alternatives without unwrapping null or following a ref.
pub(super) fn select<'v>(
    mut value: &'v V,
    path: &[SelectorSegment],
    budget: &mut Budget<'_>,
) -> Result<&'v V, E> {
    for segment in path {
        budget.step(1)?;
        value = match (segment, value) {
            (SelectorSegment::Field(name), V::Record(fields)) => {
                budget.step(
                    (name.len() as u64 + 1)
                        .checked_mul(fields.len() as u64 + 1)
                        .ok_or(E::Limit)?,
                )?;
                fields
                    .iter()
                    .find(|(field, _)| field == name)
                    .and_then(|(_, value)| value.as_ref())
                    .ok_or(E::Restriction)?
            }
            (SelectorSegment::Index(index), V::List(values)) => values
                .get(usize::try_from(*index).map_err(|_| E::Limit)?)
                .ok_or(E::Restriction)?,
            (SelectorSegment::Alternative(expected), V::Variant { tag, payload }) => {
                budget.step(expected.len() as u64 + 1)?;
                if expected != tag {
                    return Err(E::Restriction);
                }
                payload
            }
            _ => return Err(E::Restriction),
        };
    }
    Ok(value)
}

#[cfg(test)]
#[path = "../../tests/unit/evaluation_selectors.rs"]
mod tests;
