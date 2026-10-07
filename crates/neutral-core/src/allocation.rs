// SPDX-License-Identifier: Apache-2.0

//! Safe fallible ownership primitives. Semantic count, depth and cancellation checks belong to callers.

/// Allocation or capacity failure without formatting, heap-owned error text or partial output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationError;

/// Fallibly constructed immutable shared ownership for successor-only public contracts.
///
/// Unlike standard `Arc` construction, this type offers no infallible allocating
/// constructor or slice conversion. Cloning shares an existing allocation.
pub struct Shared<T>(triomphe::Arc<T>);

impl<T> Shared<T> {
    /// Creates shared ownership, returning allocation failure instead of aborting.
    ///
    /// # Errors
    /// Returns [`AllocationError`] if the shared header/data allocation fails.
    pub fn try_new(value: T) -> Result<Self, AllocationError> {
        triomphe::Arc::try_new(value)
            .map(Self)
            .map_err(|_| AllocationError)
    }
}
impl<T> Clone for Shared<T> {
    /// Shares immutable ownership without allocating another value or header.
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> std::ops::Deref for Shared<T> {
    type Target = T;
    /// Borrows the immutable shared value without copying.
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T> AsRef<T> for Shared<T> {
    /// Borrows the immutable shared value without copying.
    fn as_ref(&self) -> &T {
        self
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    /// Delegates to the value's existing safe diagnostic projection.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl<T: PartialEq> PartialEq for Shared<T> {
    /// Compares retained meaning rather than allocator identity.
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<T: Eq> Eq for Shared<T> {}

/// Copies previously bounded owned data using fallible allocation throughout the implementation.
///
/// Callers must preflight hostile recursive values before copying; this trait is
/// not a validator and does not replace depth, retention or cancellation policy.
pub trait TryClone: Sized {
    /// Returns an exact independent copy or an allocation failure, never partial ownership.
    ///
    /// # Errors
    /// Returns [`AllocationError`] when any constituent allocation cannot be reserved.
    fn try_clone(&self) -> Result<Self, AllocationError>;
}

/// Copies previously bounded UTF-8 bytes without an infallible string growth operation.
///
/// # Errors
/// Returns [`AllocationError`] when capacity or allocator limits prevent the copy.
pub fn text(value: &str) -> Result<String, AllocationError> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|_| AllocationError)?;
    result.push_str(value);
    Ok(result)
}

/// Moves a value into a fallible standard box without changing public data representations.
///
/// The reviewed dependency checks the global allocation result for null; errors
/// are discarded directly, never formatted or converted to allocating I/O errors.
///
/// # Errors
/// Returns [`AllocationError`] when the box allocation fails.
pub fn boxed<T>(value: T) -> Result<Box<T>, AllocationError> {
    trybox::or_drop(value).map_err(|_| AllocationError)
}

impl TryClone for String {
    /// Copies the exact UTF-8 bytes after one fallible reservation.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        text(self)
    }
}
impl TryClone for u64 {
    /// Copies a nonallocating scalar used by optional length restrictions.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        Ok(*self)
    }
}
impl<T: ?Sized> TryClone for &T {
    /// Copies a borrowed reference without allocating or copying the referent.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        Ok(*self)
    }
}
impl<T: TryClone> TryClone for Vec<T> {
    /// Reserves the outer collection and copies every owned child fallibly.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        copy_slice(self)
    }
}

/// Copies a preflighted borrowed slice with fallible outer and constituent retention.
///
/// # Errors
/// Returns [`AllocationError`] when any outer reservation or child copy fails.
pub fn copy_slice<T: TryClone>(values: &[T]) -> Result<Vec<T>, AllocationError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(values.len())
        .map_err(|_| AllocationError)?;
    for value in values {
        result.push(value.try_clone()?);
    }
    Ok(result)
}
impl<T: TryClone> TryClone for Box<T> {
    /// Copies the child and reserves its box, propagating either allocation failure.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        boxed((**self).try_clone()?)
    }
}
impl<T: TryClone> TryClone for Option<T> {
    /// Preserves absence without allocating and copies present values fallibly.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        self.as_ref().map(TryClone::try_clone).transpose()
    }
}
impl<A: TryClone, B: TryClone> TryClone for (A, B) {
    /// Copies both tuple fields without exposing an incomplete tuple.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        Ok((self.0.try_clone()?, self.1.try_clone()?))
    }
}
impl<A: TryClone, B: TryClone, C: TryClone> TryClone for (A, B, C) {
    /// Copies a complete nominal key fallibly, dropping any successful prefix on failure.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        Ok((
            self.0.try_clone()?,
            self.1.try_clone()?,
            self.2.try_clone()?,
        ))
    }
}
impl TryClone for std::convert::Infallible {
    /// Eliminates the uninhabited closed-reference representation.
    fn try_clone(&self) -> Result<Self, AllocationError> {
        match *self {}
    }
}

#[cfg(test)]
#[path = "../tests/unit/allocation.rs"]
mod tests;
