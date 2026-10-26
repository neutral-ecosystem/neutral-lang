// SPDX-License-Identifier: Apache-2.0

//! Safe foundational constructor failures and whole-request outcome classes.
//!
//! [`CoreError`] records the small set of failures needed by core constructors:
//! invalid ranges/codes, zero ceilings and unrepresentable transcript framing.
//! Its payloads are numeric facts rather than captured text, paths or formatted
//! error messages. The layer handling the failure chooses a diagnostic and its
//! safe presentation; core does not depend on a renderer or compiler phase.
//!
//! [`ResultClass`] serves a different purpose: it groups whole-request outcomes
//! for callers and reports. A class is neither a detailed diagnostic nor evidence
//! of a valid partial artifact. Allocation and higher-layer validation failures
//! have their own error types and are mapped to outcomes by their owning layer.

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
    /// A typed identity-reference target or edge was invalid.
    Reference,
    /// A captured vocabulary contract or payload was invalid.
    Vocabulary,
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
