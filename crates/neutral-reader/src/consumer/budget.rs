// SPDX-License-Identifier: Apache-2.0

//! Shared fallible request budget and public-contract checks for expression consumers.

use super::{ConsumerError, ConsumerLimits};
use neutral_core::CancellationToken;
use neutral_ir::{ModuleSymbolIdentity, project_interface::ProjectPublicType};

/// Per-request accounting; already validated project storage is borrowed, not rematerialized.
pub(super) struct Budget<'a> {
    /// Caller-specific hard caps for one entry request.
    limits: ConsumerLimits,
    /// Cancellation source checked while charging work.
    cancellation: &'a CancellationToken,
    /// Charged retained bytes so far.
    bytes: u64,
    /// Charged traversal work so far.
    work: u64,
}
impl<'a> Budget<'a> {
    /// Starts an empty request budget with the supplied caps and cancellation source.
    pub(super) fn new(limits: ConsumerLimits, cancellation: &'a CancellationToken) -> Self {
        Self {
            limits,
            cancellation,
            bytes: 0,
            work: 0,
        }
    }
    /// Returns traversal work remaining before the request cap.
    pub(super) fn remaining_work(&self) -> u64 {
        self.limits.work - self.work
    }
    /// Returns retained bytes remaining before the request cap.
    pub(super) fn remaining_bytes(&self) -> u64 {
        self.limits.bytes - self.bytes
    }
    /// Charges traversal work and observes cancellation.
    pub(super) fn work(&mut self, count: u64) -> Result<(), ConsumerError> {
        if self.cancellation.is_cancelled() {
            return Err(ConsumerError::Cancelled);
        }
        self.work = self.work.checked_add(count).ok_or(ConsumerError::Limit)?;
        if self.work > self.limits.work {
            return Err(ConsumerError::Limit);
        }
        Ok(())
    }
    /// Charges retained bytes and one unit of traversal work.
    pub(super) fn bytes(&mut self, count: u64) -> Result<(), ConsumerError> {
        self.work(1)?;
        self.bytes = self.bytes.checked_add(count).ok_or(ConsumerError::Limit)?;
        if self.bytes > self.limits.bytes {
            return Err(ConsumerError::Limit);
        }
        Ok(())
    }
    /// Checks and charges one bounded text value.
    pub(super) fn text(&mut self, text: &str) -> Result<(), ConsumerError> {
        self.work(text.len() as u64 + 1)?;
        if text.len() as u64 > self.limits.text_bytes {
            return Err(ConsumerError::Limit);
        }
        self.bytes(text.len() as u64 + 8)?;
        if text.is_empty() || text.chars().any(char::is_control) {
            return Err(ConsumerError::InvalidInput);
        }
        Ok(())
    }
    /// Checks and charges every component of a logical symbol.
    pub(super) fn symbol(&mut self, symbol: &ModuleSymbolIdentity) -> Result<(), ConsumerError> {
        self.text(symbol.module().language_behavior_version())?;
        self.text(symbol.module().module_name())?;
        self.text(symbol.declaration_name())
    }
    /// Charges an already-authorized symbol without validating caller text.
    fn authority_symbol(&mut self, symbol: &ModuleSymbolIdentity) -> Result<(), ConsumerError> {
        self.work(symbol.module().language_behavior_version().len() as u64)?;
        self.work(symbol.module().module_name().len() as u64)?;
        self.work(symbol.declaration_name().len() as u64 + 1)
    }
    /// Checks a bounded sorted text list and charges its retained representation.
    pub(super) fn list(&mut self, values: &[&str], maximum: u64) -> Result<(), ConsumerError> {
        self.work(1)?;
        if values.len() as u64 > maximum {
            return Err(ConsumerError::Limit);
        }
        for value in values {
            self.text(value)?;
        }
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ConsumerError::InvalidInput);
        }
        Ok(())
    }
}

/// Checks an exact public nominal type in source and captured vocabulary contracts.
pub(super) fn public_scope_type(
    scope: &neutral_vocabulary::composition::ValidatedCompositionScope,
    ty: &ProjectPublicType,
    budget: &mut Budget<'_>,
) -> Result<(), ConsumerError> {
    match ty {
        ProjectPublicType::Nominal(owner) => {
            budget.symbol(owner)?;
            for source in scope.sources() {
                budget.authority_symbol(&source.owner)?;
                if &source.owner == owner && source.definition.public {
                    return Ok(());
                }
            }
        }
        ProjectPublicType::VocabularyNominal {
            identity,
            version,
            name,
        } => {
            budget.text(identity)?;
            budget.text(version)?;
            budget.text(name)?;
            for bundle in scope.catalogue().bundles() {
                budget.work(bundle.identity.identity().len() as u64 + 1)?;
                budget.work(bundle.identity.version().len() as u64 + 1)?;
                if bundle.identity.identity() == identity && bundle.identity.version() == version {
                    for definition in &bundle.definitions {
                        budget.work(definition.name.len() as u64 + 1)?;
                        if &definition.name == name && definition.public {
                            return Ok(());
                        }
                    }
                }
            }
        }
        _ => return Err(ConsumerError::TypeMismatch),
    }
    Err(ConsumerError::PrivateTarget)
}
