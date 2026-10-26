// SPDX-License-Identifier: Apache-2.0

//! Literal retention checks cancellation between children, not only after a whole subtree.
//!
//! This private copier is reached only after the public bounded entry point has
//! inspected depth, numeric size and owned storage. It preserves omitted fields,
//! inert location text, reference owners and selected variant payloads exactly;
//! it does not materialize defaults or check nominal restrictions a second time.
//! Shared sequence/check helpers propagate failures and cancellation while dropping
//! incomplete containers, including cancellation injected at their last allocation.

use super::{E, check, sequence};
use crate::composition::BindingValue as V;
use neutral_core::{
    CancellationToken,
    allocation::{TryClone, boxed, text},
};

/// Copies a preflighted literal without continuing proportional retention after cancellation.
pub(super) fn copy(value: &V, cancel: &CancellationToken) -> Result<V, E> {
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    Ok(match value {
        V::Number(n) => V::Number(check(n.try_clone(), cancel)?),
        V::String(s) => V::String(check(text(s), cancel)?),
        V::Url(s) => V::Url(check(text(s), cancel)?),
        V::Path(s) => V::Path(check(text(s), cancel)?),
        V::Bool(b) => V::Bool(*b),
        V::Null => V::Null,
        V::Reference(s) => V::Reference(check(s.try_clone(), cancel)?),
        // References stay identity keys; only actual value children are copied recursively.
        V::List(values) => V::List(sequence(values, cancel, |v| copy(v, cancel))?),
        V::Record(fields) => V::Record(sequence(fields, cancel, |(name, value)| {
            Ok((
                check(text(name), cancel)?,
                value.as_ref().map(|v| copy(v, cancel)).transpose()?,
            ))
        })?),
        V::Variant { tag, payload } => V::Variant {
            tag: check(text(tag), cancel)?,
            payload: check(boxed(copy(payload, cancel)?), cancel)?,
        },
    })
}
