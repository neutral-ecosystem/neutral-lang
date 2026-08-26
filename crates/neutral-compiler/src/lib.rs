// SPDX-License-Identifier: Apache-2.0

//! Neutral compilation pipeline boundary.
//!
//! This crate owns capture contracts, the private frontend and semantic model,
//! and lowering into public logical IR. Its pure captured-input compilation
//! path must not use filesystem, process, environment, network, locale, or clock
//! authority. Stage 2 establishes captured-input contracts before frontend
//! acceptance behavior is implemented.

use neutral_core::{CancellationToken, ResultClass, SourceContentDigest, StructuralLimits};
use std::sync::Arc;

/// The frozen v0 language-behavior contract used for captured compilation.
pub const LANGUAGE_BEHAVIOR_VERSION: &str = "0.1.0";

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
        }
    }

    /// Requires the exact digest that capture must observe before compiling.
    #[must_use]
    pub fn requiring_source_digest(mut self, expected: SourceContentDigest) -> Self {
        self.expected_source_digest = Some(expected);
        self
    }
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

/// A non-authoritative result of the current compilation boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationResult {
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
}

/// The specific non-authoritative reason for a compilation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompilationFailureDetail {
    /// The caller cancelled before frontend work could begin.
    Cancelled,
    /// Stage 2 frontend acceptance is not active until the next implementation step.
    FrontendUnavailable,
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
    })
}

/// Compiles immutable captured input without any host acquisition or ambient I/O.
///
/// Stage 2 Step 2 establishes this boundary but intentionally does not accept
/// source until the minimal frontend slice is implemented.
#[must_use]
pub fn compile_captured(captured: &CapturedCompilation) -> CompilationResult {
    let detail = if captured.cancellation.is_cancelled() {
        CompilationFailureDetail::Cancelled
    } else {
        CompilationFailureDetail::FrontendUnavailable
    };
    let class = if detail == CompilationFailureDetail::Cancelled {
        ResultClass::Cancellation
    } else {
        ResultClass::Internal
    };
    CompilationResult::Failure(CompilationFailure { class, detail })
}

/// Captures then compiles one host-supplied request without exposing partial IR.
///
/// # Errors
///
/// Returns a capture error before compilation when the request cannot be frozen.
pub fn compile(request: CompilationRequest) -> Result<CompilationResult, CaptureError> {
    capture(request).map(|captured| compile_captured(&captured))
}

#[cfg(test)]
/// Unit tests for captured-input and I/O-free compiler contracts.
mod tests {
    use super::{
        CaptureError, CompilationFailureDetail, CompilationRequest, capture, compile_captured,
    };
    use neutral_core::{CancellationToken, SourceContentDigest, StructuralLimits};

    /// Returns small deterministic limits for capture tests.
    fn test_limits() -> StructuralLimits {
        StructuralLimits::new(64, 4).expect("test limits should be valid")
    }

    #[test]
    /// Verifies that capture records exact source bytes and their typed digest.
    fn capture_records_exact_source_identity() {
        let source = b"neu \"0.1\"\n".to_vec();
        let captured = capture(CompilationRequest::new(
            source.clone(),
            test_limits(),
            CancellationToken::new(),
        ))
        .expect("source should capture");
        assert_eq!(captured.source(), source);
        assert_eq!(
            captured.source_digest(),
            SourceContentDigest::from_bytes(&source)
        );
    }

    #[test]
    /// Verifies that digest mismatches fail before compiler work starts.
    fn capture_rejects_an_unmatched_digest() {
        let request =
            CompilationRequest::new(b"source".to_vec(), test_limits(), CancellationToken::new())
                .requiring_source_digest(SourceContentDigest::from_bytes(b"other"));
        assert!(matches!(
            capture(request),
            Err(CaptureError::SourceDigestMismatch { .. })
        ));
    }

    #[test]
    /// Verifies that the pre-frontend boundary never exposes authoritative IR.
    fn compile_captured_has_no_authoritative_result_before_the_frontend_slice() {
        let captured = capture(CompilationRequest::new(
            b"source".to_vec(),
            test_limits(),
            CancellationToken::new(),
        ))
        .expect("source should capture");
        assert!(matches!(
            compile_captured(&captured),
            super::CompilationResult::Failure(super::CompilationFailure {
                detail: CompilationFailureDetail::FrontendUnavailable,
                ..
            })
        ));
    }
}
