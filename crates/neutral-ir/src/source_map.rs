// SPDX-License-Identifier: Apache-2.0

//! Original-byte locations for a document and its declared elements.
//!
//! A source digest and byte length anchor declaration, type, name and value spans
//! to one captured input. Element IDs connect entries to the logical document;
//! they are not durable cross-document identities. Consumers must validate the
//! map against its document rather than treating construction as proof of coverage.
//!
//! A span can be ordered yet outside the captured source, or attached to the wrong
//! element. Constructors intentionally do not infer coverage, overlap/containment,
//! duplicate IDs or source authenticity. Those relational checks need the complete
//! document and its independently checked companions. These minimal document
//! entries coexist with current project and fine-grained expression attribution.

use crate::ElementId;
use neutral_core::{ByteSpan, SourceContentDigest};

/// Source-map entry for one declaration and its minimal child components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceMapEntry {
    /// Graph-local declaration identifier.
    element_id: ElementId,
    /// Complete declaration source span.
    declaration_span: ByteSpan,
    /// Explicit type source span.
    type_span: ByteSpan,
    /// Declaration-name source span.
    name_span: ByteSpan,
    /// Explicit value source span.
    value_span: ByteSpan,
}

impl SourceMapEntry {
    /// Creates source accounting for one root declaration.
    /// Accepts already-checked spans without verifying child containment or ID ownership.
    #[must_use]
    pub const fn new(
        element_id: ElementId,
        declaration_span: ByteSpan,
        type_span: ByteSpan,
        name_span: ByteSpan,
        value_span: ByteSpan,
    ) -> Self {
        Self {
            element_id,
            declaration_span,
            type_span,
            name_span,
            value_span,
        }
    }

    /// Returns the declaration element identifier.
    #[must_use]
    pub const fn element_id(self) -> ElementId {
        self.element_id
    }

    /// Returns the complete declaration span.
    #[must_use]
    pub const fn declaration_span(self) -> ByteSpan {
        self.declaration_span
    }

    /// Returns the explicit type span.
    #[must_use]
    pub const fn type_span(self) -> ByteSpan {
        self.type_span
    }

    /// Returns the declaration-name span.
    #[must_use]
    pub const fn name_span(self) -> ByteSpan {
        self.name_span
    }

    /// Returns the explicit value span.
    #[must_use]
    pub const fn value_span(self) -> ByteSpan {
        self.value_span
    }
}

/// Immutable original-byte source map for a validated document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMap {
    /// Exact captured source identity.
    source_digest: SourceContentDigest,
    /// Exact captured source byte length.
    source_byte_length: u64,
    /// Complete module-header source span.
    module_span: ByteSpan,
    /// Declaration mappings.
    entries: Vec<SourceMapEntry>,
}

impl SourceMap {
    /// Creates an immutable source map from validated compiler accounting.
    /// Preserves entry order and claimed length/digest; no input bytes are available
    /// here to check integrity or range bounds.
    #[must_use]
    pub fn new(
        source_digest: SourceContentDigest,
        source_byte_length: u64,
        module_span: ByteSpan,
        entries: Vec<SourceMapEntry>,
    ) -> Self {
        Self {
            source_digest,
            source_byte_length,
            module_span,
            entries,
        }
    }

    /// Returns the exact captured source identity.
    #[must_use]
    pub const fn source_digest(&self) -> SourceContentDigest {
        self.source_digest
    }

    /// Returns the exact captured source byte length.
    #[must_use]
    pub const fn source_byte_length(&self) -> u64 {
        self.source_byte_length
    }

    /// Returns the complete module-header source span.
    #[must_use]
    pub const fn module_span(&self) -> ByteSpan {
        self.module_span
    }

    /// Returns deterministic declaration source mappings.
    #[must_use]
    pub fn entries(&self) -> &[SourceMapEntry] {
        &self.entries
    }

    /// Finds source accounting for one graph-local element.
    /// Linear in entry count and returns the first match. Duplicate-entry rejection
    /// belongs to validation, not this lookup convenience.
    #[must_use]
    pub fn entry(&self, element_id: ElementId) -> Option<&SourceMapEntry> {
        self.entries
            .iter()
            .find(|entry| entry.element_id == element_id)
    }
}
