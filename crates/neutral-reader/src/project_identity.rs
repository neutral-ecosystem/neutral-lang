// SPDX-License-Identifier: Apache-2.0

//! Typed identity access bound to complete validated project data.

use crate::{ProjectReadError, ValidatedProject, ViewRequest};
use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    project::{PROJECT_VIEW_SCHEMA, ProjectLimits},
    project_identity::{
        ArtifactIdentityInput, ArtifactKind, CAPTURE_LIMIT_TAGS, CapturedClosureIdentity,
        CapturedIdentityInput, DerivationContext, IdentityError, IdentityLimits,
        IdentityTranscript, LogicalProjectIdentity, ProjectArtifactIdentity,
        ProjectDerivationIdentity, artifact_identity, canonical_logical_project, captured_closure,
        derivation_identity,
    },
};

/// Explicit capture-lock and producer facts; no host environment is consulted.
pub struct ProjectIdentityContext<'a> {
    /// Exact capture facts, checked against the validated source companions.
    pub capture: CapturedIdentityInput<'a>,
    /// Stable producer identity, not a host path or executable discovery result.
    pub producer: &'a str,
    /// Explicit producer revision, independent of the transcript profile.
    pub producer_version: &'a str,
    /// Acceptance controls in the frozen capture-limit order.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
}

/// Atomic reader identity failures, never a partially published identity set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectIdentityReadError {
    /// Explicit capture facts do not match the complete project companions.
    CaptureMismatch,
    /// Bounded canonical identity construction failed.
    Identity(IdentityError),
    /// Selected artifact roots are absent, private, duplicate, or cancelled.
    View(ProjectReadError),
}

impl ProjectIdentityReadError {
    /// Returns the shared project failure envelope, never a package release version.
    #[must_use]
    pub const fn schema(self) -> &'static str {
        crate::PROJECT_RESULT_SCHEMA
    }
}

/// Explicit processing facts associated with a complete validated project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDerivationFacts {
    /// Exact captured closure identity, not logical meaning.
    pub captured: CapturedClosureIdentity,
    /// Complete logical project identity, including private declarations.
    pub logical: LogicalProjectIdentity,
    /// Stable producer identifier supplied by the caller.
    pub producer: String,
    /// Explicit producer revision supplied by the caller.
    pub producer_version: String,
    /// Frozen acceptance controls; limits do not enter logical identity.
    pub capture_limits: [u64; CAPTURE_LIMIT_TAGS.len()],
    /// Authoritative project bounds retained in the complete artifact.
    pub project_limits: ProjectLimits,
}

/// Three immutable typed identity layers bound to one validated reader.
#[derive(Debug)]
pub struct ProjectIdentities<'a> {
    /// Complete validated authority used to check artifact selections.
    project: &'a ValidatedProject,
    /// Exact captured-closure transcript.
    captured: IdentityTranscript<CapturedClosureIdentity>,
    /// Complete logical transcript, never a selected public fingerprint.
    logical: IdentityTranscript<LogicalProjectIdentity>,
    /// Explicit processing transcript.
    derivation: IdentityTranscript<ProjectDerivationIdentity>,
    /// Caller-supplied facts bound to the validated project companions.
    facts: ProjectDerivationFacts,
}

impl ProjectIdentities<'_> {
    /// Returns the exact captured-closure transcript and typed digest.
    #[must_use]
    pub const fn captured(&self) -> &IdentityTranscript<CapturedClosureIdentity> {
        &self.captured
    }
    /// Returns the complete logical transcript, independent of root selection.
    #[must_use]
    pub const fn logical(&self) -> &IdentityTranscript<LogicalProjectIdentity> {
        &self.logical
    }
    /// Returns the explicit derivation transcript and typed digest.
    #[must_use]
    pub const fn derivation(&self) -> &IdentityTranscript<ProjectDerivationIdentity> {
        &self.derivation
    }
    /// Returns processing facts without inferring authenticity or authority.
    #[must_use]
    pub const fn facts(&self) -> &ProjectDerivationFacts {
        &self.facts
    }
    /// Derives an artifact transcript only after checking public selection roots.
    ///
    /// # Errors
    /// Rejects private, unknown, or duplicate roots, malformed options, bounds,
    /// or cancellation. No complete-project transcript is mutated.
    pub fn artifact(
        &self,
        input: &ArtifactIdentityInput<'_>,
        limits: IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<IdentityTranscript<ProjectArtifactIdentity>, ProjectIdentityReadError> {
        let transcript = artifact_identity(self.derivation.identity(), input, limits, cancellation)
            .map_err(ProjectIdentityReadError::Identity)?;
        if input.kind == ArtifactKind::View {
            self.project
                .derive_view(
                    &ViewRequest {
                        schema: PROJECT_VIEW_SCHEMA.to_owned(),
                        roots: input.roots.to_vec(),
                    },
                    cancellation,
                )
                .map_err(ProjectIdentityReadError::View)?;
        }
        Ok(transcript)
    }
}

impl ValidatedProject {
    /// Returns complete logical identity through an independently validated reader.
    ///
    /// # Errors
    /// Rejects explicit work bounds or cancellation, without consulting roots.
    pub fn logical_identity(
        &self,
        limits: IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<IdentityTranscript<LogicalProjectIdentity>, IdentityError> {
        canonical_logical_project(self.complete_ir(), limits, cancellation)
    }

    /// Binds exact capture and explicit producer facts to validated complete IR.
    ///
    /// Lock encoding/schema/features and producer claims remain explicit caller
    /// inputs: this API does not authenticate a producer or recapture source bytes.
    ///
    /// # Errors
    /// Rejects mismatched source/vocabulary companions, work bounds, or cancellation.
    pub fn identities(
        &self,
        context: &ProjectIdentityContext<'_>,
        limits: IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<ProjectIdentities<'_>, ProjectIdentityReadError> {
        if cancellation.is_cancelled() {
            return Err(ProjectIdentityReadError::Identity(IdentityError::Cancelled));
        }
        let ir = self.complete_ir();
        let capture = &context.capture;
        if capture.profile != V1_SOURCE_PROFILE
            || capture.sources.len() != ir.sources.len()
            || capture.vocabularies.len() != ir.vocabulary_sources.len()
            || capture
                .sources
                .iter()
                .zip(&ir.sources)
                .any(|(fact, source)| {
                    fact.module != source.module
                        || fact.source_id != source.source_id
                        || fact.digest != source.digest
                        || fact.byte_len != source.byte_len
                })
            || capture
                .vocabularies
                .iter()
                .zip(&ir.vocabulary_sources)
                .any(|(fact, source)| {
                    fact.identity != source.identity
                        || fact.version != source.version
                        || fact.digest != source.digest
                        || fact.byte_len != source.byte_len
                })
        {
            return Err(ProjectIdentityReadError::CaptureMismatch);
        }
        let captured = captured_closure(capture, limits, cancellation)
            .map_err(ProjectIdentityReadError::Identity)?;
        let logical = self
            .logical_identity(limits, cancellation)
            .map_err(ProjectIdentityReadError::Identity)?;
        let derivation = derivation_identity(
            logical.identity(),
            &DerivationContext {
                captured: captured.identity(),
                producer: context.producer,
                producer_version: context.producer_version,
                capture_limits: context.capture_limits,
                project_limits: ir.limits,
            },
            limits,
            cancellation,
        )
        .map_err(ProjectIdentityReadError::Identity)?;
        // Own caller text only after the bounded transcript has accepted it.
        let facts = ProjectDerivationFacts {
            captured: captured.identity(),
            logical: logical.identity(),
            producer: context.producer.to_owned(),
            producer_version: context.producer_version.to_owned(),
            capture_limits: context.capture_limits,
            project_limits: ir.limits,
        };
        Ok(ProjectIdentities {
            project: self,
            captured,
            logical,
            derivation,
            facts,
        })
    }
}
