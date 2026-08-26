// SPDX-License-Identifier: Apache-2.0

//! Validated, immutable access to Neutral artifacts.
//!
//! This crate owns validation at the external artifact boundary and the reader
//! views exposed after validation. It treats encoded data as untrusted and must
//! not acquire inputs or depend on compiler-private representations.

use neutral_core::SourceLocation;
use neutral_ir::{CompilationArtifacts, Declaration};
use std::{collections::BTreeSet, sync::Arc};

pub use neutral_ir::ElementId;

/// Immutable typed traversal over already-validated in-process artifacts.
#[derive(Clone, Debug)]
pub struct ValidatedDocument {
    /// Shared immutable compiler artifacts.
    artifacts: Arc<CompilationArtifacts>,
}

impl ValidatedDocument {
    /// Validates cross-artifact indexes before exposing immutable reader views.
    ///
    /// # Errors
    ///
    /// Returns a fail-closed error for duplicate elements or missing source and
    /// provenance records.
    pub fn from_compiler_output(artifacts: Arc<CompilationArtifacts>) -> Result<Self, ReaderError> {
        validate_artifacts(&artifacts)?;
        Ok(Self { artifacts })
    }

    /// Returns the logical module name.
    #[must_use]
    pub fn module_name(&self) -> &str {
        self.artifacts.logical_document().module().module_name()
    }

    /// Returns immutable declarations in deterministic order.
    #[must_use]
    pub fn declarations(&self) -> &[Declaration] {
        self.artifacts.logical_document().declarations()
    }

    /// Finds one declaration by its validated source name.
    #[must_use]
    pub fn declaration_by_name(&self, name: &str) -> Option<&Declaration> {
        self.declarations()
            .iter()
            .find(|declaration| declaration.name() == name)
    }

    /// Maps a declaration element to its complete original-byte source location.
    #[must_use]
    pub fn source_location(&self, element_id: ElementId) -> Option<SourceLocation> {
        self.artifacts.source_map().entry(element_id).map(|entry| {
            SourceLocation::new(
                self.artifacts.source_map().source_digest(),
                entry.declaration_span(),
            )
        })
    }

    /// Returns the immutable validated compiler artifacts for advanced readers.
    #[must_use]
    pub const fn artifacts(&self) -> &Arc<CompilationArtifacts> {
        &self.artifacts
    }
}

/// A fail-closed in-process reader validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderError {
    /// Two declarations reused one graph-local element identifier.
    DuplicateElementId,
    /// Two declarations reused one source name.
    DuplicateDeclarationName,
    /// A declaration had no matching source-map entry.
    MissingSourceMapEntry,
    /// A declaration had no matching value-provenance record.
    MissingProvenanceRecord,
}

/// Validates relationships among logical declarations and companion artifacts.
fn validate_artifacts(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    let mut element_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for declaration in artifacts.logical_document().declarations() {
        if !element_ids.insert(declaration.element_id()) {
            return Err(ReaderError::DuplicateElementId);
        }
        if !names.insert(declaration.name()) {
            return Err(ReaderError::DuplicateDeclarationName);
        }
        if artifacts
            .source_map()
            .entry(declaration.element_id())
            .is_none()
        {
            return Err(ReaderError::MissingSourceMapEntry);
        }
        if !artifacts
            .provenance()
            .iter()
            .any(|record| record.element_id() == declaration.element_id())
        {
            return Err(ReaderError::MissingProvenanceRecord);
        }
    }
    Ok(())
}
