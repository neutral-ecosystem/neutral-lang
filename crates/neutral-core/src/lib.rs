// SPDX-License-Identifier: Apache-2.0

//! Foundational contracts shared across the Neutral implementation.
//!
//! This crate owns source identity, source spans, diagnostics, resource limits,
//! cancellation, and result classification. It must remain independent of the
//! compiler, reader, command-line hosts, and ambient host services.

use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering as CompareOrdering,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// A typed SHA-256 digest of exact captured source bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceContentDigest([u8; 32]);

impl SourceContentDigest {
    /// Computes the digest over exactly `bytes`, without normalization.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bytes: [u8; 32] = Sha256::digest(bytes).into();
        Self(bytes)
    }

    /// Returns the raw SHA-256 bytes for integrity comparison or storage.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// A typed SHA-256 digest produced by a Neutral Hash Transcript v1 domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticDigest([u8; 32]);

impl SemanticDigest {
    /// Hashes `payload` using the frozen NHT-v1 framing and supplied ASCII domain.
    ///
    /// # Errors
    ///
    /// Returns an error when a tag is non-ASCII or a framed length cannot be
    /// represented by the frozen transcript integer widths.
    pub fn from_nht(domain: &str, payload: &[u8]) -> Result<Self, CoreError> {
        let domain = nht_frame(domain, payload)?;
        let transcript = nht_frame("neutral-nht-v1", &domain)?;
        let bytes: [u8; 32] = Sha256::digest(transcript).into();
        Ok(Self(bytes))
    }

    /// Returns the raw SHA-256 bytes for integrity comparison or storage.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for SemanticDigest {
    /// Formats the semantic digest using lower hexadecimal text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Frames one NHT-v1 tagged payload using unsigned big-endian lengths.
///
/// # Errors
///
/// Returns an error for non-ASCII tags or lengths outside the frozen widths.
pub fn nht_frame(tag: &str, payload: &[u8]) -> Result<Vec<u8>, CoreError> {
    if !tag.is_ascii() {
        return Err(CoreError::InvalidTranscriptTag);
    }
    let tag_length = u16::try_from(tag.len()).map_err(|_| CoreError::TranscriptLengthExceeded)?;
    let payload_length =
        u64::try_from(payload.len()).map_err(|_| CoreError::TranscriptLengthExceeded)?;
    let capacity = 2_usize
        .checked_add(tag.len())
        .and_then(|length| length.checked_add(8))
        .and_then(|length| length.checked_add(payload.len()))
        .ok_or(CoreError::TranscriptLengthExceeded)?;
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(&tag_length.to_be_bytes());
    frame.extend_from_slice(tag.as_bytes());
    frame.extend_from_slice(&payload_length.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

impl fmt::Display for SourceContentDigest {
    /// Formats the digest using the frozen `sha256:` lower-hex text form.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sha256:")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// A checked half-open byte range within one original source byte sequence.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteSpan {
    /// The first included original-byte offset.
    start: u64,
    /// The first excluded original-byte offset.
    end: u64,
}

impl ByteSpan {
    /// Creates a half-open span when `start` does not follow `end`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvertedSpan`] when `start` follows `end`.
    pub const fn new(start: u64, end: u64) -> Result<Self, CoreError> {
        if start > end {
            return Err(CoreError::InvertedSpan { start, end });
        }
        Ok(Self { start, end })
    }

    /// Returns the first included original-byte offset.
    #[must_use]
    pub const fn start(self) -> u64 {
        self.start
    }

    /// Returns the first excluded original-byte offset.
    #[must_use]
    pub const fn end(self) -> u64 {
        self.end
    }

    /// Returns the number of bytes in this span.
    #[must_use]
    pub const fn len(self) -> u64 {
        self.end - self.start
    }

    /// Returns whether this span contains no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// A source location tied to one exact captured source identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceLocation {
    /// The exact captured source identity.
    source: SourceContentDigest,
    /// The half-open original-byte range.
    span: ByteSpan,
}

impl SourceLocation {
    /// Creates a location for `span` in the source identified by `source`.
    #[must_use]
    pub const fn new(source: SourceContentDigest, span: ByteSpan) -> Self {
        Self { source, span }
    }

    /// Returns the exact source content identity.
    #[must_use]
    pub const fn source(self) -> SourceContentDigest {
        self.source
    }

    /// Returns the original-byte span.
    #[must_use]
    pub const fn span(self) -> ByteSpan {
        self.span
    }
}

/// A one-based line and original-byte column within captured source bytes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LineColumn {
    /// The one-based logical line number.
    line: u64,
    /// The one-based original-byte column number.
    column: u64,
}

impl LineColumn {
    /// Returns the one-based logical line number.
    #[must_use]
    pub const fn line(self) -> u64 {
        self.line
    }

    /// Returns the one-based original-byte column number.
    #[must_use]
    pub const fn column(self) -> u64 {
        self.column
    }
}

/// Derives a one-based line and original-byte column at `offset`.
///
/// CRLF is one line ending, while a lone CR and a lone LF are each one line
/// ending. The LF byte in a CRLF pair maps to the following line's first column.
///
/// # Errors
///
/// Returns [`CoreError::OffsetOutOfBounds`] when `offset` exceeds `bytes`.
pub fn line_column_at(bytes: &[u8], offset: u64) -> Result<LineColumn, CoreError> {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    if offset > bytes.len() {
        return Err(CoreError::OffsetOutOfBounds {
            offset: u64::try_from(offset).unwrap_or(u64::MAX),
            source_bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }

    let mut line = 1_u64;
    let mut column = 1_u64;
    let mut index = 0_usize;
    while index < offset {
        match bytes[index] {
            b'\r' => {
                line = line.saturating_add(1);
                column = 1;
                index += 1;
                if index < offset && bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
            }
            b'\n' => {
                line = line.saturating_add(1);
                column = 1;
                index += 1;
            }
            _ => {
                column = column.saturating_add(1);
                index += 1;
            }
        }
    }
    Ok(LineColumn { line, column })
}

/// A stable, nonempty diagnostic code.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Creates a stable code when `value` is nonempty ASCII text.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvalidDiagnosticCode`] for empty or non-ASCII input.
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into();
        if value.is_empty() || !value.is_ascii() {
            return Err(CoreError::InvalidDiagnosticCode);
        }
        Ok(Self(value))
    }

    /// Returns the stable code text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The layer that owns a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticLayer {
    /// Capture or host-input validation.
    Capture,
    /// Source decoding, lexing, layout, or parsing.
    Syntax,
    /// Semantic validation.
    Semantics,
    /// Reader-facing consumer or probe observation.
    Consumer,
    /// Resource limits or cancellation.
    Resource,
    /// Internal invariant failure.
    Internal,
}

/// The severity assigned to a diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticSeverity {
    /// A non-authoritative informational observation.
    Note,
    /// A source or input failure that prevents authoritative output.
    Error,
}

/// A deterministically ordered, safe diagnostic projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// The stable diagnostic identifier.
    code: DiagnosticCode,
    /// The owning compiler or validation layer.
    layer: DiagnosticLayer,
    /// The user-facing severity classification.
    severity: DiagnosticSeverity,
    /// The primary exact source location.
    primary: SourceLocation,
    /// Deterministic safe values used to render the diagnostic.
    parameters: Vec<String>,
    /// Whether detail was bounded by a configured limit.
    truncated: bool,
}

impl Ord for Diagnostic {
    /// Orders diagnostics by source identity, byte range, code, then safe parameters.
    fn cmp(&self, other: &Self) -> CompareOrdering {
        self.primary
            .cmp(&other.primary)
            .then_with(|| self.code.cmp(&other.code))
            .then_with(|| self.parameters.cmp(&other.parameters))
            .then_with(|| self.layer.cmp(&other.layer))
            .then_with(|| self.severity.cmp(&other.severity))
            .then_with(|| self.truncated.cmp(&other.truncated))
    }
}

impl PartialOrd for Diagnostic {
    /// Delegates partial comparison to the total canonical diagnostic ordering.
    fn partial_cmp(&self, other: &Self) -> Option<CompareOrdering> {
        Some(self.cmp(other))
    }
}

impl Diagnostic {
    /// Creates a diagnostic with safe, deterministic parameters.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        layer: DiagnosticLayer,
        severity: DiagnosticSeverity,
        primary: SourceLocation,
        parameters: Vec<String>,
        truncated: bool,
    ) -> Self {
        Self {
            code,
            layer,
            severity,
            primary,
            parameters,
            truncated,
        }
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the owning layer.
    #[must_use]
    pub const fn layer(&self) -> DiagnosticLayer {
        self.layer
    }

    /// Returns the severity.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns the primary source location.
    #[must_use]
    pub const fn primary(&self) -> SourceLocation {
        self.primary
    }

    /// Returns only safe diagnostic parameters.
    #[must_use]
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    /// Returns whether parameters or related detail were truncated by limits.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }
}

/// The deterministic budgets that bound captured-input processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralLimits {
    /// The maximum number of captured source bytes.
    source_bytes: u64,
    /// The maximum number of retained diagnostics.
    diagnostics: u32,
}

impl StructuralLimits {
    /// Creates limits when all bounds are nonzero.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ZeroLimit`] when a required bound is zero.
    pub const fn new(source_bytes: u64, diagnostics: u32) -> Result<Self, CoreError> {
        if source_bytes == 0 || diagnostics == 0 {
            return Err(CoreError::ZeroLimit);
        }
        Ok(Self {
            source_bytes,
            diagnostics,
        })
    }

    /// Returns the maximum exact captured source-byte count.
    #[must_use]
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }

    /// Returns the maximum retained diagnostic count.
    #[must_use]
    pub const fn diagnostics(self) -> u32 {
        self.diagnostics
    }
}

/// A shareable cancellation signal for bounded compilation work.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Creates a cancellation signal that is initially clear.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation for every holder of this signal.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// A stable non-authoritative outcome class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultClass {
    /// Capture input could not be accepted.
    Capture,
    /// A structural limit was exceeded.
    Resource,
    /// Captured source was rejected by decoding, lexing, layout, or parsing.
    Syntax,
    /// Parsed source was rejected by name, type, or value semantics.
    Semantics,
    /// The caller cancelled bounded work.
    Cancellation,
    /// A compiler invariant failed without authoritative output.
    Internal,
}

/// A failure while constructing foundational contracts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreError {
    /// A half-open span ended before it began.
    InvertedSpan {
        /// The supplied first included byte offset.
        start: u64,
        /// The supplied first excluded byte offset.
        end: u64,
    },
    /// A requested original-byte offset was outside the captured source.
    OffsetOutOfBounds {
        /// The requested original-byte offset.
        offset: u64,
        /// The exact captured source byte count.
        source_bytes: u64,
    },
    /// A diagnostic code was empty or contained non-ASCII text.
    InvalidDiagnosticCode,
    /// A required deterministic limit was zero.
    ZeroLimit,
    /// An NHT tag contained non-ASCII text.
    InvalidTranscriptTag,
    /// An NHT tag, payload, or combined frame exceeded its fixed-width length.
    TranscriptLengthExceeded,
}

#[cfg(test)]
/// Unit tests for foundational value contracts.
mod tests {
    use super::{
        ByteSpan, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity, SemanticDigest,
        SourceContentDigest, SourceLocation, line_column_at, nht_frame,
    };

    #[test]
    /// Verifies that exact source bytes affect the typed digest.
    fn exact_source_bytes_affect_the_digest() {
        assert_ne!(
            SourceContentDigest::from_bytes(b"a\n"),
            SourceContentDigest::from_bytes(b"a\r\n")
        );
    }

    #[test]
    /// Verifies frozen SHA-256 text vectors for exact captured source bytes.
    fn source_digest_uses_the_frozen_sha256_text_form() {
        assert_eq!(
            SourceContentDigest::from_bytes(b"").to_string(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            SourceContentDigest::from_bytes(b"abc").to_string(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    /// Verifies exact NHT framing widths, order, and domain-separated hashing.
    fn semantic_digest_uses_the_frozen_nht_v1_frame() {
        assert_eq!(
            nht_frame("x", b"ab").expect("short frame should succeed"),
            [0, 1, b'x', 0, 0, 0, 0, 0, 0, 0, 2, b'a', b'b']
        );
        assert_ne!(
            SemanticDigest::from_nht("domain-a", b"value").expect("ASCII domain should hash"),
            SemanticDigest::from_nht("domain-b", b"value").expect("ASCII domain should hash")
        );
    }

    #[test]
    /// Verifies that inverted source spans are rejected.
    fn inverted_spans_are_rejected() {
        assert!(ByteSpan::new(2, 1).is_err());
    }

    #[test]
    /// Verifies that CRLF and lone CR retain deterministic original-byte locations.
    fn line_columns_distinguish_supported_line_endings() {
        assert_eq!(
            line_column_at(b"a\r\nb\rc", 3).expect("offset should be valid"),
            super::LineColumn { line: 2, column: 1 }
        );
        assert_eq!(
            line_column_at(b"a\r\nb\rc", 5).expect("offset should be valid"),
            super::LineColumn { line: 3, column: 1 }
        );
    }

    #[test]
    /// Verifies canonical diagnostic ordering uses source position before stable code.
    fn diagnostics_sort_by_source_position_before_stable_code() {
        let source = SourceContentDigest::from_bytes(b"source");
        let later = Diagnostic::new(
            DiagnosticCode::new("AAA").expect("code should be valid"),
            DiagnosticLayer::Syntax,
            DiagnosticSeverity::Error,
            SourceLocation::new(source, ByteSpan::new(2, 3).expect("span should be valid")),
            Vec::new(),
            false,
        );
        let earlier = Diagnostic::new(
            DiagnosticCode::new("ZZZ").expect("code should be valid"),
            DiagnosticLayer::Syntax,
            DiagnosticSeverity::Error,
            SourceLocation::new(source, ByteSpan::new(1, 2).expect("span should be valid")),
            Vec::new(),
            false,
        );
        assert!(earlier < later);
    }
}
