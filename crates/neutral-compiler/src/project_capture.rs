// SPDX-License-Identifier: Apache-2.0

//! Complete, bounded, and effect-free project capture for the v1 profile.

use neutral_core::{
    CancellationToken, SourceContentDigest, VocabularyContentDigest, profile::LanguageProfile,
};
use neutral_vocabulary::VocabularyLock;
use std::{collections::BTreeSet, sync::Arc};

/// Exact version accepted by the captured-project request boundary.
pub const CAPTURE_REQUEST_VERSION: &str = "neutral.capture/v1";

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
    /// Imports retained for one module once import capture is active.
    pub imports_per_module: u64,
    /// Total project import edges once import capture is active.
    pub import_edges: u64,
    /// Modules retained in one SCC once graph capture is active.
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
#[derive(Clone, Debug)]
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
    #[must_use]
    pub const fn requiring_digest(mut self, digest: SourceContentDigest) -> Self {
        self.expected_digest = Some(digest);
        self
    }
}

/// One exact host-supplied vocabulary bundle and immutable semantic lock.
#[derive(Clone, Debug)]
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
    source_id: Arc<str>,
    /// Qualified logical module ID.
    module_id: Arc<str>,
    /// Exact immutable source bytes.
    bytes: Arc<[u8]>,
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
    bytes: Arc<[u8]>,
    /// Exact immutable semantic lock.
    lock: VocabularyLock,
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
pub struct CapturedProject {
    /// Exact project profile.
    profile: LanguageProfile,
    /// All supplied sources in `(module ID, source ID)` order.
    sources: Arc<[CapturedProjectSource]>,
    /// All exact vocabularies in canonical-identity order.
    vocabularies: Arc<[CapturedProjectVocabulary]>,
    /// Complete deterministic limits, excluding mutable cancellation state.
    limits: ProjectCaptureLimits,
}

impl CapturedProject {
    /// Returns the exact captured language profile.
    #[must_use]
    pub const fn profile(&self) -> LanguageProfile {
        self.profile
    }

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

/// Captures a complete supplied project without resolution or ambient I/O.
///
/// Every byte sequence and lock must already be present in `request`. The
/// request type has no resolver, callback, path, URL, or root field.
///
/// # Errors
///
/// Returns one stable fail-closed error and never exposes a partial project.
pub fn capture_project(
    request: CapturedProjectRequest,
) -> Result<CapturedProject, ProjectCaptureError> {
    capture_project_with_checkpoints(request, |_| {})
}

/// Deterministic capture handoffs used to prove cancellation behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectCaptureCheckpoint {
    /// Before request validation or traversal.
    Start,
    /// After all exact source and vocabulary integrity checks.
    Integrity,
    /// After every source header has been validated.
    Headers,
    /// Immediately before immutable output publication.
    Publish,
}

/// Captures a project while exposing deterministic handoffs to private tests.
fn capture_project_with_checkpoints(
    request: CapturedProjectRequest,
    mut checkpoint: impl FnMut(ProjectCaptureCheckpoint),
) -> Result<CapturedProject, ProjectCaptureError> {
    checkpoint(ProjectCaptureCheckpoint::Start);
    if request.controls.cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled);
    }
    validate_envelope(&request)?;
    let values = request.controls.limits.values();
    let mut captured_sources = validate_sources(request.sources, values)?;
    let vocabulary_ids = validate_vocabularies(&request.vocabularies, values)?;
    checkpoint(ProjectCaptureCheckpoint::Integrity);
    if request.controls.cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled);
    }
    let required_vocabularies = validate_headers(&captured_sources, request.profile)?;
    checkpoint(ProjectCaptureCheckpoint::Headers);
    if request.controls.cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled);
    }
    validate_vocabulary_cover(&required_vocabularies, &vocabulary_ids)?;
    checkpoint(ProjectCaptureCheckpoint::Publish);
    if request.controls.cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled);
    }
    freeze_project(
        &request.controls.cancellation,
        request.profile,
        request.controls.limits,
        &mut captured_sources,
        request.vocabularies,
    )
}

/// Validates closed-envelope fields and collection bounds before traversal.
fn validate_envelope(request: &CapturedProjectRequest) -> Result<(), ProjectCaptureError> {
    if request.request_version != CAPTURE_REQUEST_VERSION {
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

/// Validates source identities, independent byte bounds, and exact digests.
fn validate_sources(
    sources: Vec<CapturedSourceInput>,
    values: ProjectCaptureLimitValues,
) -> Result<Vec<(CapturedSourceInput, SourceContentDigest)>, ProjectCaptureError> {
    let mut source_ids = BTreeSet::new();
    let mut module_ids = BTreeSet::new();
    let mut total_source_bytes = 0_u64;
    let mut captured_sources = Vec::with_capacity(sources.len());
    for source in sources {
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
        if !source_ids.insert(source.source_id.clone()) {
            return Err(ProjectCaptureError::DuplicateSourceId);
        }
        if !module_ids.insert(source.module_id.clone()) {
            return Err(ProjectCaptureError::DuplicateModuleId);
        }
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

/// Validates vocabulary identity uniqueness, byte bounds, and exact locks.
fn validate_vocabularies(
    vocabularies: &[CapturedVocabularyInput],
    values: ProjectCaptureLimitValues,
) -> Result<BTreeSet<String>, ProjectCaptureError> {
    let mut vocabulary_ids = BTreeSet::new();
    let mut total_vocabulary_bytes = 0_u64;
    for vocabulary in vocabularies {
        if exceeds(vocabulary.bytes.len(), values.vocabulary_bytes_per_unit) {
            return Err(ProjectCaptureError::LimitExceeded);
        }
        total_vocabulary_bytes = total_vocabulary_bytes
            .checked_add(length(vocabulary.bytes.len()))
            .ok_or(ProjectCaptureError::LimitExceeded)?;
        if total_vocabulary_bytes > values.total_vocabulary_bytes {
            return Err(ProjectCaptureError::LimitExceeded);
        }
        if !vocabulary_ids.insert(vocabulary.lock.identity().to_owned()) {
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

/// Validates request/header agreement and returns the complete required lock set.
fn validate_headers(
    sources: &[(CapturedSourceInput, SourceContentDigest)],
    profile: LanguageProfile,
) -> Result<BTreeSet<String>, ProjectCaptureError> {
    let mut required_vocabularies = BTreeSet::new();
    for (source, _) in sources {
        let headers = scan_headers(&source.bytes)?;
        if headers.profile != profile.source_version() {
            return Err(ProjectCaptureError::ProfileMismatch);
        }
        if headers.module_id != source.module_id {
            return Err(ProjectCaptureError::ModuleHeaderMismatch);
        }
        required_vocabularies.extend(headers.vocabularies);
    }
    Ok(required_vocabularies)
}

/// Requires supplied locks to be exactly the set referenced by source headers.
fn validate_vocabulary_cover(
    required: &BTreeSet<String>,
    supplied: &BTreeSet<String>,
) -> Result<(), ProjectCaptureError> {
    if required.difference(supplied).next().is_some() {
        return Err(ProjectCaptureError::MissingVocabulary);
    }
    if supplied.difference(required).next().is_some() {
        return Err(ProjectCaptureError::ExtraVocabulary);
    }
    Ok(())
}

/// Canonically orders and freezes a completely validated request.
fn freeze_project(
    cancellation: &CancellationToken,
    profile: LanguageProfile,
    limits: ProjectCaptureLimits,
    sources: &mut Vec<(CapturedSourceInput, SourceContentDigest)>,
    mut vocabularies: Vec<CapturedVocabularyInput>,
) -> Result<CapturedProject, ProjectCaptureError> {
    sources.sort_by(|(left, _), (right, _)| {
        (&left.module_id, &left.source_id).cmp(&(&right.module_id, &right.source_id))
    });
    let sources: Vec<_> = std::mem::take(sources)
        .into_iter()
        .map(|(source, digest)| CapturedProjectSource {
            source_id: Arc::from(source.source_id),
            module_id: Arc::from(source.module_id),
            bytes: Arc::from(source.bytes),
            digest,
        })
        .collect();
    vocabularies.sort_by(|left, right| left.lock.identity().cmp(right.lock.identity()));
    let vocabularies: Vec<_> = vocabularies
        .into_iter()
        .map(|vocabulary| CapturedProjectVocabulary {
            bytes: Arc::from(vocabulary.bytes),
            lock: vocabulary.lock,
        })
        .collect();
    if cancellation.is_cancelled() {
        return Err(ProjectCaptureError::Cancelled);
    }
    Ok(CapturedProject {
        profile,
        sources: Arc::from(sources),
        vocabularies: Arc::from(vocabularies),
        limits,
    })
}

/// Minimal bounded header facts required by Stage 2 capture.
struct HeaderFacts<'a> {
    /// Exact unescaped profile text.
    profile: &'a str,
    /// Exact qualified module ID.
    module_id: &'a str,
    /// Exact required vocabulary identities.
    vocabularies: BTreeSet<String>,
}

/// Scans only required header lines without invoking the inactive v1 parser.
fn scan_headers(bytes: &[u8]) -> Result<HeaderFacts<'_>, ProjectCaptureError> {
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
    let mut vocabularies = BTreeSet::new();
    for line in lines {
        if line.starts_with("neu ") || line.starts_with("module ") {
            return Err(ProjectCaptureError::InvalidHeader);
        }
        let Some(requirement) = line.strip_prefix("use ") else {
            continue;
        };
        let mut words = requirement.split_ascii_whitespace();
        let identity = words.next().ok_or(ProjectCaptureError::InvalidHeader)?;
        let as_keyword = words.next().ok_or(ProjectCaptureError::InvalidHeader)?;
        let alias = words.next().ok_or(ProjectCaptureError::InvalidHeader)?;
        if words.next().is_some()
            || as_keyword != "as"
            || !valid_vocabulary_identity(identity)
            || !valid_name_segment(alias)
            || !vocabularies.insert(identity.to_owned())
        {
            return Err(ProjectCaptureError::InvalidHeader);
        }
    }
    Ok(HeaderFacts {
        profile,
        module_id,
        vocabularies,
    })
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

/// Checks one exact qualified snake-case logical module ID.
fn valid_module_id(value: &str) -> bool {
    !value.is_empty() && value.split("::").all(valid_name_segment)
}

/// Checks one lowercase snake-case name segment.
fn valid_name_segment(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !value.ends_with('_')
        && !value.contains("__")
}

/// Checks the Stage 2 canonical vocabulary identity spelling.
fn valid_vocabulary_identity(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
#[path = "../tests/project_capture/mod.rs"]
mod tests;
