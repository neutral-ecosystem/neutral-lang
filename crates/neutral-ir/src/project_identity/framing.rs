// SPDX-License-Identifier: Apache-2.0

//! In-place NHT frames; nested payloads never require proportional temporary copies.
//!
//! One writer owns an unpublished buffer and independent output/work budgets.
//! The caller chooses the domain and field order; this module enforces framing,
//! cancellation, and limits, not semantic validity. On any error the enclosing
//! transcript operation discards the writer, so incomplete bytes cannot escape.

use super::{IdentityError, IdentityLimits, MAX_TRANSCRIPT_BYTES, MAX_TRANSCRIPT_NODES};
use neutral_core::CancellationToken;
use neutral_core::allocation::RetainCapacity;

/// One bounded, cancellation-aware transcript under construction.
pub(super) struct Writer<'a> {
    /// Complete transcript, unpublished until every frame succeeds.
    bytes: Vec<u8>,
    /// Intersected independent byte/node bounds.
    limits: IdentityLimits,
    /// Number of retained frames.
    nodes: u64,
    /// Aggregate bytes of keys inspected before canonical-order comparison.
    key_bytes: u64,
    /// Explicit cooperative cancellation.
    cancellation: &'a CancellationToken,
}

impl<'a> Writer<'a> {
    /// Rejects zero bounds and cancellation before allocating proportional output.
    pub(super) fn new(
        limits: IdentityLimits,
        cancellation: &'a CancellationToken,
    ) -> Result<Self, IdentityError> {
        if limits.bytes == 0 || limits.nodes == 0 {
            return Err(IdentityError::Limit);
        }
        let result = Self {
            bytes: Vec::new(),
            limits: IdentityLimits {
                bytes: limits.bytes.min(MAX_TRANSCRIPT_BYTES),
                nodes: limits.nodes.min(MAX_TRANSCRIPT_NODES),
            },
            nodes: 0,
            key_bytes: 0,
            cancellation,
        };
        result.check()?;
        Ok(result)
    }
    /// Checks cancellation at every framing and writing boundary.
    pub(super) fn check(&self) -> Result<(), IdentityError> {
        if self.cancellation.is_cancelled() {
            Err(IdentityError::Cancelled)
        } else {
            Ok(())
        }
    }
    /// Bounds input collections before sorting or traversing their contents.
    pub(super) fn items(&self, count: usize) -> Result<(), IdentityError> {
        self.check()?;
        if count as u64 > self.limits.nodes {
            Err(IdentityError::Limit)
        } else {
            Ok(())
        }
    }
    /// Bounds text keys before canonical-order comparisons or sorting.
    ///
    /// This cumulative comparison budget is separate from output length: large
    /// keys must fail before an ordering check performs unbounded text work.
    pub(super) fn text(&mut self, text: &str) -> Result<(), IdentityError> {
        self.check()?;
        self.key_bytes = self
            .key_bytes
            .checked_add(text.len() as u64)
            .ok_or(IdentityError::Limit)?;
        if self.key_bytes > self.limits.bytes {
            Err(IdentityError::Limit)
        } else {
            Ok(())
        }
    }
    /// Appends bytes only after checked capacity and cancellation validation.
    fn append(&mut self, bytes: &[u8]) -> Result<(), IdentityError> {
        self.check()?;
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(IdentityError::Limit)?;
        if length as u64 > self.limits.bytes {
            return Err(IdentityError::Limit);
        }
        self.bytes
            .try_retain(bytes.len())
            .map_err(|_| IdentityError::Limit)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    /// Writes and backpatches one NHT frame without retaining an intermediate payload.
    ///
    /// Reserve the fixed-width payload-length slot, let nested frames append into
    /// the same buffer, then fill the slot from the completed byte range. A failed
    /// body leaves a partial buffer; callers must propagate the error and discard
    /// this writer, not retry or publish it. Node accounting includes containers
    /// as well as leaves, so deep framing cannot evade the work budget.
    pub(super) fn frame(
        &mut self,
        tag: &str,
        body: impl FnOnce(&mut Self) -> Result<(), IdentityError>,
    ) -> Result<(), IdentityError> {
        self.check()?;
        self.nodes = self.nodes.checked_add(1).ok_or(IdentityError::Limit)?;
        if self.nodes > self.limits.nodes {
            return Err(IdentityError::Limit);
        }
        if !tag.is_ascii() {
            return Err(IdentityError::InvalidInput);
        }
        let tag_length = u16::try_from(tag.len()).map_err(|_| IdentityError::Limit)?;
        self.append(&tag_length.to_be_bytes())?;
        self.append(tag.as_bytes())?;
        let length_offset = self.bytes.len();
        self.append(&0_u64.to_be_bytes())?;
        let start = self.bytes.len();
        body(self)?;
        // Offsets, rather than borrowed slices, survive reallocations in `body`.
        let length = u64::try_from(self.bytes.len() - start).map_err(|_| IdentityError::Limit)?;
        self.bytes[length_offset..length_offset + 8].copy_from_slice(&length.to_be_bytes());
        self.check()
    }
    /// Writes one explicitly tagged scalar byte sequence.
    pub(super) fn leaf(&mut self, tag: &str, bytes: &[u8]) -> Result<(), IdentityError> {
        self.frame(tag, |writer| writer.append(bytes))
    }
    /// Writes a fixed-width unsigned integer independent of host endianness.
    pub(super) fn number(&mut self, tag: &str, value: u64) -> Result<(), IdentityError> {
        self.leaf(tag, &value.to_be_bytes())
    }
    /// Returns only the complete successfully framed transcript.
    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// Rejects duplicate and noncanonical sequences in already typed logical content.
///
/// Do not sort here: accepting reordered producer data would conceal a malformed
/// canonical representation. Normalization is explicit only at contract-approved
/// boundaries such as artifact root selection.
pub(super) fn ordered<T: Ord>(values: impl IntoIterator<Item = T>) -> Result<(), IdentityError> {
    let mut previous = None;
    for value in values {
        if previous.as_ref().is_some_and(|previous| previous >= &value) {
            return Err(IdentityError::InvalidInput);
        }
        previous = Some(value);
    }
    Ok(())
}
