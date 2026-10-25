// SPDX-License-Identifier: Apache-2.0

//! Independent bounded vocabulary decoding and validation controls.

use crate::VocabularyError;
use neutral_core::StructuralLimits;

/// Stack-safe ceiling for recursively decoded vocabulary JSON and defaults.
pub const MAX_VOCABULARY_NESTING_DEPTH: u64 = 64;

/// Explicit resource limits for strict vocabulary decoding and validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VocabularyLimits {
    /// Maximum exact captured bundle bytes.
    pub(crate) bundle_bytes: u64,
    /// Maximum decoded bytes in one JSON string.
    pub(crate) string_bytes: u64,
    /// Maximum members in one JSON object.
    pub(crate) object_members: u64,
    /// Maximum items in one JSON array.
    pub(crate) array_items: u64,
    /// Maximum nested JSON/type/default depth.
    pub(crate) nesting_depth: u64,
    /// Maximum total JSON and semantic nodes.
    pub(crate) total_nodes: u64,
    /// Maximum nominal vocabulary types.
    pub(crate) types: u64,
    /// Maximum fields in one type or record default.
    pub(crate) fields: u64,
    /// Maximum required structural features.
    pub(crate) features: u64,
    /// Maximum exact-number significant digits.
    pub(crate) numeric_digits: u64,
    /// Maximum exact-number absolute scale.
    pub(crate) numeric_scale: u64,
}

impl VocabularyLimits {
    /// Returns the independent exact coefficient digit budget, not a semantic numeric restriction.
    #[must_use]
    pub const fn numeric_digits(self) -> u64 {
        self.numeric_digits
    }
    /// Returns the independent absolute exponent budget; no exponent-sized padding is allocated.
    #[must_use]
    pub const fn numeric_scale(self) -> u64 {
        self.numeric_scale
    }
    /// Creates vocabulary limits from the shared structural policy.
    ///
    /// The captured byte bound also bounds JSON members, arrays, types, and
    /// features until a host supplies narrower values through the builder API.
    /// Recursive decoding depth is additionally capped at the stack-safe
    /// [`MAX_VOCABULARY_NESTING_DEPTH`] even when the host permits more.
    #[must_use]
    pub const fn from_structural(limits: StructuralLimits) -> Self {
        let nesting_depth = if limits.nesting_depth() < MAX_VOCABULARY_NESTING_DEPTH {
            limits.nesting_depth()
        } else {
            MAX_VOCABULARY_NESTING_DEPTH
        };
        Self {
            bundle_bytes: limits.source_bytes(),
            string_bytes: limits.string_bytes(),
            object_members: limits.record_fields(),
            array_items: limits.list_items(),
            nesting_depth,
            total_nodes: limits.traversal_nodes(),
            types: limits.declarations(),
            fields: limits.record_fields(),
            features: limits.list_items(),
            numeric_digits: limits.numeric_digits(),
            numeric_scale: limits.numeric_scale(),
        }
    }

    /// Overrides the exact captured bundle-byte limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_bundle_bytes(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.bundle_bytes = value;
        Ok(self)
    }

    /// Overrides the per-object member limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_object_members(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.object_members = value;
        Ok(self)
    }

    /// Overrides the per-array item limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_array_items(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.array_items = value;
        Ok(self)
    }

    /// Overrides the nominal-type limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_types(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.types = value;
        Ok(self)
    }

    /// Overrides the structural-feature limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_features(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.features = value;
        Ok(self)
    }

    /// Returns the exact captured bundle-byte limit.
    #[must_use]
    pub const fn bundle_bytes(self) -> u64 {
        self.bundle_bytes
    }

    /// Returns the maximum decoded JSON string bytes.
    #[must_use]
    pub const fn string_bytes(self) -> u64 {
        self.string_bytes
    }

    /// Returns the maximum object members.
    #[must_use]
    pub const fn object_members(self) -> u64 {
        self.object_members
    }

    /// Returns the maximum array items.
    #[must_use]
    pub const fn array_items(self) -> u64 {
        self.array_items
    }

    /// Returns the maximum nesting depth.
    #[must_use]
    pub const fn nesting_depth(self) -> u64 {
        self.nesting_depth
    }

    /// Returns the maximum total node count.
    #[must_use]
    pub const fn total_nodes(self) -> u64 {
        self.total_nodes
    }

    /// Returns the maximum nominal type count.
    #[must_use]
    pub const fn types(self) -> u64 {
        self.types
    }

    /// Returns the maximum fields per type or record default.
    #[must_use]
    pub const fn fields(self) -> u64 {
        self.fields
    }

    /// Returns the maximum structural feature count.
    #[must_use]
    pub const fn features(self) -> u64 {
        self.features
    }
}
