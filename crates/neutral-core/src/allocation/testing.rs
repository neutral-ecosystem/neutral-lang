// SPDX-License-Identifier: Apache-2.0

//! Thread-local reservation failure scopes; never replace the process allocator.

use super::AllocationError;
use crate::CancellationToken;
use std::cell::{Cell, RefCell};

/// Scalar-only state: observations cannot allocate while an allocation is failing.
#[derive(Clone, Copy, Default)]
struct State {
    /// Whether a test currently observes this thread.
    active: bool,
    /// Zero-based reservation to reject; absence only counts reservations.
    fail_at: Option<usize>,
    /// Observed reservation count.
    count: usize,
}

thread_local! {
    /// Independent allocation policy for this thread, not inherited by child threads.
    static STATE: Cell<State> = const { Cell::new(State { active: false, fail_at: None, count: 0 }) };
    /// Optional cancellation signal for one reservation inside the current scope.
    static CANCEL: RefCell<Option<(usize, CancellationToken)>> = const { RefCell::new(None) };
}

/// Restores a possibly nested observation scope even when test code panics.
struct Restore(State);

/// Restores the outer cancellation policy after an observed operation or unwind.
struct RestoreCancellation(Option<(usize, CancellationToken)>);
impl Drop for RestoreCancellation {
    /// Restores the previous signal without allocating.
    fn drop(&mut self) {
        CANCEL.with(|signal| *signal.borrow_mut() = self.0.take());
    }
}

impl Drop for Restore {
    /// Restores the prior scalar policy without allocating or formatting.
    fn drop(&mut self) {
        STATE.with(|state| state.set(self.0));
    }
}

/// Counts or fails one reservation inside a thread-local, unwind-safe scope.
///
/// Setup and assertions should run outside the closure so only the operation
/// under review contributes to the count. The result carries no partial output
/// unless the operation's own public contract explicitly permits it.
pub fn observe<T>(fail_at: Option<usize>, operation: impl FnOnce() -> T) -> (T, usize) {
    let previous = STATE.with(|state| {
        let previous = state.get();
        state.set(State {
            active: true,
            fail_at,
            count: 0,
        });
        previous
    });
    let restore = Restore(previous);
    let result = operation();
    let count = STATE.with(|state| state.get().count);
    drop(restore);
    (result, count)
}

/// Cancels at a reservation boundary without returning a synthetic allocation error.
///
/// The operation must observe cancellation through its real token checks and
/// reject publication. The signal and policy remain confined to this thread.
pub fn observe_cancellation<T>(
    index: usize,
    signal: &CancellationToken,
    operation: impl FnOnce() -> T,
) -> (T, usize) {
    let previous = CANCEL.with(|slot| slot.replace(Some((index, signal.clone()))));
    let restore = RestoreCancellation(previous);
    let result = observe(None, operation);
    drop(restore);
    result
}

/// Rejects exactly one observed reservation before its storage operation begins.
pub(super) fn checkpoint() -> Result<(), AllocationError> {
    STATE.with(|state| {
        let mut current = state.get();
        if !current.active {
            return Ok(());
        }
        let failed = current.fail_at == Some(current.count);
        CANCEL.with(|signal| {
            if let Some((index, signal)) = signal.borrow().as_ref()
                && *index == current.count
            {
                signal.cancel();
            }
        });
        current.count += 1;
        state.set(current);
        if failed { Err(AllocationError) } else { Ok(()) }
    })
}
