// SPDX-License-Identifier: Apache-2.0

//! Explicit bounded in-process syntax reuse; graph, semantics, and companions stay fresh.
//!
//! A generation contains only private parsed source units from a successful run.
//! Entries cannot be supplied by callers or loaded from disk. Reuse is safe only
//! for exact bytes under the same module/profile; source IDs and current controls
//! are deliberately not taken from cached entries. Retention budgets describe
//! retained units and original bytes, not a promise about total heap usage.

use super::{Root, lowering::compile_project_with_parser, parse_roots};
use crate::{CapturedProject, ProjectCompileFailure};
use neutral_core::{CancellationToken, SourceContentDigest};
use neutral_ir::project::ProjectIr;
use std::{collections::BTreeMap, sync::Arc};

/// Independent retained-unit and exact-byte budgets for one caller-owned cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectCacheLimits {
    /// Maximum retained parsed source units.
    pub source_units: u64,
    /// Maximum aggregate retained original source bytes.
    pub source_bytes: u64,
}

/// Observed unit parser execution, not captured replay or inferred equivalence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectCacheStats {
    /// Units actually sent through the lexer and declaration parser this run.
    pub parsed_units: u64,
    /// Units reused from the previous successful cache generation.
    pub reused_units: u64,
    /// Existing module entries rejected for changed bytes or identity context.
    pub rejected_entries: u64,
}

/// Immutable parse entry; no API accepts externally supplied cached syntax.
#[derive(Clone)]
struct Entry {
    /// Logical module context; a misplaced entry cannot reuse another module's AST.
    module: String,
    /// Exact language profile used to parse the source.
    profile: String,
    /// Exact source-byte identity, never a public-interface or logical digest.
    digest: SourceContentDigest,
    /// Original bytes also compared to reject stale or digest-collision entries.
    bytes: Arc<[u8]>,
    /// Private source-sensitive roots, including exact original spans.
    roots: Vec<Root>,
}

/// Caller-owned syntax cache; no process-global state, I/O, clock, or host mapping.
pub struct ProjectCompilationCache {
    /// Explicit retention budgets, separate from per-request compilation controls.
    limits: ProjectCacheLimits,
    /// Only the previous successful generation, keyed by exact logical module.
    entries: BTreeMap<String, Entry>,
    /// Sum of retained exact source bytes.
    source_bytes: u64,
}

impl ProjectCompilationCache {
    /// Creates an empty bounded cache, returning `None` for zero retention budgets.
    #[must_use]
    pub fn new(limits: ProjectCacheLimits) -> Option<Self> {
        (limits.source_units != 0 && limits.source_bytes != 0).then(|| Self {
            limits,
            entries: BTreeMap::new(),
            source_bytes: 0,
        })
    }

    /// Returns retained source units, never the number of compilation requests.
    #[must_use]
    pub fn retained_units(&self) -> usize {
        self.entries.len()
    }

    /// Returns the exact retained source-byte total under the independent budget.
    #[must_use]
    pub const fn retained_source_bytes(&self) -> u64 {
        self.source_bytes
    }

    /// Compiles incrementally using exact source/profile/module-sensitive parse keys.
    ///
    /// Every request rebuilds and validates the graph, vocabulary locks, complete
    /// semantics, values, source maps, provenance, and resource facts. Source IDs,
    /// aliases, controls, private bodies, and roots are never substituted through
    /// a public fingerprint. Only successfully compiled generations are retained;
    /// over-budget entries are skipped deterministically in module order.
    ///
    /// # Errors
    /// Returns the clean compiler's failure and leaves the prior cache generation
    /// unchanged on syntax, graph, semantic, lowering, or cancellation failure.
    pub fn compile(
        &mut self,
        captured: &CapturedProject,
        cancellation: &CancellationToken,
    ) -> Result<(Arc<ProjectIr>, ProjectCacheStats), ProjectCompileFailure> {
        let mut pending = BTreeMap::<String, Entry>::new();
        let mut stats = ProjectCacheStats::default();
        let ir = compile_project_with_parser(captured, cancellation, &mut |source| {
            // Semantics and lowering can request the same roots in one run. The
            // pending generation avoids reparsing and counts each unit only once.
            if let Some(entry) = pending.get(source.module_id()) {
                return Ok(entry.roots.clone());
            }
            let previous = self.entries.get(source.module_id());
            // A logical hash or public fingerprint loses spelling/private facts.
            // Even a matching source digest needs exact bytes to reject collisions.
            let reusable = previous.filter(|entry| {
                entry.module == source.module_id()
                    && entry.profile == captured.profile().source_version()
                    && entry.digest == source.digest()
                    && entry.bytes.as_ref() == source.bytes()
            });
            let roots = if let Some(entry) = reusable {
                stats.reused_units += 1;
                entry.roots.clone()
            } else {
                stats.rejected_entries += u64::from(previous.is_some());
                stats.parsed_units += 1;
                parse_roots(source.module_id(), source.digest(), source.bytes())?
            };
            pending.insert(
                source.module_id().to_owned(),
                Entry {
                    module: source.module_id().to_owned(),
                    profile: captured.profile().source_version().to_owned(),
                    digest: source.digest(),
                    bytes: Arc::from(source.bytes()),
                    roots: roots.clone(),
                },
            );
            Ok(roots)
        })?;
        // Publication follows the same final cancellation check as clean lowering.
        if cancellation.is_cancelled() {
            return Err(ProjectCompileFailure::Lowering {
                code: super::lowering::codes::CANCELLED,
                location: None,
            });
        }
        let mut retained = BTreeMap::new();
        let mut retained_bytes = 0_u64;
        // Replace, rather than accumulate, generations: removed modules cannot
        // remain resident forever. Canonical map order makes budget eviction stable.
        for (module, entry) in pending {
            let length = entry.bytes.len() as u64;
            if (retained.len() as u64) < self.limits.source_units
                && retained_bytes
                    .checked_add(length)
                    .is_some_and(|sum| sum <= self.limits.source_bytes)
            {
                retained_bytes += length;
                retained.insert(module, entry);
            }
        }
        self.entries = retained;
        self.source_bytes = retained_bytes;
        Ok((ir, stats))
    }
}

#[cfg(test)]
#[path = "../../tests/project_cache/mod.rs"]
mod tests;
