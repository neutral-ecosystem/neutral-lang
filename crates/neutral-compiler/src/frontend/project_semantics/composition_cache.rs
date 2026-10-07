// SPDX-License-Identifier: Apache-2.0

//! Explicit successor syntax generations; values, contracts and evidence are never cached.

use super::{
    E, Ir, Phase, Root, codes, compile_observed as compile_pipeline, fail, parse, profile,
};
use crate::{CapturedCompositionProject, ProjectCacheLimits, ProjectCacheStats};
use neutral_core::allocation::{Shared as Arc, TryClone, text};
use neutral_core::ordered::OrderedMap as BTreeMap;
use neutral_core::{CancellationToken, SourceContentDigest};

/// Private exact-input parse entry, never accepted from callers or deserialized.
#[derive(Clone)]
struct Entry {
    /// Selected capture grammar and immutable feature context.
    features: Arc<Vec<String>>,
    /// Exact-byte hash is only a fast filter; bytes must also match.
    digest: SourceContentDigest,
    /// Original bytes reject stale entries and digest collisions.
    bytes: Arc<Vec<u8>>,
    /// Private parsed declarations with original byte spans.
    roots: Vec<Root>,
}

#[cfg(test)]
#[path = "../../../tests/project_cache/composition.rs"]
mod tests;

/// Caller-owned, bounded successor syntax cache with atomic successful generations.
///
/// Separate from the old-profile cache: no API accepts external entries or stores
/// graph, vocabulary, source IDs, controls, materialized values or identity results.
pub struct CompositionCompilationCache {
    /// Independent retained original-byte/unit controls.
    limits: ProjectCacheLimits,
    /// Previous successful generation in canonical logical-module order.
    entries: BTreeMap<String, Entry>,
    /// Exact total original source bytes retained.
    source_bytes: u64,
}
impl CompositionCompilationCache {
    /// Creates an empty cache with positive independent retention budgets.
    #[must_use]
    pub fn new(limits: ProjectCacheLimits) -> Option<Self> {
        (limits.source_units != 0 && limits.source_bytes != 0).then(|| Self {
            limits,
            entries: BTreeMap::new(),
            source_bytes: 0,
        })
    }
    /// Returns retained units, not request or semantic-result counts.
    #[must_use]
    pub fn retained_units(&self) -> usize {
        self.entries.len()
    }
    /// Returns exact retained original bytes, not a promise about total heap usage.
    #[must_use]
    pub const fn retained_source_bytes(&self) -> u64 {
        self.source_bytes
    }
    /// Reuses only exact source/module/feature syntax, rebuilding every other phase.
    ///
    /// # Errors
    /// Returns the clean pipeline's failure without publishing a pending cache
    /// generation. Cancellation, invalid new vocabularies and changed controls
    /// cannot be bypassed through a syntax hit.
    pub fn compile(
        &mut self,
        captured: &CapturedCompositionProject,
        cancel: &CancellationToken,
    ) -> Result<(Arc<Ir>, ProjectCacheStats), E> {
        self.compile_observed(captured, cancel, &mut |_| Ok(()))
    }
    /// Shares atomic generation construction with private phase-fault tests, never accepting public hooks.
    fn compile_observed(
        &mut self,
        captured: &CapturedCompositionProject,
        cancel: &CancellationToken,
        observer: &mut impl FnMut(Phase) -> Result<(), E>,
    ) -> Result<(Arc<Ir>, ProjectCacheStats), E> {
        self.compile_retention_observed(captured, cancel, observer, &mut || Ok(()))
    }

    /// Runs request-local fault checkpoints before cache ownership transitions.
    fn compile_retention_observed(
        &mut self,
        captured: &CapturedCompositionProject,
        cancel: &CancellationToken,
        observer: &mut impl FnMut(Phase) -> Result<(), E>,
        retention: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(Arc<Ir>, ProjectCacheStats), E> {
        let mut pending = BTreeMap::new();
        let mut stats = ProjectCacheStats::default();
        let mut pending_bytes = 0_u64;
        let ir = compile_pipeline(
            captured,
            cancel,
            &mut |source| {
                let previous = self.entries.get(source.module_id());
                let reusable = previous.filter(|entry| {
                    entry.features.as_slice() == captured.required_features()
                        && entry.digest == source.digest()
                        && entry.bytes.as_ref() == source.bytes()
                        && entry.roots.iter().all(|root| {
                            root.owner.module().module_name() == source.module_id()
                                && root.owner.module().language_behavior_version()
                                    == neutral_core::profile::V1_SOURCE_PROFILE
                        })
                });
                let roots = if let Some(entry) = reusable {
                    if cancel.is_cancelled() {
                        return Err(fail(codes::CANCELLED, None));
                    }
                    retention()?;
                    stats.reused_units += 1;
                    entry
                        .roots
                        .try_clone()
                        .map_err(|_| fail(codes::LIMIT, None))?
                } else {
                    stats.parsed_units += 1;
                    stats.rejected_entries += u64::from(previous.is_some());
                    parse(source, captured, cancel)?
                };
                let length = source.bytes().len() as u64;
                if (pending.len() as u64) < self.limits.source_units
                    && pending_bytes
                        .checked_add(length)
                        .is_some_and(|n| n <= self.limits.source_bytes)
                {
                    if cancel.is_cancelled() {
                        return Err(fail(codes::CANCELLED, None));
                    }
                    retention()?;
                    let key = text(source.module_id()).map_err(|_| fail(codes::LIMIT, None))?;
                    retention()?;
                    let retained_roots = roots.try_clone().map_err(|_| fail(codes::LIMIT, None))?;
                    retention()?;
                    pending
                        .insert(
                            key,
                            Entry {
                                features: captured.shared_features(),
                                digest: source.digest(),
                                bytes: source.shared_bytes(),
                                roots: retained_roots,
                            },
                        )
                        .map_err(|_| fail(codes::LIMIT, None))?;
                    pending_bytes += length;
                }
                Ok(roots)
            },
            observer,
        )?;
        if cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        // This type's dedicated pipeline fixes capture/grammar selection; no
        // package version or public fingerprint is ever a cache key.
        debug_assert_eq!(ir.schema, profile::PROJECT_IR_SCHEMA);
        self.entries = pending;
        self.source_bytes = pending_bytes;
        Ok((ir, stats))
    }
}
