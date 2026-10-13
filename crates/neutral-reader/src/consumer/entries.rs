// SPDX-License-Identifier: Apache-2.0

//! Exact public binding discovery; selections are post-compilation data, never execution.

use super::validation::{Budget, binding, public_type};
use super::{ConsumerBoundary, ConsumerError as E};
use crate::composition::{
    CompositionReadError, CompositionView, CompositionViewLimits, CompositionViewRequest,
    ValidatedCompositionProject, profile,
};
use neutral_core::allocation::{TryClone, text};
use neutral_core::{CancellationToken, allocation::RetainCapacity};
use neutral_ir::{
    ModuleSymbolIdentity, composition::CompositionBinding, project_interface::ProjectPublicType,
};

/// Frozen typed entry request, independent of transport and package versions.
pub const ENTRY_REQUEST_SCHEMA: &str = "neutral.entry-request/1";

/// Explicit selection; an empty explicit list never means all candidates.
#[derive(Clone, Copy)]
pub enum EntrySelection<'a> {
    /// Enumerate every public binding of exactly the requested nominal type.
    Enumerate,
    /// Require exactly one matching public binding.
    Unique,
    /// Select distinct exact public binding symbols, canonically ordered on success.
    Explicit(&'a [ModuleSymbolIdentity]),
}

/// Closed data-only request over a completely validated artifact.
#[derive(Clone, Copy)]
pub struct EntryRequest<'a> {
    /// Must equal [`ENTRY_REQUEST_SCHEMA`].
    pub schema: &'a str,
    /// Exact public nominal contract, including vocabulary revision where applicable.
    pub contract: &'a ProjectPublicType,
    /// Explicit enumeration/cardinality/root policy.
    pub selection: EntrySelection<'a>,
}

/// Atomic canonical selection borrowing only independently checked public bindings.
pub struct SelectedEntries<'p> {
    /// Complete checked authority for dependency-closed projection.
    project: &'p ValidatedCompositionProject,
    /// Remaining independent policy after entry validation/index retention.
    limits: CompositionViewLimits,
    /// Borrowed public authority, never copies of complete private data.
    bindings: Vec<&'p CompositionBinding>,
}
impl SelectedEntries<'_> {
    /// Builds complete redacted interpretation closure under the selection's remaining policy.
    ///
    /// # Errors
    /// Fails atomically for missing closure, cancellation, allocation or independent bounds.
    pub fn derive_view(&self, cancel: &CancellationToken) -> Result<CompositionView, E> {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        // Root-request construction is part of this projection, not free work before its budget starts.
        let mut work = self
            .limits
            .work
            .checked_sub(self.bindings.len() as u64 + 1)
            .ok_or(E::Limit)?;
        let mut bytes = (self.bindings.len() as u64)
            .checked_mul(std::mem::size_of::<ModuleSymbolIdentity>() as u64)
            .ok_or(E::Limit)?;
        for binding in &self.bindings {
            let o = &binding.owner;
            bytes = bytes
                .checked_add(
                    (o.module().language_behavior_version().len()
                        + o.module().module_name().len()
                        + o.declaration_name().len()) as u64,
                )
                .ok_or(E::Limit)?;
        }
        if bytes > self.limits.output_bytes {
            return Err(E::Limit);
        }
        let copying = bytes
            .checked_add(profile::PROJECT_VIEW_SCHEMA.len() as u64 + 1)
            .ok_or(E::Limit)?;
        work = work
            .checked_sub(copying)
            .filter(|remaining| *remaining > 0)
            .ok_or(E::Limit)?;
        let mut roots = Vec::new();
        roots
            .try_retain_exact(self.bindings.len())
            .map_err(|_| E::Limit)?;
        for binding in &self.bindings {
            if cancel.is_cancelled() {
                return Err(E::Cancelled);
            }
            roots.push(binding.owner.try_clone().map_err(|_| E::Limit)?);
        }
        self.project
            .derive_view_with_limits(
                &CompositionViewRequest {
                    schema: text(profile::PROJECT_VIEW_SCHEMA).map_err(|_| E::Limit)?,
                    roots,
                },
                CompositionViewLimits {
                    work,
                    ..self.limits
                },
                cancel,
            )
            .map_err(|e| match e {
                CompositionReadError::Cancelled => E::Cancelled,
                CompositionReadError::Limit => E::Limit,
                CompositionReadError::Schema => E::Unsupported,
                _ => E::IncompleteContract,
            })
    }
    /// Returns stable-symbol-ordered public entries, never type declarations.
    #[must_use]
    pub fn bindings(&self) -> &[&CompositionBinding] {
        &self.bindings
    }
}
impl std::fmt::Debug for SelectedEntries<'_> {
    /// Counts only: diagnostics cannot disclose caller symbols or payloads.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectedEntries")
            .field("count", &self.bindings.len())
            .finish_non_exhaustive()
    }
}

impl<'p> ConsumerBoundary<'p, '_> {
    /// Selects exact public typed entries without changing complete project identity.
    ///
    /// # Errors
    /// Rejects unsupported schemas, private/absent targets, wrong contracts,
    /// duplicate roots, missing/ambiguous unique entries, limits or cancellation.
    pub fn entries(
        &self,
        request: &EntryRequest<'_>,
        cancel: &CancellationToken,
    ) -> Result<SelectedEntries<'p>, E> {
        let mut budget = Budget::new(self.limits, cancel);
        budget.text(request.schema)?;
        if request.schema != ENTRY_REQUEST_SCHEMA {
            return Err(E::Unsupported);
        }
        public_type(self.project, request.contract, &mut budget)?;
        let mut bindings = Vec::new();
        match request.selection {
            EntrySelection::Explicit(roots) => {
                if roots.len() as u64 > self.limits.roots {
                    return Err(E::Limit);
                }
                for root in roots {
                    let entry = binding(self.project, root, &mut budget)?;
                    charge_type(&entry.ty, &mut budget)?;
                    if &entry.ty != request.contract {
                        return Err(E::TypeMismatch);
                    }
                    retain(&mut bindings, entry, self.limits.output_bytes)?;
                    budget.work(1)?;
                }
            }
            EntrySelection::Enumerate | EntrySelection::Unique => {
                let mut candidates = 0_u64;
                for entry in self.project.bindings().bindings() {
                    let entry = entry.binding();
                    budget.work(1)?;
                    if !entry.public {
                        continue;
                    }
                    charge_type(&entry.ty, &mut budget)?;
                    if &entry.ty != request.contract {
                        continue;
                    }
                    candidates = candidates.checked_add(1).ok_or(E::Limit)?;
                    if candidates > self.limits.candidates {
                        return Err(E::Limit);
                    }
                    // Unique selection needs at most one retained candidate, but scans all candidates.
                    if matches!(request.selection, EntrySelection::Unique) && candidates > 1 {
                        continue;
                    }
                    if bindings.len() as u64 >= self.limits.roots {
                        return Err(E::Limit);
                    }
                    retain(&mut bindings, entry, self.limits.output_bytes)?;
                    budget.work(1)?;
                }
                if matches!(request.selection, EntrySelection::Unique) {
                    match candidates {
                        0 => return Err(E::MissingEntry),
                        1 => (),
                        _ => return Err(E::AmbiguousEntry),
                    }
                }
            }
        }
        let mut keys = 0_u64;
        for entry in &bindings {
            budget.authority_symbol(&entry.owner)?;
            let owner = &entry.owner;
            keys = keys
                .checked_add(
                    (owner.module().language_behavior_version().len()
                        + owner.module().module_name().len()
                        + owner.declaration_name().len()) as u64
                        + 1,
                )
                .ok_or(E::Limit)?;
        }
        budget.work(
            keys.checked_mul(u64::from(bindings.len().bit_width()) + 1)
                .ok_or(E::Limit)?,
        )?;
        bindings.sort_unstable_by(|a, b| a.owner.cmp(&b.owner));
        if bindings
            .windows(2)
            .any(|pair| pair[0].owner == pair[1].owner)
        {
            return Err(E::InvalidInput);
        }
        budget.work(1)?;
        let retained = (bindings.len() as u64)
            .checked_mul(std::mem::size_of::<&CompositionBinding>() as u64)
            .ok_or(E::Limit)?;
        Ok(SelectedEntries {
            project: self.project,
            limits: CompositionViewLimits {
                roots: self.limits.roots,
                closure_edges: self.limits.closure_edges,
                work: budget.remaining_work(),
                output_bytes: self.limits.output_bytes - retained,
            },
            bindings,
        })
    }
}

/// Charge nominal comparison keys; unlike-shaped wrapper types compare in constant time.
fn charge_type(ty: &ProjectPublicType, budget: &mut Budget<'_>) -> Result<(), E> {
    match ty {
        ProjectPublicType::Nominal(owner) => budget.authority_symbol(owner),
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => {
            budget.work(identity.len() as u64 + 1)?;
            budget.work(version.len() as u64 + 1)?;
            budget.work(name.len() as u64 + 1)
        }
        _ => budget.work(1),
    }
}

/// Bounds the borrowed output index before each fallible reservation.
fn retain<'p>(
    out: &mut Vec<&'p CompositionBinding>,
    entry: &'p CompositionBinding,
    output: u64,
) -> Result<(), E> {
    let bytes = (out.len() as u64 + 1)
        .checked_mul(std::mem::size_of::<&CompositionBinding>() as u64)
        .ok_or(E::Limit)?;
    if bytes > output {
        return Err(E::Limit);
    }
    out.try_retain_exact(1).map_err(|_| E::Limit)?;
    out.push(entry);
    Ok(())
}
