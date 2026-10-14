// SPDX-License-Identifier: Apache-2.0

//! Exact nominal public binding discovery; templates are contracts, never runtime entries.

use super::{
    ExpressionReadError, ExpressionView, ExpressionViewLimits, ValidatedExpressionProject,
    view::{self, Budget},
};
use crate::consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection};
use neutral_core::{CancellationToken, allocation::RetainCapacity};
use neutral_ir::{
    composition::project::CompositionSignature as S,
    expression::{profile::SemanticProfiles, project::ExpressionDeclaration},
    project_interface::ProjectPublicType as T,
};

/// Closed selection failures without private names, values or source information.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionEntryError {
    /// Unknown schema, exact profile or feature tuple.
    Unsupported,
    /// No matching public binding for a unique request.
    MissingEntry,
    /// More than one matching public binding for a unique request.
    AmbiguousEntry,
    /// Unknown/private/duplicate explicit roots or a non-nominal contract.
    InvalidInput,
    /// Explicit binding's exact type differs from the requested nominal contract.
    TypeMismatch,
    /// Public interpretation contract closure is absent or inconsistent.
    IncompleteContract,
    /// Independent root/work/storage or fallible reservation limit.
    Limit,
    /// Cooperative cancellation before complete selection publication.
    Cancelled,
}
impl From<ExpressionReadError> for ExpressionEntryError {
    /// Preserves resource/cancellation/schema classes without leaking artifact data.
    fn from(error: ExpressionReadError) -> Self {
        match error {
            ExpressionReadError::Cancelled => Self::Cancelled,
            ExpressionReadError::Limit => Self::Limit,
            ExpressionReadError::Schema => Self::Unsupported,
            _ => Self::IncompleteContract,
        }
    }
}

/// Canonically ordered public binding authorities from one complete validated project.
pub struct SelectedExpressionEntries<'p> {
    /// Complete authority retained only for dependency-closed projection.
    project: &'p ValidatedExpressionProject,
    /// Remaining independent selection/projection controls.
    limits: ExpressionViewLimits,
    /// Public bindings only; definitions and templates cannot enter this collection.
    bindings: Vec<&'p ExpressionDeclaration>,
}
impl std::fmt::Debug for SelectedExpressionEntries<'_> {
    /// Exposes counts only, including on a private or malformed caller request.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectedExpressionEntries")
            .field("bindings", &self.bindings.len())
            .finish_non_exhaustive()
    }
}
impl SelectedExpressionEntries<'_> {
    /// Borrows exact public binding entries in stable-symbol order.
    #[must_use]
    pub fn bindings(&self) -> &[&ExpressionDeclaration] {
        &self.bindings
    }
    /// Builds the complete public interpretation closure using the same remaining request policy.
    /// # Errors
    /// Rejects closure/resource/allocation/cancellation failures atomically, without copying symbols.
    pub fn derive_view(
        &self,
        cancel: &CancellationToken,
    ) -> Result<ExpressionView, ExpressionEntryError> {
        view::derive(
            self.project,
            self.bindings.iter().map(|d| &d.identity),
            self.bindings.len(),
            self.limits,
            cancel,
        )
        .map_err(Into::into)
    }
}

impl<'p> ValidatedExpressionProject {
    /// Selects public bindings with an exact registered nominal contract and explicit cardinality policy.
    /// # Errors
    /// Rejects unsupported tuples, missing/ambiguous matches, wrong types, invalid roots, bounds or cancellation.
    pub fn entries(
        &'p self,
        request: &EntryRequest<'_>,
        profiles: &SemanticProfiles,
        limits: ExpressionViewLimits,
        cancel: &CancellationToken,
    ) -> Result<SelectedExpressionEntries<'p>, ExpressionEntryError> {
        use ExpressionEntryError as E;
        let mut budget = Budget::new(self, limits, cancel)?;
        if request.schema != ENTRY_REQUEST_SCHEMA
            || !profiles.matches_frozen()
            || profiles != &self.ir().semantic_profiles
        {
            return Err(E::Unsupported);
        }
        budget.ty(request.contract, 0)?;
        public_contract(self, request.contract, &mut budget)?;
        budget.storage(std::mem::size_of::<SelectedExpressionEntries<'_>>())?;
        let mut bindings = Vec::new();
        let mut candidates = 0_u64;
        match request.selection {
            EntrySelection::Explicit(roots) => {
                if roots.len() as u64 > budget.remaining().roots {
                    return Err(E::Limit);
                }
                for root in roots {
                    budget.symbol(root, self.ir().declarations.len())?;
                    let index = self
                        .ir()
                        .declarations
                        .binary_search_by(|d| d.identity.cmp(root))
                        .map_err(|_| E::InvalidInput)?;
                    let declaration = &self.ir().declarations[index];
                    let S::Binding(ty) = &declaration.signature else {
                        return Err(E::InvalidInput);
                    };
                    if !declaration.public {
                        return Err(E::InvalidInput);
                    }
                    budget.ty(ty, 0)?;
                    if ty != request.contract {
                        return Err(E::TypeMismatch);
                    }
                    retain(&mut bindings, declaration, &mut budget)?;
                }
            }
            EntrySelection::Enumerate | EntrySelection::Unique => {
                for declaration in &self.ir().declarations {
                    budget.work(1)?;
                    if !declaration.public {
                        continue;
                    }
                    let S::Binding(ty) = &declaration.signature else {
                        continue;
                    };
                    budget.ty(ty, 0)?;
                    if ty != request.contract {
                        continue;
                    }
                    candidates = candidates.checked_add(1).ok_or(E::Limit)?;
                    if matches!(request.selection, EntrySelection::Unique) && candidates > 1 {
                        continue;
                    }
                    retain(&mut bindings, declaration, &mut budget)?;
                }
                if matches!(request.selection, EntrySelection::Unique) {
                    match candidates {
                        0 => return Err(E::MissingEntry),
                        1 => {}
                        _ => return Err(E::AmbiguousEntry),
                    }
                }
            }
        }
        for binding in &bindings {
            budget.symbol(&binding.identity, bindings.len())?;
        }
        bindings.sort_unstable_by(|left, right| left.identity.cmp(&right.identity));
        if bindings
            .windows(2)
            .any(|pair| pair[0].identity == pair[1].identity)
        {
            return Err(E::InvalidInput);
        }
        budget.work(0)?;
        Ok(SelectedExpressionEntries {
            project: self,
            limits: budget.remaining(),
            bindings,
        })
    }
}

/// Requires an exact public nominal entry contract before selecting any binding.
fn public_contract(
    project: &ValidatedExpressionProject,
    ty: &T,
    budget: &mut Budget<'_>,
) -> Result<(), ExpressionEntryError> {
    use ExpressionEntryError as E;
    match ty {
        T::Nominal(owner) => {
            budget.symbol(owner, project.ir().declarations.len())?;
            let index = project
                .ir()
                .declarations
                .binary_search_by(|d| d.identity.cmp(owner))
                .map_err(|_| E::IncompleteContract)?;
            let declaration = &project.ir().declarations[index];
            if !declaration.public || !matches!(declaration.signature, S::Definition(_)) {
                return Err(E::IncompleteContract);
            }
        }
        T::VocabularyNominal {
            identity,
            version,
            name,
        } => {
            budget.work(project.ir().vocabularies.len() as u64 + 1)?;
            let bundle = project
                .ir()
                .vocabularies
                .iter()
                .find(|b| b.identity.identity() == identity && b.identity.version() == version)
                .ok_or(E::IncompleteContract)?;
            budget.work(bundle.definitions.len() as u64 + 1)?;
            if !bundle
                .definitions
                .iter()
                .any(|d| d.public && d.name == *name)
            {
                return Err(E::IncompleteContract);
            }
        }
        _ => return Err(E::InvalidInput),
    }
    Ok(())
}

/// Charges each selected reference before a fallible retained-index reservation.
fn retain<'p>(
    bindings: &mut Vec<&'p ExpressionDeclaration>,
    declaration: &'p ExpressionDeclaration,
    budget: &mut Budget<'_>,
) -> Result<(), ExpressionEntryError> {
    if bindings.len() as u64 >= budget.remaining().roots {
        return Err(ExpressionEntryError::Limit);
    }
    budget.storage(std::mem::size_of::<&ExpressionDeclaration>())?;
    bindings
        .try_retain_exact(1)
        .map_err(|_| ExpressionEntryError::Limit)?;
    bindings.push(declaration);
    budget.work(0)?;
    Ok(())
}
