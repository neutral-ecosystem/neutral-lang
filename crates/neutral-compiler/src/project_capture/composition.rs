// SPDX-License-Identifier: Apache-2.0

//! Private current-source integrity and closed nominal catalogue capture.
//!
//! Sources are captured and their headers checked, not semantically compiled.
//! Publication requires the entire locked vocabulary closure to pass schema,
//! dependency, public-type, default and restriction validation. No resolver,
//! path, URL, environment or failed old-schema retry participates in capture.

use super::{
    CapturedInputs, CapturedProjectRequest, CapturedProjectSource, CapturedProjectVocabulary,
    ProjectCaptureCheckpoint, ProjectCaptureError, ProjectCaptureLimits,
    ProjectCaptureResourceFacts, capture_cancelled, freeze_project,
    scan_vocabulary_requirements_policy, validate_envelope_version, validate_headers_policy,
    validate_sources_cancellable, validate_vocabularies_cancellable,
};
use neutral_core::allocation::Shared as Arc;
use neutral_core::allocation::{RetainCapacity, TryClone, text};
use neutral_core::ordered::OrderedMap;
use neutral_core::{CancellationToken, profile::LanguageProfile};
use neutral_ir::VocabularyIdentity;
use neutral_vocabulary::composition::{
    CapturedCompositionBundle, CompositionError, CompositionLimits, MAX_CAPTURED_BYTES, MAX_WORK,
    ValidatedComposition, validate_composition_closure,
};

/// Closed successor request with explicit feature selection and independent policy.
///
/// This is a private current capture phase. A validated catalogue is not a
/// compiled project or independently checked artifact.
#[derive(Clone)]
pub(crate) struct CapturedCompositionProjectRequest {
    /// Exact bytes, locks and existing capture policy, never acquisition instructions.
    envelope: CapturedProjectRequest,
    /// Caller-supplied feature set; validated exactly, never sorted or inferred.
    required_features: Vec<String>,
    /// Independent catalogue, default and dependency resource policy.
    composition_limits: CompositionLimits,
}

impl CapturedCompositionProjectRequest {
    /// Creates an explicit data-only request; validation happens at capture, not construction.
    #[must_use]
    pub fn new(
        envelope: CapturedProjectRequest,
        required_features: Vec<String>,
        composition_limits: CompositionLimits,
    ) -> Self {
        Self {
            envelope,
            required_features,
            composition_limits,
        }
    }
}

impl std::fmt::Debug for CapturedCompositionProjectRequest {
    /// Logs counts only; captured bytes and host correlation text must not enter diagnostics.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedCompositionProjectRequest")
            .field("sources", &self.envelope.sources.len())
            .field("vocabularies", &self.envelope.vocabularies.len())
            .field("features", &self.required_features.len())
            .finish_non_exhaustive()
    }
}

/// Private captured bytes and checked nominal catalogue shared by current compilation.
#[derive(Clone)]
pub(crate) struct CapturedCompositionProject {
    /// Reuses exact byte ownership only; never exposes a downcast to the /1 compiler.
    pub(super) captured: CapturedInputs,
    /// Fully checked transitive vocabulary contracts shared with independent consumers.
    catalogue: Arc<ValidatedComposition>,
    /// Independent policy preserved for replay and later compilation.
    composition_limits: CompositionLimits,
    /// Module-local alias -> canonical vocabulary identity; no alias enters a type owner.
    aliases: Arc<OrderedMap<String, OrderedMap<String, String>>>,
    /// Exact selected feature set, immutable and retained independently of source syntax.
    required_features: Arc<Vec<String>>,
}

impl std::fmt::Debug for CapturedCompositionProject {
    /// Excludes source bytes, private contracts, aliases and host paths from debug output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedCompositionProject")
            .field("resources", &self.captured.resource_facts())
            .finish_non_exhaustive()
    }
}

impl CapturedCompositionProject {
    /// Returns complete exact sources in canonical module/source order, including private units.
    #[must_use]
    pub fn sources(&self) -> &[CapturedProjectSource] {
        self.captured.sources()
    }

    /// Returns the exact transitive lock cover in canonical identity order.
    #[must_use]
    pub fn vocabularies(&self) -> &[CapturedProjectVocabulary] {
        self.captured.vocabularies()
    }

    /// Returns existing independent capture policy without mutable cancellation state.
    #[must_use]
    pub const fn limits(&self) -> ProjectCaptureLimits {
        self.captured.limits()
    }

    /// Returns explicit capture controls in the immutable derivation transcript order.
    #[must_use]
    pub fn identity_capture_limits(
        &self,
    ) -> [u64; neutral_ir::project_identity::CAPTURE_LIMIT_TAGS.len()] {
        self.captured.identity_capture_limits()
    }

    /// Returns every independent composition budget, preserved exactly for replay.
    #[must_use]
    pub const fn composition_limits(&self) -> CompositionLimits {
        self.composition_limits
    }

    /// Shares the already retained exact feature set without copying host strings.
    pub(crate) fn shared_features(&self) -> Arc<Vec<String>> {
        self.required_features.clone()
    }

    /// Returns exact accepted source and complete-closure byte/count facts.
    #[must_use]
    pub const fn resource_facts(&self) -> ProjectCaptureResourceFacts {
        self.captured.resource_facts()
    }

    /// Shares validated contracts without reparsing input, copying defaults or linking a reader.
    #[must_use]
    pub fn catalogue(&self) -> Arc<ValidatedComposition> {
        Arc::clone(&self.catalogue)
    }

    /// Resolves a module-local source alias to its canonical exact vocabulary owner.
    ///
    /// Returns no contract when the module or alias is absent. Transitive dependencies
    /// are not implicit source aliases; source must explicitly declare a `use`.
    #[must_use]
    pub fn vocabulary_for_alias(&self, module: &str, alias: &str) -> Option<&VocabularyIdentity> {
        let identity = self.aliases.get(module)?.get(alias)?;
        let bundles = self.catalogue.bundles();
        let index = bundles
            .binary_search_by(|bundle| bundle.identity.identity().cmp(identity))
            .ok()?;
        Some(&bundles[index].identity)
    }

    /// Shares the same fallible exact-input reconstruction with current expression replay.
    pub(super) fn replay_envelope(
        &self,
        request_version: &str,
        cancellation: CancellationToken,
    ) -> Result<CapturedProjectRequest, ProjectCaptureError> {
        capture_cancelled(Some(&cancellation))?;
        let mut sources = Vec::new();
        sources
            .try_retain_exact(self.sources().len())
            .map_err(|_| ProjectCaptureError::LimitExceeded)?;
        for source in self.sources() {
            capture_cancelled(Some(&cancellation))?;
            sources.push(
                super::CapturedSourceInput::new(
                    text(source.source_id()).map_err(|_| ProjectCaptureError::LimitExceeded)?,
                    text(source.module_id()).map_err(|_| ProjectCaptureError::LimitExceeded)?,
                    replay_bytes(source.bytes(), &cancellation)?,
                )
                .requiring_digest(source.digest()),
            );
        }
        let mut vocabularies = Vec::new();
        vocabularies
            .try_retain_exact(self.vocabularies().len())
            .map_err(|_| ProjectCaptureError::LimitExceeded)?;
        for vocabulary in self.vocabularies() {
            capture_cancelled(Some(&cancellation))?;
            vocabularies.push(super::CapturedVocabularyInput::new(
                replay_bytes(vocabulary.bytes(), &cancellation)?,
                vocabulary
                    .lock()
                    .try_clone()
                    .map_err(|_| ProjectCaptureError::LimitExceeded)?,
            ));
        }
        let version = text(request_version).map_err(|_| ProjectCaptureError::LimitExceeded)?;
        capture_cancelled(Some(&cancellation))?;
        Ok(CapturedProjectRequest::new(
            version,
            self.captured.profile,
            sources,
            vocabularies,
            super::ProjectCaptureControls::new(self.limits(), cancellation),
        ))
    }
}

/// Copies already bounded exact captured bytes, checking cancellation before retention and copying.
fn replay_bytes(bytes: &[u8], cancel: &CancellationToken) -> Result<Vec<u8>, ProjectCaptureError> {
    capture_cancelled(Some(cancel))?;
    let mut copied = Vec::new();
    copied
        .try_retain_exact(bytes.len())
        .map_err(|_| ProjectCaptureError::LimitExceeded)?;
    capture_cancelled(Some(cancel))?;
    copied.extend_from_slice(bytes);
    Ok(copied)
}

/// Whole-request rejection with phase-specific codes and no raw input or host information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionCaptureFailure {
    /// Existing capture envelope/header/integrity/resource classification.
    Capture(ProjectCaptureError),
    /// Strict vocabulary schema, closure, defaults, restrictions or public-type failure.
    Catalogue(CompositionError),
}

impl CompositionCaptureFailure {
    /// Returns the stable diagnostic identifier of the phase that rejected the complete request.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Capture(error) => error.code(),
            Self::Catalogue(error) => error.diagnostic_code(),
        }
    }
}

impl From<ProjectCaptureError> for CompositionCaptureFailure {
    /// Preserves capture failure precedence and its safe diagnostic classification.
    fn from(error: ProjectCaptureError) -> Self {
        Self::Capture(error)
    }
}

impl From<CompositionError> for CompositionCaptureFailure {
    /// Preserves catalogue failure classification without introducing invented source attribution.
    fn from(error: CompositionError) -> Self {
        Self::Catalogue(error)
    }
}

/// Shared integrity/header/catalogue phases, after an exact boundary-specific selection.
pub(super) fn capture_selected(
    request: CapturedCompositionProjectRequest,
    version: &str,
    features: &[&str],
    mut checkpoint: impl FnMut(ProjectCaptureCheckpoint),
) -> Result<CapturedCompositionProject, CompositionCaptureFailure> {
    let envelope = request.envelope;
    // Unsupported selection wins over zero controls or an already-cancelled
    // request. Never normalize features, infer them from source or retry /1.
    if envelope.request_version != version
        || !request
            .required_features
            .iter()
            .map(String::as_str)
            .eq(features.iter().copied())
    {
        return Err(ProjectCaptureError::InvalidRequest.into());
    }
    if envelope.profile != LanguageProfile::V1_0 {
        return Err(ProjectCaptureError::ProfileMismatch.into());
    }
    if !envelope.controls.limits.all_nonzero() || request.composition_limits.validate().is_err() {
        return Err(ProjectCaptureError::LimitExceeded.into());
    }
    let cancellation = &envelope.controls.cancellation;
    checkpoint(ProjectCaptureCheckpoint::Start);
    capture_cancelled(Some(cancellation))?;
    validate_envelope_version(&envelope, version)?;
    // Intersect successor aggregate ceilings before reserving per-bundle storage
    // or hashing/parsing oversized inputs; generous capture policy is not a bypass.
    validate_catalogue_capture_bounds(&envelope, request.composition_limits)?;
    let values = envelope.controls.limits.values();
    let mut sources = validate_sources_cancellable(envelope.sources, values, Some(cancellation))?;
    validate_vocabularies_cancellable(&envelope.vocabularies, values, Some(cancellation))?;
    checkpoint(ProjectCaptureCheckpoint::Integrity);
    capture_cancelled(Some(cancellation))?;
    let requirements = validate_headers_policy(&sources, envelope.profile, Some(cancellation))?;
    checkpoint(ProjectCaptureCheckpoint::Headers);
    capture_cancelled(Some(cancellation))?;

    let mut roots = Vec::new();
    roots
        .try_retain(requirements.len())
        .map_err(|_| CompositionError::Allocation)?;
    for identity in &requirements {
        capture_cancelled(Some(cancellation))?;
        let lock = envelope
            .vocabularies
            .iter()
            .map(|input| &input.lock)
            .find(|lock| lock.identity() == identity)
            .ok_or(ProjectCaptureError::MissingVocabulary)?;
        roots.push((lock.identity(), lock.version()));
    }
    let mut inputs = Vec::new();
    inputs
        .try_retain(envelope.vocabularies.len())
        .map_err(|_| CompositionError::Allocation)?;
    inputs.extend(
        envelope
            .vocabularies
            .iter()
            .map(|input| CapturedCompositionBundle {
                bytes: &input.bytes,
                lock: &input.lock,
            }),
    );
    let catalogue =
        validate_composition_closure(&inputs, &roots, request.composition_limits, cancellation)?;
    let aliases = capture_aliases(
        sources.iter().map(|(source, _)| source),
        request.composition_limits.work,
        cancellation,
    )?;
    checkpoint(ProjectCaptureCheckpoint::Publish);
    capture_cancelled(Some(cancellation))?;
    let captured = freeze_project(
        cancellation,
        envelope.profile,
        envelope.controls.limits,
        &mut sources,
        envelope.vocabularies,
    )?;
    let result = CapturedCompositionProject {
        captured,
        catalogue: Arc::try_new(catalogue).map_err(|_| ProjectCaptureError::LimitExceeded)?,
        composition_limits: request.composition_limits,
        aliases: Arc::try_new(aliases).map_err(|_| ProjectCaptureError::LimitExceeded)?,
        required_features: Arc::try_new(request.required_features)
            .map_err(|_| ProjectCaptureError::LimitExceeded)?,
    };
    capture_cancelled(Some(cancellation))?;
    Ok(result)
}

/// Retains module-local aliases fallibly, charging sorted-index shifts before insertion.
fn capture_aliases<'a>(
    sources: impl Iterator<Item = &'a super::CapturedSourceInput>,
    mut work: u64,
    cancellation: &CancellationToken,
) -> Result<OrderedMap<String, OrderedMap<String, String>>, ProjectCaptureError> {
    let mut aliases = OrderedMap::new();
    for source in sources {
        capture_cancelled(Some(cancellation))?;
        let text =
            std::str::from_utf8(&source.bytes).map_err(|_| ProjectCaptureError::InvalidHeader)?;
        let mut local = OrderedMap::new();
        for (identity, alias) in scan_vocabulary_requirements_policy(text, Some(cancellation))? {
            capture_cancelled(Some(cancellation))?;
            work = work
                .checked_sub(local.len() as u64 + 1)
                .ok_or(ProjectCaptureError::LimitExceeded)?;
            local
                .insert(alias, identity)
                .map_err(|_| ProjectCaptureError::LimitExceeded)?;
        }
        work = work
            .checked_sub(aliases.len() as u64 + 1)
            .ok_or(ProjectCaptureError::LimitExceeded)?;
        aliases
            .insert(
                neutral_core::allocation::text(&source.module_id)
                    .map_err(|_| ProjectCaptureError::LimitExceeded)?,
                local,
            )
            .map_err(|_| ProjectCaptureError::LimitExceeded)?;
    }
    Ok(aliases)
}

/// Enforces catalogue aggregate count/byte ceilings before proportional capture work.
fn validate_catalogue_capture_bounds(
    envelope: &CapturedProjectRequest,
    limits: CompositionLimits,
) -> Result<(), CompositionCaptureFailure> {
    if super::exceeds(envelope.vocabularies.len(), limits.bundles.min(MAX_WORK)) {
        return Err(CompositionError::Limit.into());
    }
    let mut total = 0_u64;
    for input in &envelope.vocabularies {
        capture_cancelled(Some(&envelope.controls.cancellation))?;
        total = total
            .checked_add(super::length(input.bytes.len()))
            .ok_or(CompositionError::Limit)?;
        if total > limits.captured_bytes.min(MAX_CAPTURED_BYTES) {
            return Err(CompositionError::Limit.into());
        }
    }
    Ok(())
}
