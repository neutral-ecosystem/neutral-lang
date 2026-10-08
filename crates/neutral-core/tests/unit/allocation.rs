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

/// Observation is thread-local, nested scopes restore their parent, and unwind cannot poison later requests.
#[cfg(feature = "allocation-testing")]
#[test]
fn allocation_observation_is_isolated_and_unwind_safe() {
    use testing::observe;
    let (result, count) = observe(Some(1), || {
        assert!(text("first").is_ok());
        let (nested, nested_count) = observe(Some(0), || text("nested"));
        assert!(nested.is_err());
        assert_eq!(nested_count, 1);
        assert!(
            std::thread::spawn(|| text("other thread"))
                .join()
                .unwrap()
                .is_ok()
        );
        text("second")
    });
    assert!(result.is_err());
    assert_eq!(count, 2);
    let panic = std::panic::catch_unwind(|| {
        observe(Some(0), || panic!("test observer unwind"));
    });
    assert!(panic.is_err());
    assert_eq!(text("recovered").unwrap(), "recovered");
}

/// Vector/string growth, boxes and shared headers each reject failure before ownership changes.
#[cfg(feature = "allocation-testing")]
#[test]
fn allocation_primitives_fail_before_mutation() {
    let mut bytes = vec![1_u8, 2];
    let mut string = "exact".to_owned();
    assert!(
        testing::observe(Some(0), || bytes.try_retain(100))
            .0
            .is_err()
    );
    assert!(
        testing::observe(Some(0), || string.try_retain_exact(100))
            .0
            .is_err()
    );
    assert_eq!(bytes, [1, 2]);
    assert_eq!(string, "exact");
    assert!(testing::observe(Some(0), || boxed(42)).0.is_err());
    assert!(testing::observe(Some(0), || Shared::try_new(42)).0.is_err());
    assert!(testing::observe(Some(0), || text("exact")).0.is_err());
}
