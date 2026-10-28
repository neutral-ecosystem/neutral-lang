// SPDX-License-Identifier: Apache-2.0

//! Complete, bounded, and effect-free project capture for the v1 profile.
//!
//! This is the host-input ownership boundary: requests contain already-acquired
//! source bytes and exact vocabulary locks, never resolver callbacks or ambient
//! acquisition authority. Logical IDs connect inputs without giving the compiler
//! a filesystem/network lookup policy. Request builders assemble data; capture
//! performs the definitive envelope, identity, integrity and header checks.
//!
//! Sources are unique by both logical source ID and module ID. Byte digests bind
//! the original spelling, including whitespace, while source headers must agree
//! with the supplied module/profile. Vocabulary requirements are collected from
//! headers; the private composition phase checks their exact transitive lock cover
//! and closed catalogue. None of this is complete source semantic compilation.
//!
//! Successful inputs are sorted into canonical order and moved into fallible shared
//! ownership. Cancellation is observed during traversal and again before publication;
//! a failed attempt never returns an incomplete captured project. Later compilation
//! still enforces graph, declaration, expression and expansion controls retained here.

use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::Shared as Arc;
use neutral_core::allocation::{TryClone, text};
use neutral_core::ordered::OrderedSet;
use neutral_core::{
    CancellationToken, SourceContentDigest, VocabularyContentDigest, profile::LanguageProfile,
};
use neutral_ir::language::is_snake_name;
use neutral_vocabulary::VocabularyLock;

mod composition;
mod expression;
pub use composition::CompositionCaptureFailure;
pub(crate) use composition::{CapturedCompositionProject, CapturedCompositionProjectRequest};
#[cfg(test)]
pub(crate) use expression::capture_expression_observed;
pub use expression::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, ExpressionCaptureFailure,
    capture_expression_project, source_module_name,
};

/// Exact version accepted by the captured-project request boundary.
pub use neutral_ir::expression::profile::CAPTURE_REQUEST_VERSION;

/// Named values for every independent project-capture resource bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectCaptureLimitValues {
    /// Sum of exact source bytes.
    pub total_source_bytes: u64,
    /// Exact source bytes in one unit.
    pub source_bytes_per_unit: u64,
    /// Number of supplied source units.
    pub source_units: u64,
    /// UTF-8 bytes in one logical source ID.
    pub source_id_bytes: u64,
    /// UTF-8 bytes in one logical module ID.
    pub module_id_bytes: u64,
    /// Number of supplied vocabulary inputs.
    pub vocabulary_units: u64,
    /// Exact bundle bytes in one vocabulary input.
    pub vocabulary_bytes_per_unit: u64,
    /// Sum of exact vocabulary bundle bytes.
    pub total_vocabulary_bytes: u64,
    /// Maximum imports retained for one module by graph validation.
    pub imports_per_module: u64,
    /// Maximum aggregate project import edges checked during graph validation.
    pub import_edges: u64,
    /// Maximum modules in one strongly connected import component.
    pub scc_units: u64,
    /// Total project declarations.
    pub declarations: u64,
    /// Retained diagnostics.
    pub diagnostics: u64,
    /// Complete authoritative encoded output bytes.
    pub output_bytes: u64,
}

/// Complete deterministic resource policy for project capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectCaptureLimits(ProjectCaptureLimitValues);

impl ProjectCaptureLimits {
    /// Captures all independent limit values without applying ambient defaults.
    /// Does not reject zero at construction; capture validates every required control.
    #[must_use]
    pub const fn new(values: ProjectCaptureLimitValues) -> Self {
        Self(values)
    }

    /// Returns all exact limit values.
    #[must_use]
    pub const fn values(self) -> ProjectCaptureLimitValues {
        self.0
    }

    /// Returns whether every required bound is nonzero.
    const fn all_nonzero(self) -> bool {
        let value = self.0;
        value.total_source_bytes != 0
            && value.source_bytes_per_unit != 0
            && value.source_units != 0
            && value.source_id_bytes != 0
            && value.module_id_bytes != 0
            && value.vocabulary_units != 0
            && value.vocabulary_bytes_per_unit != 0
            && value.total_vocabulary_bytes != 0
            && value.imports_per_module != 0
            && value.import_edges != 0
            && value.scc_units != 0
            && value.declarations != 0
            && value.diagnostics != 0
            && value.output_bytes != 0
    }
}

/// Deterministic limits and cooperative cancellation for one capture attempt.
#[derive(Clone, Debug)]
pub struct ProjectCaptureControls {
    /// Complete explicit resource bounds.
    limits: ProjectCaptureLimits,
    /// Host-controlled cooperative cancellation signal.
    cancellation: CancellationToken,
}

impl ProjectCaptureControls {
    /// Creates complete processing controls with no ambient defaults.
    #[must_use]
    pub const fn new(limits: ProjectCaptureLimits, cancellation: CancellationToken) -> Self {
        Self {
            limits,
            cancellation,
        }
    }

    /// Returns the exact resource limits.
    #[must_use]
    pub const fn limits(&self) -> ProjectCaptureLimits {
        self.limits
    }
}

/// One exact host-supplied source unit, before validation and freezing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedSourceInput {
    /// Request-scoped inert logical source ID.
    source_id: String,
    /// Exact qualified logical module ID.
    module_id: String,
    /// Exact source bytes.
    bytes: Vec<u8>,
    /// Optional host-required exact content identity.
    expected_digest: Option<SourceContentDigest>,
}

impl CapturedSourceInput {
    /// Creates one source input without a host location or acquisition authority.
    /// Takes ownership without checking names, header agreement or byte integrity.
    #[must_use]
    pub fn new(source_id: impl Into<String>, module_id: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            source_id: source_id.into(),
            module_id: module_id.into(),
            bytes,
            expected_digest: None,
        }
    }

    /// Requires capture to observe one exact source digest.
    /// This assertion is verified against supplied bytes, not substituted for hashing them.
    #[must_use]
    pub const fn requiring_digest(mut self, digest: SourceContentDigest) -> Self {
        self.expected_digest = Some(digest);
        self
    }
}

/// One exact host-supplied vocabulary bundle and immutable semantic lock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedVocabularyInput {
    /// Exact already-acquired bundle bytes.
    bytes: Vec<u8>,
    /// Exact immutable vocabulary lock.
    lock: VocabularyLock,
}

impl CapturedVocabularyInput {
    /// Creates one data-only vocabulary input with no resolver or callback.
    #[must_use]
    pub const fn new(bytes: Vec<u8>, lock: VocabularyLock) -> Self {
        Self { bytes, lock }
    }
}

/// Stable failure from host-side logical source mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectHostError {
    /// One source ID was mapped to different module IDs, bytes, or digest facts.
    ConflictingSourceMapping,
}

impl ProjectHostError {
    /// Returns the stable host-adapter diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ConflictingSourceMapping => "NEU-HOST-001",
        }
    }
}

/// Host-neutral constructor for the closed captured-project request schema.
///
/// CLI, Editor, test, and other hosts acquire bytes independently and submit
/// only logical IDs and exact captured data through this builder. Host paths
/// and locations are never accepted or retained.
#[derive(Clone, Debug)]
pub struct CapturedProjectRequestBuilder {
    /// Exact requested language profile.
    profile: LanguageProfile,
    /// Optional bounded non-semantic project key.
    project_key: Option<String>,
    /// Coalesced source mappings in host submission order.
    sources: Vec<CapturedSourceInput>,
    /// Complete exact vocabulary sequence.
    vocabularies: Vec<CapturedVocabularyInput>,
    /// Complete deterministic controls.
    controls: ProjectCaptureControls,
}

impl CapturedProjectRequestBuilder {
    /// Starts one request using the current exact request schema version.
    #[must_use]
    pub const fn new(profile: LanguageProfile, controls: ProjectCaptureControls) -> Self {
        Self {
            profile,
            project_key: None,
            sources: Vec::new(),
            vocabularies: Vec::new(),
            controls,
        }
    }

    /// Attaches bounded opaque host correlation text.
    #[must_use]
    pub fn with_project_key(mut self, project_key: impl Into<String>) -> Self {
        self.project_key = Some(project_key.into());
        self
    }

    /// Adds or coalesces one host mapping before core capture.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectHostError::ConflictingSourceMapping`] when an existing
    /// source ID has different logical or exact-byte facts.
    pub fn add_source(&mut self, source: CapturedSourceInput) -> Result<(), ProjectHostError> {
        if let Some(existing) = self
            .sources
            .iter()
            .find(|existing| existing.source_id == source.source_id)
        {
            return if existing == &source {
                Ok(())
            } else {
                Err(ProjectHostError::ConflictingSourceMapping)
            };
        }
        self.sources.push(source);
        Ok(())
    }

    /// Adds one exact already-acquired vocabulary input.
    pub fn add_vocabulary(&mut self, vocabulary: CapturedVocabularyInput) {
        self.vocabularies.push(vocabulary);
    }

    /// Produces the single closed request type consumed by core capture.
    #[must_use]
    pub fn build(self) -> CapturedProjectRequest {
        CapturedProjectRequest {
            request_version: CAPTURE_REQUEST_VERSION.to_owned(),
            profile: self.profile,
            project_key: self.project_key,
            sources: self.sources,
            vocabularies: self.vocabularies,
            controls: self.controls,
        }
    }
}

/// Closed host-to-compiler request for one complete supplied project.
///
/// Resolver callbacks are deliberately not part of this API:
///
/// ```compile_fail
/// # use neutral_compiler::CapturedProjectRequest;
/// let request: CapturedProjectRequest = todo!();
/// let _ = request.with_resolver(|identity| identity);
/// ```
#[derive(Clone, Debug)]
pub struct CapturedProjectRequest {
    /// Exact request schema version.
    request_version: String,
    /// Exact requested language profile.
    profile: LanguageProfile,
    /// Optional bounded non-semantic host correlation text.
    project_key: Option<String>,
    /// Complete source-unit sequence.
    sources: Vec<CapturedSourceInput>,
    /// Complete exact vocabulary sequence.
    vocabularies: Vec<CapturedVocabularyInput>,
    /// Complete deterministic controls.
    controls: ProjectCaptureControls,
}

impl CapturedProjectRequest {
    /// Creates a closed data-only request.
    #[must_use]
    pub fn new(
        request_version: impl Into<String>,
        profile: LanguageProfile,
        sources: Vec<CapturedSourceInput>,
        vocabularies: Vec<CapturedVocabularyInput>,
        controls: ProjectCaptureControls,
    ) -> Self {
        Self {
            request_version: request_version.into(),
            profile,
            project_key: None,
            sources,
            vocabularies,
            controls,
        }
    }

    /// Attaches bounded opaque host correlation text that does not affect meaning.
    #[must_use]
    pub fn with_project_key(mut self, project_key: impl Into<String>) -> Self {
        self.project_key = Some(project_key.into());
        self
    }
}

/// One immutable captured source member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedProjectSource {
    /// Request-scoped source ID.
    source_id: Arc<String>,
    /// Qualified logical module ID.
    module_id: Arc<String>,
    /// Exact immutable source bytes.
    bytes: Arc<Vec<u8>>,
    /// Exact source-byte identity.
    digest: SourceContentDigest,
}

impl CapturedProjectSource {
    /// Returns the exact request-scoped source ID.
    #[must_use]
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Returns the exact qualified logical module ID.
    #[must_use]
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Returns the exact immutable source bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Shares already frozen bytes with private syntax caches without allocating another payload.
    pub(crate) fn shared_bytes(&self) -> Arc<Vec<u8>> {
        Arc::clone(&self.bytes)
    }

    /// Returns the exact source-byte identity.
    #[must_use]
    pub const fn digest(&self) -> SourceContentDigest {
        self.digest
    }
}

/// One immutable captured vocabulary member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedProjectVocabulary {
    /// Exact immutable bundle bytes.
    bytes: Arc<Vec<u8>>,
    /// Exact immutable semantic lock.
    lock: VocabularyLock,
}

/// Exact resource facts observed while accepting one complete capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectCaptureResourceFacts {
    /// Number of retained source units.
    source_units: u64,
    /// Sum of retained exact source bytes.
    total_source_bytes: u64,
    /// Number of retained vocabulary inputs.
    vocabulary_units: u64,
    /// Sum of retained exact vocabulary bytes.
    total_vocabulary_bytes: u64,
}

impl ProjectCaptureResourceFacts {
    /// Returns the retained source-unit count.
    #[must_use]
    pub const fn source_units(self) -> u64 {
        self.source_units
    }

    /// Returns the retained exact source-byte sum.
    #[must_use]
    pub const fn total_source_bytes(self) -> u64 {
        self.total_source_bytes
    }

    /// Returns the retained vocabulary-input count.
    #[must_use]
    pub const fn vocabulary_units(self) -> u64 {
        self.vocabulary_units
    }

    /// Returns the retained exact vocabulary-byte sum.
    #[must_use]
    pub const fn total_vocabulary_bytes(self) -> u64 {
        self.total_vocabulary_bytes
    }
}

impl CapturedProjectVocabulary {
    /// Returns the exact canonical vocabulary identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        self.lock.identity()
    }

    /// Returns the exact immutable bundle bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the exact immutable vocabulary lock.
    #[must_use]
    pub const fn lock(&self) -> &VocabularyLock {
        &self.lock
    }
}

/// Immutable, complete, canonically ordered captured project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CapturedInputs {
    /// Exact project profile.
    profile: LanguageProfile,
    /// All supplied sources in `(module ID, source ID)` order.
    sources: Arc<Vec<CapturedProjectSource>>,
    /// All exact vocabularies in canonical-identity order.
    vocabularies: Arc<Vec<CapturedProjectVocabulary>>,
    /// Complete deterministic limits, excluding mutable cancellation state.
    limits: ProjectCaptureLimits,
    /// Exact accepted resource facts.
    resource_facts: ProjectCaptureResourceFacts,
}

impl CapturedInputs {
    /// Returns every supplied source, including disconnected members.
    #[must_use]
    pub fn sources(&self) -> &[CapturedProjectSource] {
        &self.sources
    }

    /// Returns the exact vocabulary cover in canonical identity order.
    #[must_use]
    pub fn vocabularies(&self) -> &[CapturedProjectVocabulary] {
        &self.vocabularies
    }

    /// Returns the complete captured resource limits.
    #[must_use]
    pub const fn limits(&self) -> ProjectCaptureLimits {
        self.limits
    }

    /// Returns exact resource facts observed during capture.
    #[must_use]
    pub const fn resource_facts(&self) -> ProjectCaptureResourceFacts {
        self.resource_facts
    }
}

/// Fail-closed bounded project-capture outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectCaptureError {
    /// `NEU-CAP-001`: the closed request version or envelope was invalid.
    InvalidRequest,
    /// `NEU-CAP-002`: the requested and source-header profiles did not agree.
    ProfileMismatch,
    /// `NEU-CAP-003`: no source unit was supplied.
    EmptySourceSet,
    /// `NEU-CAP-004`: a logical source ID appeared more than once.
    DuplicateSourceId,
    /// `NEU-CAP-005`: a logical module ID appeared more than once.
    DuplicateModuleId,
    /// `NEU-CAP-006`: a requested module ID and source header disagreed.
    ModuleHeaderMismatch,
    /// `NEU-CAP-007`: a required header was missing, malformed, or repeated.
    InvalidHeader,
    /// `NEU-CAP-008`: a required vocabulary input was absent.
    MissingVocabulary,
    /// `NEU-CAP-009`: an unused vocabulary input was supplied.
    ExtraVocabulary,
    /// `NEU-CAP-010`: a vocabulary identity appeared more than once.
    DuplicateVocabulary,
    /// `NEU-CAP-011`: exact source or vocabulary bytes failed integrity checking.
    IntegrityMismatch,
    /// `NEU-CAP-012`: a required limit was zero or exceeded.
    LimitExceeded,
    /// `NEU-CAP-013`: cooperative cancellation was observed.
    Cancelled,
}

impl ProjectCaptureError {
    /// Returns the stable capture diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "NEU-CAP-001",
            Self::ProfileMismatch => "NEU-CAP-002",
            Self::EmptySourceSet => "NEU-CAP-003",
            Self::DuplicateSourceId => "NEU-CAP-004",
            Self::DuplicateModuleId => "NEU-CAP-005",
            Self::ModuleHeaderMismatch => "NEU-CAP-006",
            Self::InvalidHeader => "NEU-CAP-007",
            Self::MissingVocabulary => "NEU-CAP-008",
            Self::ExtraVocabulary => "NEU-CAP-009",
            Self::DuplicateVocabulary => "NEU-CAP-010",
            Self::IntegrityMismatch => "NEU-CAP-011",
            Self::LimitExceeded => "NEU-CAP-012",
            Self::Cancelled => "NEU-CAP-013",
        }
    }
}

/// Deterministic capture handoffs used to prove cancellation behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectCaptureCheckpoint {
    /// Before request validation or traversal.
    Start,
    /// After all exact source and vocabulary integrity checks.
    Integrity,
    /// After every source header has been validated.
    Headers,
    /// Immediately before immutable output publication.
    Publish,
}

/// Checks shared capture controls while retaining explicit, non-fallback profile selection.
fn validate_envelope_version(
    request: &CapturedProjectRequest,
    version: &str,
) -> Result<(), ProjectCaptureError> {
    if request.request_version != version {
        return Err(ProjectCaptureError::InvalidRequest);
    }
    if request.profile != LanguageProfile::V1_0 {
        return Err(ProjectCaptureError::ProfileMismatch);
    }
    let limits = request.controls.limits;
    if !limits.all_nonzero() {
        return Err(ProjectCaptureError::LimitExceeded);
    }
    let values = limits.values();
    if request.sources.is_empty() {
        return Err(ProjectCaptureError::EmptySourceSet);
    }
    if exceeds(request.sources.len(), values.source_units)
        || exceeds(request.vocabularies.len(), values.vocabulary_units)
        || request
            .project_key
            .as_ref()
            .is_some_and(|key| exceeds(key.len(), values.source_id_bytes))
    {
        return Err(ProjectCaptureError::LimitExceeded);
    }
    Ok(())
}

/// Validates exact sources with optional per-member successor cancellation checkpoints.
fn validate_sources_cancellable(
    sources: Vec<CapturedSourceInput>,
    values: ProjectCaptureLimitValues,
    cancellation: Option<&CancellationToken>,
) -> Result<Vec<(CapturedSourceInput, SourceContentDigest)>, ProjectCaptureError> {
    let mut source_ids = OrderedSet::new();
    let mut module_ids = OrderedSet::new();
    let mut total_source_bytes = 0_u64;
    let mut captured_sources = Vec::new();
    captured_sources
        .try_retain(sources.len())
        .map_err(|_| ProjectCaptureError::LimitExceeded)?;
    for source in sources {
        capture_cancelled(cancellation)?;
        if !valid_source_id(&source.source_id) || !valid_module_id(&source.module_id) {
            return Err(ProjectCaptureError::InvalidRequest);
        }
        if exceeds(source.source_id.len(), values.source_id_bytes)
            || exceeds(source.module_id.len(), values.module_id_bytes)
            || exceeds(source.bytes.len(), values.source_bytes_per_unit)
        {
            return Err(ProjectCaptureError::LimitExceeded);
        }
        total_source_bytes = total_source_bytes
            .checked_add(length(source.bytes.len()))
            .ok_or(ProjectCaptureError::LimitExceeded)?;
        if total_source_bytes > values.total_source_bytes {
            return Err(ProjectCaptureError::LimitExceeded);
        }
        if !source_ids
            .insert(
                source
                    .source_id
                    .try_clone()
                    .map_err(|_| ProjectCaptureError::LimitExceeded)?,
            )
            .map_err(|_| ProjectCaptureError::LimitExceeded)?
        {
            return Err(ProjectCaptureError::DuplicateSourceId);
        }
        if !module_ids
            .insert(
                source
                    .module_id
                    .try_clone()
                    .map_err(|_| ProjectCaptureError::LimitExceeded)?,
            )
            .map_err(|_| ProjectCaptureError::LimitExceeded)?
        {
            return Err(ProjectCaptureError::DuplicateModuleId);
        }
        // Hash after byte ceilings and unique ownership checks; assertions never replace content hashing.
        let digest = SourceContentDigest::from_bytes(&source.bytes);
        if source
            .expected_digest
            .is_some_and(|expected| expected != digest)
        {
            return Err(ProjectCaptureError::IntegrityMismatch);
        }
        captured_sources.push((source, digest));
    }
    Ok(captured_sources)
}

/// Checks exact lock integrity without disabling successor per-bundle cancellation.
fn validate_vocabularies_cancellable(
    vocabularies: &[CapturedVocabularyInput],
    values: ProjectCaptureLimitValues,
    cancellation: Option<&CancellationToken>,
) -> Result<OrderedSet<String>, ProjectCaptureError> {
    let mut vocabulary_ids = OrderedSet::new();
    let mut total_vocabulary_bytes = 0_u64;
    for vocabulary in vocabularies {
        capture_cancelled(cancellation)?;
        if exceeds(vocabulary.bytes.len(), values.vocabulary_bytes_per_unit) {
            return Err(ProjectCaptureError::LimitExceeded);
        }
        total_vocabulary_bytes = total_vocabulary_bytes
            .checked_add(length(vocabulary.bytes.len()))
            .ok_or(ProjectCaptureError::LimitExceeded)?;
        if total_vocabulary_bytes > values.total_vocabulary_bytes {
            return Err(ProjectCaptureError::LimitExceeded);
        }
        if !vocabulary_ids
            .insert(
                text(vocabulary.lock.identity()).map_err(|_| ProjectCaptureError::LimitExceeded)?,
            )
            .map_err(|_| ProjectCaptureError::LimitExceeded)?
        {
            return Err(ProjectCaptureError::DuplicateVocabulary);
        }
        if VocabularyContentDigest::from_bytes(&vocabulary.bytes)
            != vocabulary.lock.content_digest()
        {
            return Err(ProjectCaptureError::IntegrityMismatch);
        }
    }
    Ok(vocabulary_ids)
}

/// Checks exact profile/module agreement and aggregates canonical vocabulary requirements.
fn validate_headers_policy(
    sources: &[(CapturedSourceInput, SourceContentDigest)],
    profile: LanguageProfile,
    cancellation: Option<&CancellationToken>,
) -> Result<OrderedSet<String>, ProjectCaptureError> {
    let mut required_vocabularies = OrderedSet::new();
    for (source, _) in sources {
        capture_cancelled(cancellation)?;
        let headers = scan_headers(&source.bytes, cancellation)?;
        if headers.profile != profile.source_version() {
            return Err(ProjectCaptureError::ProfileMismatch);
        }
        if headers.module_id != source.module_id {
            return Err(ProjectCaptureError::ModuleHeaderMismatch);
        }
        for identity in headers.vocabularies {
            capture_cancelled(cancellation)?;
            required_vocabularies
                .insert(identity)
                .map_err(|_| ProjectCaptureError::LimitExceeded)?;
        }
    }
    Ok(required_vocabularies)
}

/// Canonically orders and freezes a completely validated request.
fn freeze_project(
    cancellation: &CancellationToken,
    profile: LanguageProfile,
    limits: ProjectCaptureLimits,
    sources: &mut Vec<(CapturedSourceInput, SourceContentDigest)>,
    mut vocabularies: Vec<CapturedVocabularyInput>,
) -> Result<CapturedInputs, ProjectCaptureError> {
    let resource_facts = ProjectCaptureResourceFacts {
        source_units: length(sources.len()),
        total_source_bytes: sources
            .iter()
            .map(|(source, _)| length(source.bytes.len()))
            .sum(),
        vocabulary_units: length(vocabularies.len()),
        total_vocabulary_bytes: vocabularies
            .iter()
            .map(|vocabulary| length(vocabulary.bytes.len()))
            .sum(),
    };
    // Canonicalize after integrity succeeds; acquisition order must not leak into later traversal.
    sources.sort_unstable_by(|(left, _), (right, _)| {
        (&left.module_id, &left.source_id).cmp(&(&right.module_id, &right.source_id))
    });
    let mut frozen_sources = Vec::new();
    frozen_sources
        .try_retain_exact(sources.len())
        .map_err(|_| ProjectCaptureError::LimitExceeded)?;
    for (source, digest) in std::mem::take(sources) {
        capture_cancelled(Some(cancellation))?;
        frozen_sources.push(CapturedProjectSource {
            source_id: Arc::try_new(source.source_id)
                .map_err(|_| ProjectCaptureError::LimitExceeded)?,
            module_id: Arc::try_new(source.module_id)
                .map_err(|_| ProjectCaptureError::LimitExceeded)?,
            bytes: Arc::try_new(source.bytes).map_err(|_| ProjectCaptureError::LimitExceeded)?,
            digest,
        });
    }
    vocabularies.sort_unstable_by(|left, right| left.lock.identity().cmp(right.lock.identity()));
    let mut frozen_vocabularies = Vec::new();
    frozen_vocabularies
        .try_retain_exact(vocabularies.len())
        .map_err(|_| ProjectCaptureError::LimitExceeded)?;
    for vocabulary in vocabularies {
        capture_cancelled(Some(cancellation))?;
        frozen_vocabularies.push(CapturedProjectVocabulary {
            bytes: Arc::try_new(vocabulary.bytes)
                .map_err(|_| ProjectCaptureError::LimitExceeded)?,
            lock: vocabulary.lock,
        });
    }
    if cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled);
    }
    let captured = CapturedInputs {
        profile,
        sources: Arc::try_new(frozen_sources).map_err(|_| ProjectCaptureError::LimitExceeded)?,
        vocabularies: Arc::try_new(frozen_vocabularies)
            .map_err(|_| ProjectCaptureError::LimitExceeded)?,
        limits,
        resource_facts,
    };
    capture_cancelled(Some(cancellation))?;
    Ok(captured)
}

/// Minimal bounded header facts required by project capture.
struct HeaderFacts<'a> {
    /// Exact unescaped profile text.
    profile: &'a str,
    /// Exact qualified module ID.
    module_id: &'a str,
    /// Exact required vocabulary identities.
    vocabularies: OrderedSet<String>,
}

/// Scans explicit current headers before declaration parsing.
/// This narrow capture grammar requires exact leading header lines; the later
/// token scanner handles import placement and declaration syntax independently.
fn scan_headers<'a>(
    bytes: &'a [u8],
    cancellation: Option<&CancellationToken>,
) -> Result<HeaderFacts<'a>, ProjectCaptureError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ProjectCaptureError::InvalidHeader)?;
    let mut lines = source.lines();
    let language = lines.next().ok_or(ProjectCaptureError::InvalidHeader)?;
    let profile = language
        .strip_prefix("neu \"")
        .and_then(|value| value.strip_suffix('"'))
        .filter(|value| !value.is_empty() && !value.contains(['"', '\\']))
        .ok_or(ProjectCaptureError::InvalidHeader)?;
    let module_line = lines.next().ok_or(ProjectCaptureError::InvalidHeader)?;
    let module_id = module_line
        .strip_prefix("module ")
        .filter(|value| valid_module_id(value))
        .ok_or(ProjectCaptureError::InvalidHeader)?;
    let mut vocabularies = OrderedSet::new();
    for (identity, _) in scan_vocabulary_requirements_policy(source, cancellation)? {
        capture_cancelled(cancellation)?;
        vocabularies
            .insert(identity)
            .map_err(|_| ProjectCaptureError::LimitExceeded)?;
    }
    Ok(HeaderFacts {
        profile,
        module_id,
        vocabularies,
    })
}

/// Scans exact module-local aliases; repeated identities may have distinct aliases.
fn scan_vocabulary_requirements_policy(
    source: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<Vec<(String, String)>, ProjectCaptureError> {
    let mut requirements = Vec::new();
    let mut aliases = OrderedSet::new();
    let mut in_block_comment = false;
    for line in source.lines().skip(2) {
        capture_cancelled(cancellation)?;
        let active = !in_block_comment;
        scan_comment_state(line.as_bytes(), &mut in_block_comment);
        if active && (line.starts_with("neu ") || line.starts_with("module ")) {
            return Err(ProjectCaptureError::InvalidHeader);
        }
        let Some(requirement) = (if active {
            line.strip_prefix("use ")
        } else {
            None
        }) else {
            continue;
        };
        let mut words = requirement.split_ascii_whitespace();
        let identity = words.next().ok_or(ProjectCaptureError::InvalidHeader)?;
        let as_keyword = words.next().ok_or(ProjectCaptureError::InvalidHeader)?;
        let alias = words.next().ok_or(ProjectCaptureError::InvalidHeader)?;
        if words.next().is_some()
            || as_keyword != crate::language::graph_names::AS
            || !valid_vocabulary_identity(identity)
            || !is_snake_name(alias)
        {
            return Err(ProjectCaptureError::InvalidHeader);
        }
        if !aliases
            .insert(alias)
            .map_err(|_| ProjectCaptureError::LimitExceeded)?
        {
            return Err(ProjectCaptureError::InvalidHeader);
        }
        requirements
            .try_retain(1)
            .map_err(|_| ProjectCaptureError::LimitExceeded)?;
        capture_cancelled(cancellation)?;
        requirements.push((
            text(identity).map_err(|_| ProjectCaptureError::LimitExceeded)?,
            text(alias).map_err(|_| ProjectCaptureError::LimitExceeded)?,
        ));
    }
    Ok(requirements)
}

/// Observes cancellation before retaining or publishing capture facts.
fn capture_cancelled(cancellation: Option<&CancellationToken>) -> Result<(), ProjectCaptureError> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        Err(ProjectCaptureError::Cancelled)
    } else {
        Ok(())
    }
}

/// Tracks block-comment boundaries without interpreting quoted source text.
fn scan_comment_state(line: &[u8], in_block: &mut bool) {
    let mut index = 0;
    let mut in_string = false;
    while index < line.len() {
        if *in_block {
            if line[index..].starts_with(b"*/") {
                *in_block = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        // Comment-looking bytes in a literal cannot alter cross-line comment state.
        if in_string {
            match line[index] {
                b'\\' => index += 2,
                b'"' => {
                    in_string = false;
                    index += 1;
                }
                _ => index += 1,
            }
            continue;
        }
        if line[index..].starts_with(b"//") {
            break;
        }
        if line[index..].starts_with(b"/*") {
            *in_block = true;
            index += 2;
        } else if line[index] == b'"' {
            in_string = true;
            index += 1;
        } else {
            index += 1;
        }
    }
}

/// Converts a platform length into a saturating contract value.
fn length(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Returns whether a platform length exceeds one explicit bound.
fn exceeds(value: usize, limit: u64) -> bool {
    length(value) > limit
}

/// Checks the inert logical source-ID boundary.
fn valid_source_id(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

/// Checks the captured envelope's qualified snake-case spelling, before source graph validation.
/// The graph contract additionally requires a letter after each underscore and excludes keywords.
fn valid_module_id(value: &str) -> bool {
    !value.is_empty() && value.split("::").all(is_snake_name)
}

/// Checks the canonical vocabulary identity spelling during capture.
fn valid_vocabulary_identity(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
