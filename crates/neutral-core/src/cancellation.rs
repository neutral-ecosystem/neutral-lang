// SPDX-License-Identifier: Apache-2.0

//! Cooperative request cancellation without rollback or thread interruption.
//!
//! A request owner creates one [`CancellationToken`] and shares clones with its
//! phases. Once any holder calls [`CancellationToken::cancel`], all clones can
//! observe the same monotonic flag. Separate requests need separate tokens;
//! reusing a cancelled token immediately cancels later work as well.
//!
//! The token does not inject checks, wake blocked I/O, undo mutations or reclaim
//! storage. Loops must poll at bounded intervals and result/cache owners must poll
//! before publication. A clear flag is only an observation at that instant, not
//! a guarantee against a concurrent cancellation immediately afterwards.

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
    /// Repeated requests are harmless; there is no acknowledgement or reset state.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Returns whether cancellation has been requested.
    ///
    /// This is a nonblocking atomic read. Observing `true` is permanent for this
    /// token; observing `false` does not reserve permission to publish later.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
