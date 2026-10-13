// SPDX-License-Identifier: Apache-2.0

//! The complete document result passed from producers to independent consumers.
//!
//! `CompilationArtifacts` keeps logical meaning together with source maps,
//! provenance and derivation facts, without making those companions semantic.
//! Logical equivalence compares the document, not its processing history.
//! Construction does not validate consistency; reader validation is the trust boundary.

use crate::{
    DerivationManifest, FieldProvenanceRecord, LogicalDocument, ProvenanceRecord,
    ReferenceProvenanceRecord, ReuseProvenanceRecord, SourceMap,
};

/// Complete immutable successful compiler output before optional encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationArtifacts {
    /// Authoritative logical document.
    logical_document: LogicalDocument,
    /// Original-byte source map.
    source_map: SourceMap,
    /// Value provenance records.
    provenance: Vec<ProvenanceRecord>,
    /// Field-level explicit/default provenance records.
    field_provenance: Vec<FieldProvenanceRecord>,
    /// Ordinary immutable-value reuse edges.
    reuse_provenance: Vec<ReuseProvenanceRecord>,
    /// Typed identity-reference provenance edges.
    reference_provenance: Vec<ReferenceProvenanceRecord>,
    /// Partitioned derivation manifest.
    derivation: DerivationManifest,
}

impl CompilationArtifacts {
    /// Groups a document and its companions without checking cross-record consistency.
    ///
    /// Intended for producers after semantic validation. An untrusted caller can
    /// construct inconsistent data, so construction is not independent reader authority.
    #[must_use]
    pub fn new(
        logical_document: LogicalDocument,
        source_map: SourceMap,
        provenance: Vec<ProvenanceRecord>,
        derivation: DerivationManifest,
    ) -> Self {
        Self {
            logical_document,
            source_map,
            provenance,
            field_provenance: Vec::new(),
            reuse_provenance: Vec::new(),
            reference_provenance: Vec::new(),
            derivation,
        }
    }

    /// Compares only logical payloads under graph-local ID alpha-renaming.
    ///
    /// This deliberately excludes source maps, provenance, derivation facts,
    /// and other companion or envelope records. Exact artifact equality remains
    /// available through `PartialEq` when those records must also match.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.logical_document
            .logically_equivalent(&other.logical_document)
    }

    /// Attaches field-level provenance; does not validate owners, fields or source spans.
    #[must_use]
    pub fn with_field_provenance(mut self, field_provenance: Vec<FieldProvenanceRecord>) -> Self {
        self.field_provenance = field_provenance;
        self
    }

    /// Attaches reuse provenance without resolving or validating dependency edges.
    #[must_use]
    pub fn with_reuse_provenance(mut self, reuse_provenance: Vec<ReuseProvenanceRecord>) -> Self {
        self.reuse_provenance = reuse_provenance;
        self
    }

    /// Attaches reference provenance without validating target types or source ownership.
    #[must_use]
    pub fn with_reference_provenance(
        mut self,
        reference_provenance: Vec<ReferenceProvenanceRecord>,
    ) -> Self {
        self.reference_provenance = reference_provenance;
        self
    }

    /// Returns the authoritative logical document.
    #[must_use]
    pub const fn logical_document(&self) -> &LogicalDocument {
        &self.logical_document
    }

    /// Returns the exact original-byte source map.
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Returns deterministic value provenance records.
    #[must_use]
    pub fn provenance(&self) -> &[ProvenanceRecord] {
        &self.provenance
    }

    /// Returns deterministic explicit/default record-field provenance.
    #[must_use]
    pub fn field_provenance(&self) -> &[FieldProvenanceRecord] {
        &self.field_provenance
    }

    /// Returns deterministic ordinary immutable-value reuse edges.
    #[must_use]
    pub fn reuse_provenance(&self) -> &[ReuseProvenanceRecord] {
        &self.reuse_provenance
    }

    /// Returns deterministic typed identity-reference provenance edges.
    #[must_use]
    pub fn reference_provenance(&self) -> &[ReferenceProvenanceRecord] {
        &self.reference_provenance
    }

    /// Returns the partitioned derivation manifest.
    #[must_use]
    pub const fn derivation(&self) -> &DerivationManifest {
        &self.derivation
    }
}
