// SPDX-License-Identifier: Apache-2.0

//! Validated structural ceilings shared by source and vocabulary processing.

use crate::CoreError;

/// The deterministic budgets that bound captured-input processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralLimits {
    /// The maximum number of captured source bytes.
    source_bytes: u64,
    /// The maximum number of retained diagnostics.
    diagnostics: u32,
    /// Maximum decoded UTF-8 bytes in one string scalar.
    string_bytes: u64,
    /// Maximum significant decimal digits in one exact numeric scalar.
    numeric_digits: u64,
    /// Maximum absolute decimal scale in one exact numeric scalar.
    numeric_scale: u64,
    /// Maximum root declarations in one source unit.
    declarations: u64,
    /// Maximum fields in one record declaration or value.
    record_fields: u64,
    /// Maximum nested contextual-record value depth.
    nesting_depth: u64,
    /// Maximum items in one list value.
    list_items: u64,
    /// Maximum total recursively parsed value nodes.
    traversal_nodes: u64,
}

impl StructuralLimits {
    /// Creates limits when all bounds are nonzero.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when a required bound is zero.
    pub const fn new(source_bytes: u64, diagnostics: u32) -> Result<Self, CoreError> {
        if source_bytes == 0 || diagnostics == 0 {
            return Err(CoreError::ZeroLimit);
        }
        Ok(Self {
            source_bytes,
            diagnostics,
            string_bytes: source_bytes,
            numeric_digits: source_bytes,
            numeric_scale: source_bytes,
            declarations: source_bytes,
            record_fields: source_bytes,
            nesting_depth: source_bytes,
            list_items: source_bytes,
            traversal_nodes: source_bytes,
        })
    }

    /// Overrides the maximum decoded UTF-8 bytes in one string scalar.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `string_bytes` is zero.
    pub const fn with_string_bytes(mut self, string_bytes: u64) -> Result<Self, CoreError> {
        if string_bytes == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.string_bytes = string_bytes;
        Ok(self)
    }

    /// Overrides the maximum significant decimal digits in one exact number.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `numeric_digits` is zero.
    pub const fn with_numeric_digits(mut self, numeric_digits: u64) -> Result<Self, CoreError> {
        if numeric_digits == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.numeric_digits = numeric_digits;
        Ok(self)
    }

    /// Overrides the maximum absolute decimal scale in one exact number.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `numeric_scale` is zero.
    pub const fn with_numeric_scale(mut self, numeric_scale: u64) -> Result<Self, CoreError> {
        if numeric_scale == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.numeric_scale = numeric_scale;
        Ok(self)
    }

    /// Overrides the maximum root declaration count.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `declarations` is zero.
    pub const fn with_declarations(mut self, declarations: u64) -> Result<Self, CoreError> {
        if declarations == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.declarations = declarations;
        Ok(self)
    }

    /// Overrides the maximum fields in one record declaration or value.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `record_fields` is zero.
    pub const fn with_record_fields(mut self, record_fields: u64) -> Result<Self, CoreError> {
        if record_fields == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.record_fields = record_fields;
        Ok(self)
    }

    /// Overrides the maximum nested contextual-record value depth.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `nesting_depth` is zero.
    pub const fn with_nesting_depth(mut self, nesting_depth: u64) -> Result<Self, CoreError> {
        if nesting_depth == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.nesting_depth = nesting_depth;
        Ok(self)
    }

    /// Overrides the maximum items in one list value.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `list_items` is zero.
    pub const fn with_list_items(mut self, list_items: u64) -> Result<Self, CoreError> {
        if list_items == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.list_items = list_items;
        Ok(self)
    }

    /// Overrides the maximum total recursively parsed value nodes.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when `traversal_nodes` is zero.
    pub const fn with_traversal_nodes(mut self, traversal_nodes: u64) -> Result<Self, CoreError> {
        if traversal_nodes == 0 {
            return Err(CoreError::ZeroLimit);
        }
        self.traversal_nodes = traversal_nodes;
        Ok(self)
    }

    /// Returns the maximum exact captured source-byte count.
    #[must_use]
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }

    /// Returns the maximum retained diagnostic count.
    #[must_use]
    pub const fn diagnostics(self) -> u32 {
        self.diagnostics
    }

    /// Returns the maximum decoded UTF-8 bytes in one string scalar.
    #[must_use]
    pub const fn string_bytes(self) -> u64 {
        self.string_bytes
    }

    /// Returns the maximum significant decimal digits in one exact number.
    #[must_use]
    pub const fn numeric_digits(self) -> u64 {
        self.numeric_digits
    }

    /// Returns the maximum absolute decimal scale in one exact number.
    #[must_use]
    pub const fn numeric_scale(self) -> u64 {
        self.numeric_scale
    }

    /// Returns the maximum root declaration count.
    #[must_use]
    pub const fn declarations(self) -> u64 {
        self.declarations
    }

    /// Returns the maximum fields in one record declaration or value.
    #[must_use]
    pub const fn record_fields(self) -> u64 {
        self.record_fields
    }

    /// Returns the maximum nested contextual-record value depth.
    #[must_use]
    pub const fn nesting_depth(self) -> u64 {
        self.nesting_depth
    }

    /// Returns the maximum items in one list value.
    #[must_use]
    pub const fn list_items(self) -> u64 {
        self.list_items
    }

    /// Returns the maximum total recursively parsed value nodes.
    #[must_use]
    pub const fn traversal_nodes(self) -> u64 {
        self.traversal_nodes
    }
}
