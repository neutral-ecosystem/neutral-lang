// SPDX-License-Identifier: Apache-2.0

//! Borrowed identity projections from successfully frozen capture, without recapture or I/O.

use crate::{CapturedProject, CapturedProjectSource, CapturedProjectVocabulary};
use neutral_core::CancellationToken;
use neutral_core::allocation::RetainCapacity;
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
        let (sources, vocabularies) =
            capture_facts(self.sources(), self.vocabularies(), limits, cancellation)?;
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

/// Bounded borrowed projection buffers shared by old and successor capture identities.
type CaptureFacts<'a> = (
    Vec<CapturedIdentitySource<'a>>,
    Vec<CapturedIdentityVocabulary<'a>>,
);

/// Projects only verified immutable capture facts, enforcing independent bounds before reservation.
pub(crate) fn capture_facts<'a>(
    captured_sources: &'a [CapturedProjectSource],
    captured_vocabularies: &'a [CapturedProjectVocabulary],
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<CaptureFacts<'a>, IdentityError> {
    if cancellation.is_cancelled() {
        return Err(IdentityError::Cancelled);
    }
    if limits.bytes == 0
        || limits.nodes == 0
        || captured_sources.len() as u64 > limits.nodes.min(MAX_TRANSCRIPT_NODES)
        || captured_vocabularies.len() as u64 > limits.nodes.min(MAX_TRANSCRIPT_NODES)
    {
        return Err(IdentityError::Limit);
    }
    let mut sources = Vec::new();
    sources
        .try_retain(captured_sources.len())
        .map_err(|_| IdentityError::Limit)?;
    for source in captured_sources {
        if cancellation.is_cancelled() {
            return Err(IdentityError::Cancelled);
        }
        sources.push(CapturedIdentitySource {
            module: source.module_id(),
            source_id: source.source_id(),
            digest: source.digest(),
            byte_len: source.bytes().len() as u64,
        });
    }
    let mut vocabularies = Vec::new();
    vocabularies
        .try_retain(captured_vocabularies.len())
        .map_err(|_| IdentityError::Limit)?;
    for vocabulary in captured_vocabularies {
        if cancellation.is_cancelled() {
            return Err(IdentityError::Cancelled);
        }
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
    Ok((sources, vocabularies))
}
