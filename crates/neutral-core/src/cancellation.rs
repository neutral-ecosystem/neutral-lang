// SPDX-License-Identifier: Apache-2.0

//! Cooperative request cancellation without rollback or thread interruption.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// A shareable cancellation signal for bounded compilation work.
///
/// Clones share one monotonic signal; there is deliberately no reset operation.
/// This is cooperative cancellation, not thread interruption or rollback. Each
/// phase must poll at its work boundaries and before authoritative publication.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Creates a cancellation signal that is initially clear.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation for every holder of this signal.
    ///
    /// Release/acquire ordering synchronizes observation of the request; callers
    /// must still explicitly poll and must not infer successful work from a token.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
