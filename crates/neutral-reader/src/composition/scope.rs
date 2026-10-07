// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent public inspection of both exact nominal declaration origins.

use super::{
    CompositionCatalogue, CompositionInspectionLimits, CompositionLookupError,
    CompositionReferenceError, ReferenceTypeDependency, references,
};
use neutral_core::CancellationToken;
use neutral_core::allocation::Shared as Arc;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{ClosedValue, CompositionDefinition, SourceCompositionDefinition},
    project_interface::ProjectPublicType,
};
use neutral_vocabulary::composition::{
    CompositionError, CompositionLimits, ValidatedCompositionScope, ValidatedCompositionValue,
};

/// Public-only reader over a validated resolved source/vocabulary type scope.
///
/// This is not a decoded project/view or a source parser. It exposes complete
/// public contracts without inventing source provenance, aliases or binding edges.
#[derive(Clone)]
pub struct CompositionTypeCatalogue {
    /// Immutable scope with no raw/mutable construction bypass.
    scope: Arc<ValidatedCompositionScope>,
    /// Existing exact vocabulary inspection without duplicate contract storage.
    vocabularies: CompositionCatalogue,
}

impl std::fmt::Debug for CompositionTypeCatalogue {
    /// Logs public counts only, excluding private types, defaults and source owner names.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompositionTypeCatalogue")
            .field("public_source_types", &self.source_types().count())
            .field("vocabularies", &self.vocabularies)
            .finish_non_exhaustive()
    }
}

impl CompositionTypeCatalogue {
    /// Opens validated shared contracts without compiler linkage or reparsing any source.
    #[must_use]
    pub fn from_shared(scope: Arc<ValidatedCompositionScope>) -> Self {
        let vocabularies = CompositionCatalogue::from_shared(Arc::clone(scope.catalogue()));
        Self {
            scope,
            vocabularies,
        }
    }

    /// Returns public vocabulary inspection with exact locked identity/revision lookup.
    #[must_use]
    pub fn vocabularies(&self) -> &CompositionCatalogue {
        &self.vocabularies
    }

    /// Enumerates public source records and variants in canonical module-symbol order.
    pub fn source_types(&self) -> impl Iterator<Item = &SourceCompositionDefinition> {
        self.scope
            .sources()
            .iter()
            .filter(|source| source.definition.public)
    }

    /// Resolves one exact public source owner without distinguishing private from absent.
    ///
    /// # Errors
    /// Returns only `UnavailableType` for a private, missing or incompatible-profile owner.
    pub fn source_type(
        &self,
        owner: &ModuleSymbolIdentity,
    ) -> Result<&CompositionDefinition, CompositionLookupError> {
        self.source_types()
            .find(|source| &source.owner == owner)
            .map(|source| &source.definition)
            .ok_or(CompositionLookupError::UnavailableType)
    }

    /// Returns either exact public nominal contract without cross-origin fallback.
    ///
    /// # Errors
    /// Rejects unknown owners/revisions, private roots or non-nominal type requests.
    pub fn public_type(
        &self,
        owner: &ProjectPublicType,
    ) -> Result<&CompositionDefinition, CompositionLookupError> {
        match owner {
            ProjectPublicType::Nominal(symbol) => self.source_type(symbol),
            ProjectPublicType::VocabularyNominal {
                identity,
                version,
                name,
            } => self.vocabularies.public_type(identity, version, name),
            _ => Err(CompositionLookupError::UnavailableType),
        }
    }

    /// Checks closed supplied data with the same typing/default/constraint engine for both origins.
    ///
    /// Root depth is zero; origins are safe value paths, not fabricated source spans.
    /// Non-null references and source binding reuse belong to later project compilation.
    ///
    /// # Errors
    /// Rejects unavailable public roots, invalid values, exhausted policy or cancellation atomically.
    pub fn materialize(
        &self,
        owner: &ProjectPublicType,
        value: &ClosedValue,
        limits: CompositionLimits,
        cancellation: &CancellationToken,
    ) -> Result<ValidatedCompositionValue, CompositionError> {
        limits.validate()?;
        if cancellation.is_cancelled() {
            return Err(CompositionError::Cancelled);
        }
        self.public_type(owner)
            .map_err(|_| CompositionError::UnknownType)?;
        self.scope.materialize(owner, value, limits, cancellation)
    }

    /// Enumerates reference type contracts across every public field/alternative and wrapper.
    ///
    /// # Errors
    /// Rejects unavailable roots, invalid/exhausted traversal policy or cancellation.
    pub fn reference_types(
        &self,
        owner: &ProjectPublicType,
        limits: CompositionInspectionLimits,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ReferenceTypeDependency<'_>>, CompositionReferenceError> {
        if cancellation.is_cancelled() {
            return Err(CompositionReferenceError::Cancelled);
        }
        let definition = self
            .public_type(owner)
            .map_err(CompositionReferenceError::Lookup)?;
        references::inspect(definition, limits, cancellation)
    }
}
