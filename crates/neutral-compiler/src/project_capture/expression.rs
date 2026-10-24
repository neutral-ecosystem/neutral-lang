// SPDX-License-Identifier: Apache-2.0

//! Explicit expression capture; shared integrity phases never retry a historical envelope.

use super::{
    CapturedCompositionProject, CapturedCompositionProjectRequest, CapturedProjectRequest,
    CapturedProjectSource, CapturedProjectVocabulary, CompositionCaptureFailure,
    ProjectCaptureError, ProjectCaptureLimits, ProjectCaptureResourceFacts,
};
use crate::frontend::expression_codes as ec;
use neutral_core::{CancellationToken, allocation::Shared};
use neutral_ir::expression::{
    profile::{self, ExpressionLimits, SemanticProfiles},
    templates::ExpansionLimits,
};
use neutral_vocabulary::composition::{CompositionLimits, ValidatedComposition};

/// Exact successor selection and independent source/semantic/finite-expansion policies.
pub struct CapturedExpressionProjectRequest {
    /// Captured bytes, logical names and original acquisition-independent controls.
    envelope: CapturedProjectRequest,
    /// Exact frozen expression/evaluator/expansion/decimal selectors and features.
    profiles: SemanticProfiles,
    /// Independent nominal catalogue, default and restriction bounds.
    composition_limits: CompositionLimits,
    /// Independent pure expression retention and evaluation bounds.
    expression_limits: ExpressionLimits,
    /// Independent finite compile-time expansion bounds.
    expansion_limits: ExpansionLimits,
}
impl CapturedExpressionProjectRequest {
    /// Creates a data-only request. Capture checks the exact tuple before source parsing.
    #[must_use]
    pub fn new(
        envelope: CapturedProjectRequest,
        profiles: SemanticProfiles,
        composition_limits: CompositionLimits,
        expression_limits: ExpressionLimits,
        expansion_limits: ExpansionLimits,
    ) -> Self {
        Self {
            envelope,
            profiles,
            composition_limits,
            expression_limits,
            expansion_limits,
        }
    }
}

/// Complete immutable bytes/catalogue and exact /3 policies, without evaluator bindings.
pub struct CapturedExpressionProject {
    /// Shared immutable original sources and independently checked nominal catalogue.
    pub(crate) composition: CapturedCompositionProject,
    /// Exact accepted profile tuple, not global activation authority.
    profiles: Shared<SemanticProfiles>,
    /// Caller expression policy retained for compiler/reader intersection.
    expression_limits: ExpressionLimits,
    /// Caller static expansion policy retained for compiler/reader intersection.
    expansion_limits: ExpansionLimits,
}
impl CapturedExpressionProject {
    /// Borrows the captured original source bytes, never host paths.
    #[must_use]
    pub fn sources(&self) -> &[CapturedProjectSource] {
        self.composition.sources()
    }
    /// Borrows the complete exact vocabulary locks and captured bytes.
    #[must_use]
    pub fn vocabularies(&self) -> &[CapturedProjectVocabulary] {
        self.composition.vocabularies()
    }
    /// Returns independent capture policy.
    #[must_use]
    pub fn limits(&self) -> ProjectCaptureLimits {
        self.composition.limits()
    }
    /// Returns independent nominal/default/restriction policy.
    #[must_use]
    pub fn composition_limits(&self) -> CompositionLimits {
        self.composition.composition_limits()
    }
    /// Returns independent expression controls, without changing profile semantics.
    #[must_use]
    pub const fn expression_limits(&self) -> ExpressionLimits {
        self.expression_limits
    }
    /// Returns independent finite expansion controls.
    #[must_use]
    pub const fn expansion_limits(&self) -> ExpansionLimits {
        self.expansion_limits
    }
    /// Borrows the exact accepted semantic tuple.
    #[must_use]
    pub fn semantic_profiles(&self) -> &SemanticProfiles {
        &self.profiles
    }
    /// Shares immutable grammar context with private in-process syntax generations.
    pub(crate) fn shared_profiles(&self) -> Shared<SemanticProfiles> {
        self.profiles.clone()
    }
    /// Borrows the exact sorted required capability set.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        &self.profiles.required_features
    }
    /// Shares already validated nominal vocabulary contracts.
    #[must_use]
    pub fn catalogue(&self) -> Shared<ValidatedComposition> {
        self.composition.catalogue()
    }
    /// Returns independently checked input accounting.
    #[must_use]
    pub fn resource_facts(&self) -> ProjectCaptureResourceFacts {
        self.composition.resource_facts()
    }
    /// Returns captured derivation policy in frozen order.
    #[must_use]
    pub fn identity_capture_limits(
        &self,
    ) -> [u64; neutral_ir::project_identity::CAPTURE_LIMIT_TAGS.len()] {
        self.composition.identity_capture_limits()
    }
}

/// Atomic safe capture rejection, never a successful prefix or raw host input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpressionCaptureFailure {
    /// Exact version/profile/feature selection is unsupported.
    Unsupported,
    /// Existing original-byte integrity/header/catalogue failure.
    Capture(CompositionCaptureFailure),
    /// Invalid independent semantic controls or allocation failure.
    Limit,
}
impl ExpressionCaptureFailure {
    /// Returns the frozen source-phase diagnostic classification.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => ec::EXP_UNSUPPORTED,
            Self::Capture(e) => e.code(),
            Self::Limit => ec::EXP_LIMIT,
        }
    }
}

/// Captures exactly /3 source bytes and validates the complete closed vocabulary cover.
///
/// # Errors
/// Rejects unsupported selection before parsing, then invalid controls, cancellation,
/// integrity, original headers and nominal catalogue failures atomically.
pub fn capture_expression_project(
    request: CapturedExpressionProjectRequest,
) -> Result<CapturedExpressionProject, ExpressionCaptureFailure> {
    if request.envelope.request_version != profile::CAPTURE_REQUEST_VERSION
        || !request.profiles.matches_frozen()
    {
        return Err(ExpressionCaptureFailure::Unsupported);
    }
    if request.expression_limits.values().contains(&0)
        || request.expansion_limits.values().contains(&0)
    {
        return Err(ExpressionCaptureFailure::Limit);
    }
    let profiles = request.profiles;
    let cancellation = request.envelope.controls.cancellation.clone();
    let features = neutral_core::allocation::TryClone::try_clone(&profiles.required_features)
        .map_err(|_| ExpressionCaptureFailure::Limit)?;
    let composition = super::composition::capture_selected(
        CapturedCompositionProjectRequest::new(
            request.envelope,
            features,
            request.composition_limits,
        ),
        profile::CAPTURE_REQUEST_VERSION,
        profile::REQUIRED_FEATURES,
        |_| {},
    )
    .map_err(ExpressionCaptureFailure::Capture)?;
    let profiles = Shared::try_new(profiles).map_err(|_| ExpressionCaptureFailure::Limit)?;
    if cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled.into());
    }
    Ok(CapturedExpressionProject {
        composition,
        profiles,
        expression_limits: request.expression_limits,
        expansion_limits: request.expansion_limits,
    })
}

impl From<ProjectCaptureError> for ExpressionCaptureFailure {
    fn from(error: ProjectCaptureError) -> Self {
        Self::Capture(CompositionCaptureFailure::Capture(error))
    }
}

impl CapturedExpressionProject {
    /// Computes the distinct /3 captured identity from exact retained byte evidence.
    ///
    /// # Errors
    /// Rejects independent transcript bounds, allocation failures or cancellation.
    pub fn identity_transcript(
        &self,
        limits: neutral_ir::project_identity::IdentityLimits,
        cancellation: &CancellationToken,
    ) -> Result<
        neutral_ir::project_identity::IdentityTranscript<
            neutral_ir::project_identity::ExpressionCapturedClosureIdentity,
        >,
        neutral_ir::project_identity::IdentityError,
    > {
        let (sources, vocabularies) = crate::project_identity::capture_facts(
            self.sources(),
            self.vocabularies(),
            limits,
            cancellation,
        )?;
        neutral_ir::project_identity::captured_expression_closure(
            &neutral_ir::project_identity::CapturedIdentityInput {
                profile: neutral_core::profile::V1_SOURCE_PROFILE,
                sources: &sources,
                vocabularies: &vocabularies,
            },
            self.semantic_profiles(),
            limits,
            cancellation,
        )
    }
}
