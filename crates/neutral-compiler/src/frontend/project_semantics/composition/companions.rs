// SPDX-License-Identifier: Apache-2.0

//! Current `/3` source, module and vocabulary companions.

use super::{E, Resolver, codes, fail, reserve, retain};
use neutral_core::allocation::text;
use neutral_core::profile::V1_SOURCE_PROFILE;
use neutral_ir::{
    LogicalModuleIdentity,
    project::{ProjectModule, ProjectSource, ProjectVocabularySource},
};

/// Exact captured accounting and complete logical module topology.
pub(super) struct CapturedCompanions {
    /// All modules, including disconnected and private-only units.
    pub(super) modules: Vec<ProjectModule>,
    /// Logical source identities and byte lengths.
    pub(super) sources: Vec<ProjectSource>,
    /// Exact locked vocabulary capture evidence.
    pub(super) vocabulary_sources: Vec<ProjectVocabularySource>,
}
/// Derives checked captured companions without acquiring any input.
pub(super) fn captured_companions(resolver: &Resolver<'_>) -> Result<CapturedCompanions, E> {
    let captured = resolver.captured;
    let mut modules = Vec::new();
    let mut sources = Vec::new();
    reserve(&mut modules, captured.sources().len())?;
    reserve(&mut sources, captured.sources().len())?;
    for s in captured.sources() {
        if resolver.cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        let mut imports = Vec::new();
        for e in resolver
            .graph
            .edges()
            .iter()
            .filter(|e| e.from() == s.module_id())
        {
            if resolver.cancel.is_cancelled() {
                return Err(fail(codes::CANCELLED, None));
            }
            reserve(&mut imports, 1)?;
            imports.push(retain(text(e.target()))?);
        }
        imports.sort_unstable();
        modules.push(ProjectModule {
            identity: LogicalModuleIdentity::new(
                retain(text(V1_SOURCE_PROFILE))?,
                retain(text(s.module_id()))?,
            ),
            imports,
        });
        sources.push(ProjectSource {
            module: retain(text(s.module_id()))?,
            source_id: retain(text(s.source_id()))?,
            digest: s.digest(),
            byte_len: s.bytes().len() as u64,
        });
    }
    let mut vocabulary_sources = Vec::new();
    reserve(
        &mut vocabulary_sources,
        captured.catalogue().bundles().len(),
    )?;
    for b in captured.catalogue().bundles() {
        if resolver.cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        let Some(source) = captured
            .vocabularies()
            .iter()
            .find(|v| v.lock().identity() == b.identity.identity())
        else {
            return Err(fail(codes::INCOMPATIBLE_VALUE, None));
        };
        vocabulary_sources.push(ProjectVocabularySource {
            identity: retain(text(b.identity.identity()))?,
            version: retain(text(b.identity.version()))?,
            digest: b.identity.content_digest(),
            byte_len: source.bytes().len() as u64,
        });
    }
    Ok(CapturedCompanions {
        modules,
        sources,
        vocabulary_sources,
    })
}

#[cfg(test)]
#[path = "../../../../tests/project_cache/assembly.rs"]
mod tests;
