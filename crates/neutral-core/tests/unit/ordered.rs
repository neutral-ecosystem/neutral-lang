// SPDX-License-Identifier: Apache-2.0

//! Canonical fallible index membership, replacement and removal regressions.

use super::*;

/// Ordered insertion preserves exact uniqueness and borrowed lookup semantics.
#[test]
fn ordered_map_preserves_canonical_membership_and_entry_updates() {
    let mut index = OrderedMap::new();
    assert_eq!(index.insert("z".to_owned(), 3).unwrap(), None);
    assert_eq!(index.insert("a".to_owned(), 1).unwrap(), None);
    assert_eq!(index.insert("z".to_owned(), 4).unwrap(), Some(3));
    *index.entry("b".to_owned()).unwrap().or_insert(0) += 2;
    assert_eq!(index.get("z"), Some(&4));
    assert_eq!(index["z"], 4);
    assert_eq!(
        index
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect::<Vec<_>>(),
        [("a", 1), ("b", 2), ("z", 4)]
    );
    assert_eq!(index.remove("b"), Some(2));
    assert_eq!(index.remove("missing"), None);
    assert_eq!(index.into_values().collect::<Vec<_>>(), [1, 4]);
}

/// Set insertion rejects duplicates without allocating additional membership.
#[test]
fn ordered_set_preserves_unique_sorted_keys() {
    let mut set = OrderedSet::new();
    assert!(set.insert(2).unwrap());
    assert!(set.insert(1).unwrap());
    assert!(!set.insert(2).unwrap());
    assert!(set.contains(&1));
    assert!(set.remove(&1));
    assert!(!set.remove(&1));
    assert_eq!(set.into_iter().collect::<Vec<_>>(), [2]);
}
