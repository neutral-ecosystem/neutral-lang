// SPDX-License-Identifier: Apache-2.0

//! Borrowed identity projections from successfully frozen capture, without recapture or I/O.

use crate::CapturedProject;
use neutral_core::CancellationToken;
use neutral_ir::project_identity::{
    CapturedClosureIdentity, CapturedIdentityInput, CapturedIdentitySource,
    CapturedIdentityVocabulary, IdentityError, IdentityLimits, IdentityTranscript,
    MAX_TRANSCRIPT_NODES, captured_closure,
};

impl CapturedProject {
    /// Returns the exact captured-closure transcript using caller-supplied independent bounds.
    ///
    /// # Errors
    /// Rejects identity bounds, allocation failures, or cancellation; capture meaning is unchanged.
    pub fn identity_transcript(
        &self,
        limits: IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<IdentityTranscript<CapturedClosureIdentity>, IdentityError> {
        if cancellation.is_cancelled() {
            return Err(IdentityError::Cancelled);
        }
        if limits.bytes == 0
            || limits.nodes == 0
            || self.sources().len() as u64 > limits.nodes.min(MAX_TRANSCRIPT_NODES)
            || self.vocabularies().len() as u64 > limits.nodes.min(MAX_TRANSCRIPT_NODES)
        {
            return Err(IdentityError::Limit);
        }
        let mut sources = Vec::new();
        sources
            .try_reserve(self.sources().len())
            .map_err(|_| IdentityError::Limit)?;
        for source in self.sources() {
            sources.push(CapturedIdentitySource {
                module: source.module_id(),
                source_id: source.source_id(),
                digest: source.digest(),
                byte_len: source.bytes().len() as u64,
            });
        }
        let mut vocabularies = Vec::new();
        vocabularies
            .try_reserve(self.vocabularies().len())
            .map_err(|_| IdentityError::Limit)?;
        for vocabulary in self.vocabularies() {
            let lock = vocabulary.lock();
            vocabularies.push(CapturedIdentityVocabulary {
                identity: lock.identity(),
                version: lock.version(),
                encoding_version: lock.encoding_version(),
                schema_version: lock.schema_version(),
                digest: lock.content_digest(),
                byte_len: vocabulary.bytes().len() as u64,
                required_features: lock.required_features(),
            });
        }
        captured_closure(
            &CapturedIdentityInput {
                profile: self.profile().source_version(),
                sources: &sources,
                vocabularies: &vocabularies,
            },
            limits,
            cancellation,
        )
    }

    /// Projects all explicit acceptance controls into the frozen derivation transcript order.
    #[must_use]
    pub fn identity_capture_limits(
        &self,
    ) -> [u64; neutral_ir::project_identity::CAPTURE_LIMIT_TAGS.len()] {
        let value = self.limits().values();
        [
            value.total_source_bytes,
            value.source_bytes_per_unit,
            value.source_units,
            value.source_id_bytes,
            value.module_id_bytes,
            value.vocabulary_units,
            value.vocabulary_bytes_per_unit,
            value.total_vocabulary_bytes,
            value.imports_per_module,
            value.import_edges,
            value.scc_units,
            value.declarations,
            value.diagnostics,
            value.output_bytes,
        ]
    }
}
