// SPDX-License-Identifier: Apache-2.0

//! Fallible ownership primitive regressions, without replacing the global allocator.

use super::*;

/// Nested boxed collections preserve exact bytes and independently owned data.
#[test]
fn fallible_copies_preserve_boxed_collection_ownership() {
    let original = vec![("é🙂".to_owned(), Some(Box::new("payload".to_owned())))];
    let mut copy = original.try_clone().unwrap();
    copy[0].0.push('!');
    assert_eq!(original[0].0, "é🙂");
    assert_eq!(copy[0].1, original[0].1);
    assert_eq!(text("").unwrap(), "");
    assert_eq!(*boxed(()).unwrap(), ());
}

/// A constituent failure short-circuits the same production collection-copy loop.
#[test]
fn fallible_collection_copy_propagates_child_failure() {
    /// A deterministic child failure, not global allocator exhaustion.
    struct Failure;
    impl TryClone for Failure {
        /// Fails without creating a copied child.
        fn try_clone(&self) -> Result<Self, AllocationError> {
            Err(AllocationError)
        }
    }
    assert!(vec![Failure].try_clone().is_err());
    assert!(Some(Failure).try_clone().is_err());
    assert!(Box::new(Failure).try_clone().is_err());
}

/// Sharing retains one allocation across threads and drops the payload exactly once.
#[test]
fn fallible_shared_owner_preserves_thread_safe_single_drop() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    /// Records payload destruction independently of the shared-owner backend.
    #[derive(Debug)]
    struct Payload(Arc<AtomicUsize>);
    impl Drop for Payload {
        /// Counts the final owner release.
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Shared::try_new(Payload(Arc::clone(&drops))).unwrap();
    let copy = owner.clone();
    assert!(std::ptr::eq(owner.as_ref(), copy.as_ref()));
    std::thread::spawn(move || drop(copy)).join().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    let text = Shared::try_new("exact bytes".to_owned()).unwrap();
    assert_eq!(text, text.clone());
}
