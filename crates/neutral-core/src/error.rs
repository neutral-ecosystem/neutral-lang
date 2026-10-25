// SPDX-License-Identifier: Apache-2.0

//! Safe foundational constructor failures and whole-request outcome classes.

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
