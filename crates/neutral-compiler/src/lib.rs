// SPDX-License-Identifier: Apache-2.0

//! Neutral compilation pipeline boundary.
//!
//! This crate owns capture contracts, the private frontend and semantic model,
//! and lowering into public logical IR. Its pure captured-input compilation
//! path must not use filesystem, process, environment, network, locale, or clock
//! authority. Stage 2 established captured-input contracts; Stages 3 through
//! 6.2 extend the private frontend while preserving the same effect-free boundary.

use neutral_core::{
    ByteSpan, CancellationToken, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity,
    ResultClass, SourceContentDigest, SourceLocation, StructuralLimits,
};
use neutral_ir::CompilationArtifacts;
use neutral_vocabulary::{
    VocabularyError, VocabularyLimits, VocabularyLock, validate_captured_bundle,
};
use std::sync::Arc;

mod frontend;
mod language;
mod semantics;

/// Stable diagnostic identifiers emitted by compiler validation.
pub mod diagnostics {
    /// A source vocabulary requirement had no exact captured input.
    pub const MISSING_VOCABULARY: &str = "NEU-VOC-001";
    /// Captured bytes or identity facts disagreed with the exact lock.
    pub const VOCABULARY_LOCK_MISMATCH: &str = "NEU-VOC-002";
    /// A captured bundle violated its closed data-only schema.
    pub const INVALID_VOCABULARY_BUNDLE: &str = "NEU-VOC-003";
    /// A captured bundle required an unknown structural feature.
    pub const UNKNOWN_VOCABULARY_FEATURE: &str = "NEU-VOC-004";
    /// A qualified source type was absent from the captured contract.
    pub const UNKNOWN_VOCABULARY_TYPE: &str = "NEU-VOC-005";
    /// Captured bundle content attempted to introduce executable behavior.
    pub const EXECUTABLE_VOCABULARY_MEMBER: &str = "NEU-VOC-006";
    /// A vocabulary payload contained an unknown field.
    pub const UNKNOWN_VOCABULARY_FIELD: &str = "NEU-VOC-007";
    /// A vocabulary payload omitted a required field.
    pub const MISSING_VOCABULARY_FIELD: &str = "NEU-VOC-008";
    /// A vocabulary payload repeated one field.
    pub const DUPLICATE_VOCABULARY_FIELD: &str = "NEU-VOC-009";
    /// A vocabulary payload field did not satisfy its captured type.
    pub const VOCABULARY_FIELD_TYPE_MISMATCH: &str = "NEU-VOC-010";
    /// A vocabulary namespace collided with a root declaration.
    pub const VOCABULARY_NAME_COLLISION: &str = "NEU-VOC-011";
    /// Missing module header diagnostic.
    pub const MISSING_MODULE_HEADER: &str = "NEU-SYN-001";
    /// Unsupported language version diagnostic.
    pub const UNSUPPORTED_LANGUAGE_VERSION: &str = "NEU-SYN-002";
    /// Recognized but unavailable language profile diagnostic.
    pub use neutral_core::profile::PROFILE_UNAVAILABLE_DIAGNOSTIC;
    /// Malformed lexical or layout boundary diagnostic.
    pub const MALFORMED_BOUNDARY: &str = "NEU-SYN-003";
    /// Unsupported source symbol diagnostic.
    pub const UNSUPPORTED_SYMBOL: &str = "NEU-LEX-001";
    /// Unterminated block comment diagnostic.
    pub const UNTERMINATED_BLOCK_COMMENT: &str = "NEU-LEX-002";
    /// Invalid string literal diagnostic.
    pub const INVALID_STRING_LITERAL: &str = "NEU-LEX-003";
    /// Unterminated string literal diagnostic.
    pub const UNTERMINATED_STRING_LITERAL: &str = "NEU-LEX-004";
    /// Invalid identifier name diagnostic.
    pub const INVALID_NAME: &str = "NEU-NAME-001";
    /// Protected core name diagnostic.
    pub const PROTECTED_NAME: &str = "NEU-NAME-002";
    /// Duplicate root declaration diagnostic.
    pub const DUPLICATE_DECLARATION: &str = "NEU-NAME-003";
    /// Duplicate record schema field diagnostic.
    pub const DUPLICATE_RECORD_FIELD: &str = "NEU-NAME-004";
    /// Invalid exact numeric value diagnostic.
    pub const INVALID_NUMBER: &str = "NEU-VAL-001";
    /// Exact numeric resource-limit diagnostic.
    pub const NUMBER_LIMIT_EXCEEDED: &str = "NEU-LIM-002";
    /// Explicit scalar type/value mismatch diagnostic.
    pub const TYPE_MISMATCH: &str = "NEU-TYP-001";
    /// Unknown nominal type diagnostic.
    pub const UNKNOWN_TYPE: &str = "NEU-TYP-002";
    /// A declaration name was used with the wrong kind.
    pub const WRONG_DECLARATION_KIND: &str = "NEU-TYP-003";
    /// Embedded nominal record recursion diagnostic.
    pub const EMBEDDED_RECORD_RECURSION: &str = "NEU-TYP-004";
    /// Missing required contextual-record field diagnostic.
    pub const MISSING_RECORD_FIELD: &str = "NEU-VAL-002";
    /// Unknown contextual-record field diagnostic.
    pub const UNKNOWN_RECORD_FIELD: &str = "NEU-VAL-003";
    /// Duplicate contextual-record value field diagnostic.
    pub const DUPLICATE_VALUE_FIELD: &str = "NEU-VAL-004";
    /// A user-record default was not a closed constant.
    pub const NON_CONSTANT_DEFAULT: &str = "NEU-VAL-005";
    /// An ordinary immutable value name did not resolve to a declaration.
    pub const UNKNOWN_VALUE: &str = "NEU-VAL-006";
    /// Ordinary immutable-value dependencies formed a cycle.
    pub const VALUE_CYCLE: &str = "NEU-VAL-007";
    /// An identity-reference target name did not resolve to a declaration.
    pub const UNKNOWN_REFERENCE_TARGET: &str = "NEU-REF-001";
    /// An identity reference targeted a non-binding declaration.
    pub const WRONG_REFERENCE_TARGET_KIND: &str = "NEU-REF-002";
    /// An identity-reference target binding had a non-exact resolved type.
    pub const REFERENCE_TARGET_TYPE_MISMATCH: &str = "NEU-REF-003";
    /// Decoded string resource-limit diagnostic.
    pub const STRING_LIMIT_EXCEEDED: &str = "NEU-LIM-001";
    /// Record structure resource-limit diagnostic.
    pub const RECORD_LIMIT_EXCEEDED: &str = "NEU-LIM-003";
    /// List item, nesting, or traversal resource-limit diagnostic.
    pub const LIST_LIMIT_EXCEEDED: &str = "NEU-LIM-004";
    /// Immutable-value dependency traversal exceeded the captured bound.
    pub const VALUE_TRAVERSAL_LIMIT_EXCEEDED: &str = "NEU-LIM-005";
}

/// The frozen v0 language-behavior contract used for captured compilation.
pub use neutral_ir::LANGUAGE_BEHAVIOR_VERSION;

/// Host-supplied input for one prospective Neutral source unit.
#[derive(Clone, Debug)]
pub struct CompilationRequest {
    /// Exact source bytes supplied by the host.
    source: Vec<u8>,
    /// Optional exact identity required by the caller.
    expected_source_digest: Option<SourceContentDigest>,
    /// Deterministic limits captured for this request.
    limits: StructuralLimits,
    /// Cooperative cancellation signal supplied by the caller.
    cancellation: CancellationToken,
    /// Optional exact bundle bytes and lock supplied by the host.
    vocabulary: Option<CapturedVocabularyInput>,
}

impl CompilationRequest {
    /// Creates a request for exactly one host-supplied source byte sequence.
    #[must_use]
    pub fn new(source: Vec<u8>, limits: StructuralLimits, cancellation: CancellationToken) -> Self {
        Self {
            source,
            expected_source_digest: None,
            limits,
            cancellation,
            vocabulary: None,
        }
    }

    /// Requires the exact digest that capture must observe before compiling.
    #[must_use]
    pub fn requiring_source_digest(mut self, expected: SourceContentDigest) -> Self {
        self.expected_source_digest = Some(expected);
        self
    }

    /// Supplies one exact already-captured vocabulary bundle and lock.
    #[must_use]
    pub fn with_captured_vocabulary(mut self, bytes: Vec<u8>, lock: VocabularyLock) -> Self {
        self.vocabulary = Some(CapturedVocabularyInput {
            bytes: Arc::from(bytes),
            lock,
        });
        self
    }
}

/// Exact host-supplied vocabulary input with no acquisition authority.
#[derive(Clone, Debug)]
struct CapturedVocabularyInput {
    /// Exact captured bundle bytes.
    bytes: Arc<[u8]>,
    /// Exact immutable lock facts.
    lock: VocabularyLock,
}

/// Immutable, replayable input for the I/O-free compilation boundary.
#[derive(Clone, Debug)]
pub struct CapturedCompilation {
    /// Immutable exact source bytes with no host path or credential metadata.
    source: Arc<[u8]>,
    /// Typed identity of the captured exact source bytes.
    source_digest: SourceContentDigest,
    /// Deterministic limits preserved from the request.
    limits: StructuralLimits,
    /// Cooperative cancellation signal preserved from the request.
    cancellation: CancellationToken,
    /// Optional exact captured vocabulary bytes and lock.
    vocabulary: Option<CapturedVocabularyInput>,
}

impl CapturedCompilation {
    /// Returns the exact original source bytes without host metadata or paths.
    #[must_use]
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    /// Returns the exact-byte source content identity.
    #[must_use]
    pub const fn source_digest(&self) -> SourceContentDigest {
        self.source_digest
    }

    /// Returns the frozen language-behavior version for this capture.
    #[must_use]
    pub const fn language_behavior_version(&self) -> &'static str {
        LANGUAGE_BEHAVIOR_VERSION
    }

    /// Returns deterministic resource limits captured with this input.
    #[must_use]
    pub const fn limits(&self) -> StructuralLimits {
        self.limits
    }

    /// Returns whether the host supplied one exact vocabulary capture.
    #[must_use]
    pub const fn has_captured_vocabulary(&self) -> bool {
        self.vocabulary.is_some()
    }
}

/// A fail-closed error while capturing host-supplied input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureError {
    /// No source bytes were supplied for the required single source unit.
    MissingSource,
    /// Source exceeded the captured deterministic byte limit.
    SourceLimitExceeded {
        /// The supplied source byte count.
        actual: u64,
        /// The captured maximum source byte count.
        limit: u64,
    },
    /// An expected exact source digest did not match the supplied source bytes.
    SourceDigestMismatch {
        /// The exact source identity required by the host request.
        expected: SourceContentDigest,
        /// The identity computed from the supplied source bytes.
        actual: SourceContentDigest,
    },
    /// The caller cancelled before capture completed.
    Cancelled,
}

/// Canonical source bytes produced by the reference formatter.
///
/// These bytes have their own ordinary source identity if captured again. They
/// are not logical IR, canonical artifact bytes, or signing material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormattedSource {
    /// Complete UTF-8 formatter output with one final line feed.
    bytes: Vec<u8>,
}

impl FormattedSource {
    /// Returns the complete formatted source bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the result and returns its complete formatted source bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A fail-closed error from source capture, validation, or formatting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatError {
    /// The host-supplied request could not be captured.
    Capture(CaptureError),
    /// The captured document did not compile to valid logical IR.
    Compilation(CompilationFailure),
    /// Canonical output exceeded the captured source-byte ceiling.
    OutputLimitExceeded,
    /// Valid compilation and private parsing disagreed unexpectedly.
    InternalDefect,
}

/// A non-authoritative result of the current compilation boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationResult {
    /// Compilation produced authoritative validated in-process artifacts.
    Success(Arc<CompilationArtifacts>),
    /// Compilation did not produce authoritative IR.
    Failure(CompilationFailure),
}

/// A failure that guarantees authoritative IR is absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationFailure {
    /// Stable broad classification for the failure.
    class: ResultClass,
    /// Bounded reason that exposes no authoritative IR.
    detail: CompilationFailureDetail,
    /// Stable, bounded diagnostics produced before authoritative output.
    diagnostics: Vec<Diagnostic>,
}

impl CompilationFailure {
    /// Returns the stable class used by callers to handle the failure.
    #[must_use]
    pub const fn class(&self) -> ResultClass {
        self.class
    }

    /// Returns bounded detail without exposing host acquisition information.
    #[must_use]
    pub const fn detail(&self) -> CompilationFailureDetail {
        self.detail
    }

    /// Returns stable diagnostics without exposing compiler-private syntax models.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// The specific non-authoritative reason for a compilation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompilationFailureDetail {
    /// The caller cancelled before frontend work could begin.
    Cancelled,
    /// The source is outside the currently implemented frontend slice.
    FrontendUnavailable,
    /// Syntax was rejected, potentially with a frozen diagnostic.
    SyntaxRejected,
    /// Parsed source was rejected by semantic validation.
    SemanticRejected,
    /// A typed identity-reference target was rejected.
    ReferenceRejected,
    /// A deterministic captured structural limit was exceeded.
    ResourceLimitExceeded,
    /// Captured vocabulary resolution, validation, or payload was rejected.
    VocabularyRejected,
}

/// Captures exact host-supplied input without consulting ambient authority.
///
/// # Errors
///
/// Returns a fail-closed error for empty input, cancellation, an exceeded byte
/// limit, or an expected-digest mismatch.
pub fn capture(request: CompilationRequest) -> Result<CapturedCompilation, CaptureError> {
    if request.cancellation.is_cancelled() {
        return Err(CaptureError::Cancelled);
    }
    if request.source.is_empty() {
        return Err(CaptureError::MissingSource);
    }
    let source_length = u64::try_from(request.source.len()).unwrap_or(u64::MAX);
    if source_length > request.limits.source_bytes() {
        return Err(CaptureError::SourceLimitExceeded {
            actual: source_length,
            limit: request.limits.source_bytes(),
        });
    }
    let source_digest = SourceContentDigest::from_bytes(&request.source);
    if let Some(expected) = request.expected_source_digest
        && expected != source_digest
    {
        return Err(CaptureError::SourceDigestMismatch {
            expected,
            actual: source_digest,
        });
    }
    Ok(CapturedCompilation {
        source: Arc::from(request.source),
        source_digest,
        limits: request.limits,
        cancellation: request.cancellation,
        vocabulary: request.vocabulary,
    })
}

/// Compiles immutable captured input without any host acquisition or ambient I/O.
///
/// Any optional vocabulary bytes and lock were supplied by the host during
/// capture; this boundary performs no registry, path, cache, or network lookup.
#[must_use]
pub fn compile_captured(captured: &CapturedCompilation) -> CompilationResult {
    compile_captured_with_checkpoints(captured, |_| {})
}

/// Internal compilation handoffs at which cooperative cancellation is observed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompilationCheckpoint {
    /// Exact source and optional vocabulary input have been captured.
    Capture,
    /// The optional vocabulary contract has been validated.
    Vocabulary,
    /// Source decoding, lexing, layout, and parsing have completed.
    Frontend,
    /// Semantic validation and lowering have completed before publication.
    Semantics,
}

/// Compiles captured input while exposing deterministic handoffs to private tests.
fn compile_captured_with_checkpoints(
    captured: &CapturedCompilation,
    mut checkpoint: impl FnMut(CompilationCheckpoint),
) -> CompilationResult {
    checkpoint(CompilationCheckpoint::Capture);
    if captured.cancellation.is_cancelled() {
        return cancellation_failure();
    }

    let vocabulary = match captured.vocabulary.as_ref() {
        Some(input) => match validate_captured_bundle(
            &input.bytes,
            &input.lock,
            VocabularyLimits::from_structural(captured.limits()),
        ) {
            Ok(bundle) => Some(bundle),
            Err(error) => return vocabulary_bundle_failure(captured.source_digest(), error),
        },
        None => None,
    };
    checkpoint(CompilationCheckpoint::Vocabulary);
    if captured.cancellation.is_cancelled() {
        return cancellation_failure();
    }
    match frontend::parse(captured.source(), captured.limits()) {
        Ok(unit) => {
            checkpoint(CompilationCheckpoint::Frontend);
            if captured.cancellation.is_cancelled() {
                return cancellation_failure();
            }
            match semantics::lower(
                unit,
                vocabulary.as_ref(),
                captured.source_digest(),
                captured.source().len(),
                captured.limits(),
            ) {
                Ok(artifacts) => {
                    checkpoint(CompilationCheckpoint::Semantics);
                    if captured.cancellation.is_cancelled() {
                        cancellation_failure()
                    } else {
                        CompilationResult::Success(Arc::new(artifacts))
                    }
                }
                Err(error) => {
                    CompilationResult::Failure(error.into_failure(captured.source_digest()))
                }
            }
        }
        Err(error) => CompilationResult::Failure(CompilationFailure {
            class: error.class(),
            detail: error.detail(),
            diagnostics: error.into_diagnostics(captured.source_digest()),
        }),
    }
}

/// Returns the uniform fail-closed compilation cancellation result.
fn cancellation_failure() -> CompilationResult {
    CompilationResult::Failure(CompilationFailure {
        class: ResultClass::Cancellation,
        detail: CompilationFailureDetail::Cancelled,
        diagnostics: Vec::new(),
    })
}

/// Converts one strict captured-bundle failure into stable bounded diagnostics.
fn vocabulary_bundle_failure(
    source: SourceContentDigest,
    error: VocabularyError,
) -> CompilationResult {
    let code = match error {
        VocabularyError::DigestMismatch
        | VocabularyError::LockMismatch
        | VocabularyError::UnsupportedEncodingVersion
        | VocabularyError::UnsupportedSchemaVersion => diagnostics::VOCABULARY_LOCK_MISMATCH,
        VocabularyError::UnknownRequiredFeature => diagnostics::UNKNOWN_VOCABULARY_FEATURE,
        VocabularyError::ExecutableShapeForbidden => diagnostics::EXECUTABLE_VOCABULARY_MEMBER,
        _ => diagnostics::INVALID_VOCABULARY_BUNDLE,
    };
    let diagnostic = Diagnostic::new(
        DiagnosticCode::new(code).expect("vocabulary diagnostic code must be ASCII"),
        DiagnosticLayer::Vocabulary,
        DiagnosticSeverity::Error,
        SourceLocation::new(
            source,
            ByteSpan::new(0, 0).expect("zero span must be valid"),
        ),
        Vec::new(),
        false,
    );
    CompilationResult::Failure(CompilationFailure {
        class: ResultClass::Vocabulary,
        detail: CompilationFailureDetail::VocabularyRejected,
        diagnostics: vec![diagnostic],
    })
}

/// Captures then compiles one host-supplied request without exposing partial IR.
///
/// # Errors
///
/// Returns a capture error before compilation when the request cannot be frozen.
pub fn compile(request: CompilationRequest) -> Result<CompilationResult, CaptureError> {
    capture(request).map(|captured| compile_captured(&captured))
}

/// Captures, validates, and canonically formats one host-supplied source unit.
///
/// # Errors
///
/// Returns a fail-closed capture, compilation, output-limit, or internal error.
pub fn format(request: CompilationRequest) -> Result<FormattedSource, FormatError> {
    let captured = capture(request).map_err(FormatError::Capture)?;
    format_captured(&captured)
}

/// Canonically formats one immutable captured source without ambient I/O.
///
/// The source must compile successfully before formatter output is published.
/// Formatted bytes remain separate from both the captured input identity and
/// logical artifact identity.
///
/// # Errors
///
/// Returns the complete compilation failure, an output-limit failure, or an
/// internal defect if a valid compilation cannot be parsed a second time.
pub fn format_captured(captured: &CapturedCompilation) -> Result<FormattedSource, FormatError> {
    if let CompilationResult::Failure(failure) = compile_captured(captured) {
        return Err(FormatError::Compilation(failure));
    }
    let unit = frontend::parse(captured.source(), captured.limits())
        .map_err(|_| FormatError::InternalDefect)?;
    let bytes = frontend::format_source(captured.source(), &unit);
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > captured.limits().source_bytes() {
        return Err(FormatError::OutputLimitExceeded);
    }
    Ok(FormattedSource { bytes })
}

#[cfg(test)]
#[path = "../tests/internal_unit/mod.rs"]
mod tests;
