// SPDX-License-Identifier: Apache-2.0

//! Foundational contracts shared across the Neutral implementation.
//!
//! This crate owns source identity, source spans, diagnostics, resource limits,
//! cancellation, and result classification. It must remain independent of the
//! compiler, reader, command-line hosts, and ambient host services.
//!
//! # Contract boundaries
//! Source and vocabulary digests identify exact bytes; semantic digests identify
//! explicitly framed meaning. They are distinct types so callers cannot quietly
//! replace one identity layer with another. Locations likewise retain original
//! byte offsets, not decoded-character or editor-display coordinates.
//!
//! This crate supplies primitives, not a validator for complete programs. Hosts
//! own acquisition, higher layers enforce their work budgets, and consumers must
//! validate external values before treating these contracts as authoritative.

pub mod allocation;
pub mod ordered;
pub mod profile;

mod cancellation;
mod diagnostics;
mod digest;
mod error;
mod limits;
mod source;

pub use cancellation::CancellationToken;
pub use diagnostics::{Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity};
pub use digest::{
    DigestTextError, EncodedSectionDigest, HASH_TRANSCRIPT_ENVELOPE, SemanticDigest,
    SourceContentDigest, VocabularyContentDigest, nht_frame,
};
pub use error::{CoreError, ResultClass};
pub use limits::StructuralLimits;
pub use source::{ByteSpan, LineColumn, SourceLocation, line_column_at};

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
