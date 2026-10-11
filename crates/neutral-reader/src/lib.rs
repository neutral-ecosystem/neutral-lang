// SPDX-License-Identifier: Apache-2.0

//! Validated, immutable access to Neutral artifacts.
//!
//! This crate owns validation at the external artifact boundary and the reader
//! views exposed after validation. It treats encoded data as untrusted and must
//! not acquire inputs or depend on compiler-private representations.

mod validation;
use validation::validate_artifacts;
pub use validation::validate_document_companions;

use neutral_core::SourceLocation;
pub use neutral_ir::VocabularyContract;
use neutral_ir::{
    CompilationArtifacts, Declaration, DeclarationFingerprint, LogicalValue, RecordTypeDefinition,
    ResolvedType, ValueOrigin,
};
use std::sync::Arc;

pub mod composition;
mod project;
mod project_identity;
mod project_interface;

pub use neutral_ir::project_identity::{
    ArtifactIdentityInput, ArtifactKind, CAPTURE_LIMIT_TAGS, CapturedClosureIdentity,
    CapturedIdentityInput, CapturedIdentitySource, CapturedIdentityVocabulary, IDENTITY_PROFILE,
    IdentityError, IdentityLimits, IdentityTranscript, LogicalProjectIdentity,
    MAX_TRANSCRIPT_BYTES, MAX_TRANSCRIPT_NODES, ProjectArtifactIdentity, ProjectDerivationIdentity,
};
pub use project::{ProjectReadError, ProjectView, ValidatedProject};
pub use project_identity::{
    ProjectDerivationFacts, ProjectIdentities, ProjectIdentityContext, ProjectIdentityReadError,
};
pub use project_interface::{ProjectInterfaceError, ValidatedProjectInterface};

pub use neutral_core::profile::{
    LanguageCapability, LanguageProfile, LanguageProfileDescriptor, ProfileAvailability,
};
pub use neutral_ir::ElementId;
pub use neutral_ir::project::{
    PROJECT_RESULT_SCHEMA, PROJECT_VIEW_SCHEMA, ProjectLimits, ViewRequest,
};
pub use neutral_ir::{ModuleSymbolIdentity, project_interface::ProjectPublicEdge};

/// Returns the shared deterministic language-profile capability catalogue.
#[must_use]
pub const fn language_profiles() -> &'static [LanguageProfileDescriptor] {
    neutral_core::profile::language_profiles()
}

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
    /// Returns a fail-closed error for invalid semantics, duplicate elements or
    /// inconsistent derivation, source-map and provenance companions.
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

    /// Returns nominal record schemas in canonical name order.
    #[must_use]
    pub fn record_types(&self) -> &[RecordTypeDefinition] {
        self.artifacts.logical_document().record_types()
    }

    /// Returns the exact captured vocabulary contract, when present.
    #[must_use]
    pub fn vocabulary(&self) -> Option<&VocabularyContract> {
        self.artifacts.logical_document().vocabulary()
    }

    /// Finds one nominal record schema by its validated name.
    #[must_use]
    pub fn record_type_by_name(&self, name: &str) -> Option<&RecordTypeDefinition> {
        self.artifacts.logical_document().record_type_by_name(name)
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

    /// Compares validated logical payloads under document-local ID renaming.
    ///
    /// Source locations, provenance, derivation, and numeric ID spellings are
    /// companion facts and do not participate. Callers must use durable symbol
    /// identities, never retain an `ElementId` for cross-document continuity.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.artifacts.logically_equivalent(&other.artifacts)
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
    /// A declaration value did not satisfy its resolved type and nullability.
    TypeValueMismatch,
    /// A nominal record schema violated ownership, ordering, or recursion rules.
    InvalidRecordSchema,
    /// Field provenance was dangling, duplicated, or used an invalid origin.
    InvalidFieldProvenance,
    /// Reuse provenance was dangling, duplicated, or inconsistent with final values.
    InvalidReuseProvenance,
    /// A typed identity edge or its provenance was dangling or inconsistent.
    InvalidReferenceEdge,
    /// A declaration fingerprint did not match its complete logical definition.
    InvalidDeclarationFingerprint,
    /// Logical and derivation vocabulary contracts were missing or inconsistent.
    InvalidVocabularyContract,
    /// Source spans, coverage or canonical ordering were inconsistent.
    InvalidSourceMap,
    /// Companion provenance coverage or ordering was inconsistent.
    InvalidProvenance,
    /// Resource and derivation facts disagreed with the document or source.
    InvalidDerivation,
    /// Symbol/module ownership or declaration ordering was inconsistent.
    InvalidLogicalIdentity,
}
