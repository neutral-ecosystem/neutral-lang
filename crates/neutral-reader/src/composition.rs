// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent catalogue, complete-project and redacted public-view inspection.
//!
//! Catalogue APIs expose contracts; the separate complete-project boundary checks
//! decoded companions and occurrence attribution before deriving public closure.
//! It never reparses source, changes identity or grants effects.

use neutral_core::CancellationToken;
use neutral_core::allocation::Shared as Arc;
use neutral_ir::composition::ClosedValue;
use neutral_ir::{
    VocabularyIdentity,
    composition::{CompositionDefinition, CompositionDependency},
};
use neutral_vocabulary::composition::{
    CompositionError, CompositionLimits, ValidatedComposition, ValidatedCompositionValue,
    materialize_composition_value,
};

mod bindings;
mod identity;
mod project;
mod references;
mod scope;
mod view;
pub use bindings::{CompositionBindingCatalogue, CompositionBindingLookupError};
pub use identity::{
    CompositionDerivationFacts, CompositionIdentities, CompositionIdentityContext,
    CompositionIdentityReadError,
};
pub use neutral_ir::composition::profile;
pub use neutral_ir::composition::project::{
    CompositionAttribution, CompositionDeclaration, CompositionOrigin, CompositionSignature,
};
pub use neutral_ir::composition::{
    CompositionBindingReference, CompositionBody, CompositionBundle, ValuePathSegment,
};
pub use neutral_ir::project_identity::CompositionLogicalIdentity;
pub use neutral_ir::project_identity::canonical_composition_project;
pub use neutral_vocabulary::VocabularyLimits as CompositionScalarLimits;
pub use neutral_vocabulary::composition::CompositionLimits as ProjectCompositionLimits;
pub use project::{CompositionReadError, ValidatedCompositionProject};
pub use references::{
    CompositionInspectionLimits, CompositionReferenceError, ReferenceTypeDependency,
    ReferenceTypeSegment,
};
pub use scope::CompositionTypeCatalogue;
pub use view::{CompositionView, CompositionViewRequest};

/// Safe lookup failures containing no private type names, captured text or host paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionLookupError {
    /// Canonical vocabulary is absent from the captured catalogue.
    UnknownVocabulary,
    /// Semantic revision does not equal the captured exact lock.
    RevisionMismatch,
    /// Type is absent or private; public callers cannot distinguish these cases.
    UnavailableType,
}

/// Immutable public inspection surface over a fully validated complete catalogue.
#[derive(Clone)]
pub struct CompositionCatalogue {
    /// Complete contracts retained internally, never mutably exposed.
    catalogue: Arc<ValidatedComposition>,
}

impl std::fmt::Debug for CompositionCatalogue {
    /// Logs only public catalogue counts, never complete private type contracts.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let public_types = self
            .catalogue
            .bundles()
            .iter()
            .flat_map(|bundle| &bundle.definitions)
            .filter(|definition| definition.public)
            .count();
        formatter
            .debug_struct("CompositionCatalogue")
            .field("vocabularies", &self.catalogue.bundles().len())
            .field("public_types", &public_types)
            .finish_non_exhaustive()
    }
}

impl CompositionCatalogue {
    /// Opens validated contracts without compiler linkage or source reparsing.
    ///
    /// # Errors
    /// Returns [`CompositionError::Allocation`] if shared ownership cannot be allocated.
    pub fn new(catalogue: ValidatedComposition) -> Result<Self, CompositionError> {
        Ok(Self::from_shared(
            Arc::try_new(catalogue).map_err(|_| CompositionError::Allocation)?,
        ))
    }

    /// Opens an immutable validated catalogue without copying contracts or linking the compiler.
    ///
    /// Validation cannot be bypassed: raw composition structures cannot construct
    /// [`ValidatedComposition`]. Sharing does not expose mutable catalogue access.
    #[must_use]
    pub fn from_shared(catalogue: Arc<ValidatedComposition>) -> Self {
        Self { catalogue }
    }

    /// Enumerates exact canonical identity/revision/schema/feature/content facts in order.
    pub fn vocabularies(&self) -> impl Iterator<Item = &VocabularyIdentity> {
        self.catalogue
            .bundles()
            .iter()
            .map(|bundle| &bundle.identity)
    }

    /// Enumerates every public nominal contract for one exact canonical owner.
    ///
    /// # Errors
    /// Rejects unknown identities or mismatched semantic revisions.
    pub fn public_types(
        &self,
        identity: &str,
        version: &str,
    ) -> Result<impl Iterator<Item = &CompositionDefinition>, CompositionLookupError> {
        Ok(self
            .bundle(identity, version)?
            .definitions
            .iter()
            .filter(|definition| definition.public))
    }

    /// Returns a complete public record/variant contract including every alternative and default.
    ///
    /// # Errors
    /// Rejects unknown owners/revisions or an absent/private type.
    pub fn public_type(
        &self,
        identity: &str,
        version: &str,
        name: &str,
    ) -> Result<&CompositionDefinition, CompositionLookupError> {
        self.public_types(identity, version)?
            .find(|definition| definition.name == name)
            .ok_or(CompositionLookupError::UnavailableType)
    }

    /// Returns exactly locked dependencies, never aliases or acquisition locators.
    ///
    /// # Errors
    /// Rejects unknown identities or mismatched semantic revisions.
    pub fn dependencies(
        &self,
        identity: &str,
        version: &str,
    ) -> Result<&[CompositionDependency], CompositionLookupError> {
        Ok(&self.bundle(identity, version)?.dependencies)
    }

    /// Checks supplied closed data and returns meaning plus safe origins using the shared validator.
    ///
    /// No compiler linkage, source reparsing, reference-target invention or project-profile
    /// activation occurs. Errors contain no raw private definition, source bytes or host path.
    ///
    /// # Errors
    /// Rejects unavailable public roots, invalid values, exhausted bounds or cancellation.
    pub fn materialize(
        &self,
        owner: (&str, &str, &str),
        supplied: &ClosedValue,
        limits: CompositionLimits,
        cancellation: &CancellationToken,
    ) -> Result<ValidatedCompositionValue, CompositionError> {
        if cancellation.is_cancelled() {
            return Err(CompositionError::Cancelled);
        }
        // Keep absent and private lookup indistinguishable at the public reader
        // boundary, matching the existing public-type lookup contract.
        self.public_type(owner.0, owner.1, owner.2)
            .map_err(|_| CompositionError::UnknownType)?;
        materialize_composition_value(&self.catalogue, owner, supplied, limits, cancellation)
    }

    /// Enumerates every declared reference type in one public record or variant.
    ///
    /// Includes references in unselected alternatives, lists and nullable wrappers.
    /// Targets are exact nominal contracts, not acquired or executable binding edges.
    ///
    /// # Errors
    /// Rejects unavailable public roots, zero/exhausted budgets, cancellation or allocation failure.
    pub fn reference_types(
        &self,
        owner: (&str, &str, &str),
        limits: CompositionInspectionLimits,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ReferenceTypeDependency<'_>>, CompositionReferenceError> {
        if cancellation.is_cancelled() {
            return Err(CompositionReferenceError::Cancelled);
        }
        let definition = self
            .public_type(owner.0, owner.1, owner.2)
            .map_err(CompositionReferenceError::Lookup)?;
        references::inspect(definition, limits, cancellation)
    }

    /// Resolves exact ownership before publishing public traversal state.
    fn bundle(
        &self,
        identity: &str,
        version: &str,
    ) -> Result<&CompositionBundle, CompositionLookupError> {
        let bundle = self
            .catalogue
            .bundles()
            .iter()
            .find(|bundle| bundle.identity.identity() == identity)
            .ok_or(CompositionLookupError::UnknownVocabulary)?;
        if bundle.identity.version() != version {
            return Err(CompositionLookupError::RevisionMismatch);
        }
        Ok(bundle)
    }
}
