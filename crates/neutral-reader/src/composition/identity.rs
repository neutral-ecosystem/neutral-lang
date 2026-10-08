// SPDX-License-Identifier: Apache-2.0

//! Successor identity facts bound to independently validated complete companions.

use super::{CompositionReadError, CompositionViewRequest, ValidatedCompositionProject, profile};
use neutral_core::allocation::{copy_slice, text};
use neutral_core::{CancellationToken, SemanticDigest};
use neutral_ir::{
    composition::project::CompositionPolicy,
    project::ProjectLimits,
    project_identity::{
        ArtifactIdentityInput, ArtifactKind, CAPTURE_LIMIT_TAGS, CapturedIdentityInput,
        CompositionArtifactIdentity, CompositionCapturedClosureIdentity,
        CompositionDerivationContext, CompositionDerivationIdentity, CompositionLogicalIdentity,
        IdentityError, IdentityLimits, IdentityTranscript, canonical_composition_project,
        captured_composition_closure, composition_artifact_identity,
        composition_derivation_identity, composition_interface,
    },
};

/// Caller-supplied exact capture and producer context, never ambient machine state.
pub struct CompositionIdentityContext<'a> {
    /// Exact source and lock facts, independently compared with complete companions.
    pub capture: CapturedIdentityInput<'a>,
    /// Explicit closed successor feature set.
    pub required_features: &'a [String],
    /// Stable producer identifier.
    pub producer: &'a str,
    /// Explicit producer revision, not a package-based profile selector.
    pub producer_version: &'a str,
    /// Acceptance controls in the frozen fourteen-position order.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
}

/// Atomic identity or selection failure without partial publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionIdentityReadError {
    /// Caller capture facts disagree with independently validated companions.
    CaptureMismatch,
    /// Bounded transcript construction failed.
    Identity(IdentityError),
    /// Public selection is invalid or could not be retained.
    View(CompositionReadError),
}

/// Explicit derivation facts; borrowed producer text avoids redundant retention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionDerivationFacts<'a> {
    /// Exact successor captured closure, never logical meaning.
    pub captured: CompositionCapturedClosureIdentity,
    /// Complete successor meaning, not a selected public view.
    pub logical: CompositionLogicalIdentity,
    /// Caller-supplied producer identity.
    pub producer: &'a str,
    /// Caller-supplied producer revision.
    pub producer_version: &'a str,
    /// Explicit capture acceptance controls.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
    /// Authoritative complete project controls.
    pub project_limits: ProjectLimits,
    /// Authoritative independent composition controls.
    pub composition_limits: CompositionPolicy,
}

/// Complete identities bound to one reader; view roots cannot mutate these transcripts.
#[derive(Debug)]
pub struct CompositionIdentities<'project, 'context> {
    /// Independent complete authority used to validate selected roots.
    project: &'project ValidatedCompositionProject,
    /// Exact source/lock closure transcript.
    captured: IdentityTranscript<CompositionCapturedClosureIdentity>,
    /// Complete meaning, including private and disconnected declarations.
    logical: IdentityTranscript<CompositionLogicalIdentity>,
    /// Independently recomputed public interpretive projection.
    interface: IdentityTranscript<SemanticDigest>,
    /// Explicit producer and processing policy transcript.
    derivation: IdentityTranscript<CompositionDerivationIdentity>,
    /// Non-authenticating caller facts bound to retained project policy.
    facts: CompositionDerivationFacts<'context>,
}

impl CompositionIdentities<'_, '_> {
    /// Returns the exact captured closure transcript.
    #[must_use]
    pub const fn captured(&self) -> &IdentityTranscript<CompositionCapturedClosureIdentity> {
        &self.captured
    }
    /// Returns the complete logical transcript, independent of view roots.
    #[must_use]
    pub const fn logical(&self) -> &IdentityTranscript<CompositionLogicalIdentity> {
        &self.logical
    }
    /// Returns the independently recomputed public interface transcript.
    #[must_use]
    pub const fn interface(&self) -> &IdentityTranscript<SemanticDigest> {
        &self.interface
    }
    /// Returns the explicit processing transcript.
    #[must_use]
    pub const fn derivation(&self) -> &IdentityTranscript<CompositionDerivationIdentity> {
        &self.derivation
    }
    /// Returns supplied producer facts and authoritative retained policies.
    #[must_use]
    pub const fn facts(&self) -> &CompositionDerivationFacts<'_> {
        &self.facts
    }
    /// Frames an artifact only after independently validating any public root selection.
    ///
    /// # Errors
    /// Rejects malformed format/options/roots, exhausted bounds, allocation failure or cancellation.
    pub fn artifact(
        &self,
        input: &ArtifactIdentityInput<'_>,
        limits: IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<IdentityTranscript<CompositionArtifactIdentity>, CompositionIdentityReadError> {
        let result =
            composition_artifact_identity(self.derivation.identity(), input, limits, cancellation)
                .map_err(CompositionIdentityReadError::Identity)?;
        if input.kind == ArtifactKind::View {
            let request = CompositionViewRequest {
                schema: text(profile::PROJECT_VIEW_SCHEMA)
                    .map_err(|_| CompositionIdentityReadError::Identity(IdentityError::Limit))?,
                roots: copy_slice(input.roots)
                    .map_err(|_| CompositionIdentityReadError::Identity(IdentityError::Limit))?,
            };
            self.project
                .derive_view(&request, cancellation)
                .map_err(CompositionIdentityReadError::View)?;
        }
        if cancellation.is_cancelled() {
            return Err(CompositionIdentityReadError::Identity(
                IdentityError::Cancelled,
            ));
        }
        Ok(result)
    }
}

impl ValidatedCompositionProject {
    /// Binds caller facts to exact decoded companions before exposing typed identity layers.
    ///
    /// Producer claims are explicit, not authenticated. Every lock selector and
    /// feature is compared; source bytes are not reacquired or reparsed.
    ///
    /// # Errors
    /// Rejects mismatched companions, bounded identity failures or cancellation.
    pub fn identities<'context>(
        &self,
        context: &CompositionIdentityContext<'context>,
        limits: IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<CompositionIdentities<'_, 'context>, CompositionIdentityReadError> {
        let captured = captured_composition_closure(
            &context.capture,
            context.required_features,
            limits,
            cancellation,
        )
        .map_err(CompositionIdentityReadError::Identity)?;
        let ir = self.complete_ir();
        if context.capture.sources.len() != ir.sources.len()
            || context.capture.vocabularies.len() != ir.vocabulary_sources.len()
            || context
                .capture
                .sources
                .iter()
                .zip(&ir.sources)
                .any(|(fact, source)| {
                    fact.module != source.module
                        || fact.source_id != source.source_id
                        || fact.digest != source.digest
                        || fact.byte_len != source.byte_len
                })
            || context
                .capture
                .vocabularies
                .iter()
                .zip(&ir.vocabulary_sources)
                .zip(&ir.vocabularies)
                .any(|((fact, source), bundle)| {
                    fact.identity != source.identity
                        || fact.version != source.version
                        || fact.digest != source.digest
                        || fact.byte_len != source.byte_len
                        || fact.encoding_version != bundle.identity.encoding_version()
                        || fact.schema_version != bundle.identity.schema_version()
                        || fact.required_features != bundle.identity.required_features()
                })
        {
            return Err(CompositionIdentityReadError::CaptureMismatch);
        }
        let logical = canonical_composition_project(ir, limits, cancellation)
            .map_err(CompositionIdentityReadError::Identity)?;
        let interface = composition_interface(ir, limits, cancellation)
            .map_err(CompositionIdentityReadError::Identity)?;
        let derivation = composition_derivation_identity(
            logical.identity(),
            &CompositionDerivationContext {
                captured: captured.identity(),
                producer: context.producer,
                producer_version: context.producer_version,
                capture_limits: context.capture_limits,
                project_limits: ir.limits,
                composition_limits: ir.composition_limits,
            },
            limits,
            cancellation,
        )
        .map_err(CompositionIdentityReadError::Identity)?;
        let facts = CompositionDerivationFacts {
            captured: captured.identity(),
            logical: logical.identity(),
            producer: context.producer,
            producer_version: context.producer_version,
            capture_limits: context.capture_limits,
            project_limits: ir.limits,
            composition_limits: ir.composition_limits,
        };
        Ok(CompositionIdentities {
            project: self,
            captured,
            logical,
            interface,
            derivation,
            facts,
        })
    }
}
