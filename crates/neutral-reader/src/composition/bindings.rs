// SPDX-License-Identifier: Apache-2.0

//! Public-only resolved binding inspection, independent of compiler/source/wire code.

use super::CompositionTypeCatalogue;
use neutral_core::allocation::Shared as Arc;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{CompositionBinding, CompositionBindingReference, ValueOrigin},
};
use neutral_vocabulary::composition::{ValidatedCompositionBinding, ValidatedCompositionBindings};

/// Safe lookup with no distinction between private and absent binding identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionBindingLookupError {
    /// The exact owner is not a public binding in this validated scope.
    UnavailableBinding,
}

/// Public binding values, occurrence facts and checked identity-only target edges.
///
/// This surface does not assert complete project/wire validation or source
/// attribution. The producer cannot supply unvalidated bindings to this reader.
#[derive(Clone)]
pub struct CompositionBindingCatalogue {
    /// Complete validated values; private entries never escape public lookup.
    bindings: Arc<ValidatedCompositionBindings>,
    /// Shared public contracts, without another catalogue or default copy.
    types: CompositionTypeCatalogue,
}

impl std::fmt::Debug for CompositionBindingCatalogue {
    /// Reports public counts, not private names, values, origins or target identifiers.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompositionBindingCatalogue")
            .field("public_bindings", &self.bindings().count())
            .finish_non_exhaustive()
    }
}

impl CompositionBindingCatalogue {
    /// Opens immutable checked values with no compiler dependency or validation bypass.
    #[must_use]
    pub fn from_shared(bindings: Arc<ValidatedCompositionBindings>) -> Self {
        let types = CompositionTypeCatalogue::from_shared(Arc::clone(bindings.scope()));
        Self { bindings, types }
    }

    /// Returns public source/vocabulary contracts with all reference type branches.
    #[must_use]
    pub fn types(&self) -> &CompositionTypeCatalogue {
        &self.types
    }

    /// Enumerates materialized public bindings in canonical owner order.
    pub fn bindings(&self) -> impl Iterator<Item = &CompositionBinding> {
        self.bindings
            .bindings()
            .iter()
            .filter(|binding| binding.binding().public)
            .map(ValidatedCompositionBinding::binding)
    }

    /// Looks up only an exact public binding, with private/missing owners indistinguishable.
    ///
    /// # Errors
    /// Returns `UnavailableBinding` without disclosing private owner or value facts.
    pub fn binding(
        &self,
        owner: &ModuleSymbolIdentity,
    ) -> Result<&CompositionBinding, CompositionBindingLookupError> {
        Ok(self.lookup(owner)?.binding())
    }

    /// Returns canonical value-origin classifications, never fabricated source coordinates.
    ///
    /// # Errors
    /// Rejects private or absent owners with the same safe lookup classification.
    pub fn origins(
        &self,
        owner: &ModuleSymbolIdentity,
    ) -> Result<&[ValueOrigin], CompositionBindingLookupError> {
        Ok(self.lookup(owner)?.origins())
    }

    /// Returns actual binding reference paths/targets, not merely declared reference types.
    ///
    /// Every target is public, invariantly typed and present in this catalogue;
    /// enumerating it does not dereference, acquire or execute a target value.
    ///
    /// # Errors
    /// Rejects private or absent consumers with the same safe lookup classification.
    pub fn references(
        &self,
        owner: &ModuleSymbolIdentity,
    ) -> Result<&[CompositionBindingReference], CompositionBindingLookupError> {
        Ok(self.lookup(owner)?.references())
    }

    /// Performs logarithmic exact-owner lookup without exposing a private entry's existence.
    fn lookup(
        &self,
        owner: &ModuleSymbolIdentity,
    ) -> Result<&ValidatedCompositionBinding, CompositionBindingLookupError> {
        let bindings = self.bindings.bindings();
        bindings
            .binary_search_by(|binding| binding.binding().owner.cmp(owner))
            .ok()
            .and_then(|index| bindings.get(index))
            .filter(|binding| binding.binding().public)
            .ok_or(CompositionBindingLookupError::UnavailableBinding)
    }
}
