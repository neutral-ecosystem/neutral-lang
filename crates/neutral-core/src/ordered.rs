// SPDX-License-Identifier: Apache-2.0

//! Fallibly growing canonical indexes. Callers charge insertion shifts and key comparisons to work policy.

use crate::allocation::{AllocationError as E, TryClone};
use std::borrow::Borrow;

/// Canonically ordered request-local index with no infallible allocating insertion.
///
/// Insertion shifts up to `len()` entries; callers must charge that work before
/// inserting attacker-controlled data. Read-only lookup uses binary search.
#[derive(Debug, Eq, PartialEq)]
pub struct OrderedMap<K, V> {
    /// Sorted unique keys and their owned or borrowed values.
    entries: Vec<(K, V)>,
}
impl<K, V> Default for OrderedMap<K, V> {
    /// Starts empty without allocating storage.
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}
impl<K: Ord, V> OrderedMap<K, V> {
    /// Starts empty without allocating storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns the exact unique entry count, also an upper bound on one insertion shift.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Returns whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Finds a borrowed key without allocating.
    fn position<Q: Ord + ?Sized>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
    {
        self.entries
            .binary_search_by(|(existing, _)| existing.borrow().cmp(key))
    }
    /// Returns immutable value access for one exact borrowed key.
    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.position(key).ok().map(|index| &self.entries[index].1)
    }
    /// Returns mutable request-local value access without growing storage.
    pub fn get_mut<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        self.position(key)
            .ok()
            .map(|index| &mut self.entries[index].1)
    }
    /// Checks exact membership without allocating.
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.position(key).is_ok()
    }
    /// Inserts with fallible growth, returning replaced data if the key already existed.
    ///
    /// # Errors
    /// Returns allocation failure without changing the index when growth cannot be reserved.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, E> {
        match self.position(&key) {
            Ok(index) => Ok(Some(std::mem::replace(&mut self.entries[index].1, value))),
            Err(index) => {
                self.entries.try_reserve(1).map_err(|_| E)?;
                self.entries.insert(index, (key, value));
                Ok(None)
            }
        }
    }
    /// Reserves a vacant entry before any nonallocating default is installed.
    ///
    /// # Errors
    /// Returns allocation failure before changing entry membership.
    pub fn entry(&mut self, key: K) -> Result<Entry<'_, K, V>, E> {
        let position = self.position(&key);
        if position.is_err() {
            self.entries.try_reserve(1).map_err(|_| E)?;
        }
        Ok(Entry {
            map: self,
            key,
            position,
        })
    }
    /// Removes one exact key without allocating.
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.position(key)
            .ok()
            .map(|index| self.entries.remove(index).1)
    }
    /// Enumerates keys and values in canonical order without copying them.
    #[must_use]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
    /// Enumerates canonical borrowed keys without allocating.
    #[must_use]
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }
    /// Enumerates canonical borrowed values without allocating.
    #[must_use]
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }
    /// Moves canonical values out without allocating or cloning them.
    pub fn into_values(self) -> impl Iterator<Item = V> {
        self.entries.into_iter().map(|(_, v)| v)
    }
}
impl<K: TryClone, V: TryClone> TryClone for OrderedMap<K, V> {
    /// Copies retained canonical entries fallibly without repeating insertion work.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            entries: self.entries.try_clone()?,
        })
    }
}
impl<K: Ord + Borrow<Q>, Q: Ord + ?Sized, V> std::ops::Index<&Q> for OrderedMap<K, V> {
    type Output = V;

    /// Accesses an already established key; untrusted membership must use `get` first.
    ///
    /// # Panics
    /// Panics if the caller has not established that the key is present.
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("established ordered index key")
    }
}
impl<K, V> IntoIterator for OrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    /// Moves entries in canonical order without allocating.
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}
impl<'a, K, V> IntoIterator for &'a OrderedMap<K, V> {
    type Item = &'a (K, V);
    type IntoIter = std::slice::Iter<'a, (K, V)>;
    /// Borrows entries in canonical order without allocating.
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
/// A reserved occupied/vacant slot; no allocation occurs when its value is installed.
pub struct Entry<'a, K, V> {
    /// Exclusively borrowed request-local index.
    map: &'a mut OrderedMap<K, V>,
    /// Owned insertion key, discarded when already present.
    key: K,
    /// Canonical occupied index or pre-reserved insertion position.
    position: Result<usize, usize>,
}
impl<'a, K, V> Entry<'a, K, V> {
    /// Installs the supplied nonallocating value after successful reservation.
    pub fn or_insert(self, value: V) -> &'a mut V {
        let index = match self.position {
            Ok(index) => index,
            Err(index) => {
                self.map.entries.insert(index, (self.key, value));
                index
            }
        };
        &mut self.map.entries[index].1
    }
}
impl<'a, K, V: Default> Entry<'a, K, V> {
    /// Installs an empty default; callers must use a nonallocating `Default` implementation.
    pub fn or_default(self) -> &'a mut V {
        self.or_insert(V::default())
    }
}

/// Canonically ordered unique membership using the same fallible storage as an index.
#[derive(Debug, Eq, PartialEq)]
pub struct OrderedSet<K>(OrderedMap<K, ()>);
impl<K> Default for OrderedSet<K> {
    /// Starts empty without allocating.
    fn default() -> Self {
        Self(OrderedMap::default())
    }
}
impl<K: Ord> OrderedSet<K> {
    /// Starts empty without allocating.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns the unique key count and insertion-shift upper bound.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Checks whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Inserts membership after a fallible growth reservation.
    ///
    /// # Errors
    /// Returns allocation failure without adding a key.
    pub fn insert(&mut self, key: K) -> Result<bool, E> {
        self.0.insert(key, ()).map(|old| old.is_none())
    }
    /// Checks borrowed exact membership without allocating.
    pub fn contains<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.0.contains_key(key)
    }
    /// Removes exact membership without allocating.
    pub fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.0.remove(key).is_some()
    }
    /// Borrows all keys in canonical order without allocating.
    #[must_use]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &K> {
        self.0.keys()
    }
}
impl<K> IntoIterator for OrderedSet<K> {
    type Item = K;
    type IntoIter = std::iter::Map<std::vec::IntoIter<(K, ())>, fn((K, ())) -> K>;
    /// Moves keys out in canonical order without allocating.
    fn into_iter(self) -> Self::IntoIter {
        self.0.entries.into_iter().map(|(k, ())| k)
    }
}
impl<'a, K> IntoIterator for &'a OrderedSet<K> {
    type Item = &'a K;
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, ())>, fn(&'a (K, ())) -> &'a K>;
    /// Borrows keys in canonical order without allocating.
    fn into_iter(self) -> Self::IntoIter {
        self.0.entries.iter().map(|(k, ())| k)
    }
}

#[cfg(test)]
#[path = "../tests/unit/ordered.rs"]
mod tests;
