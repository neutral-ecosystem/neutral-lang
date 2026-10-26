// SPDX-License-Identifier: Apache-2.0

//! Foundational contracts shared across the Neutral implementation.
//!
//! This crate owns source identity, source spans, diagnostics, resource limits,
//! cancellation, and result classification. It must remain independent of the
//! compiler, reader, command-line hosts, and ambient host services.
//!
//! # How the pieces fit together
//!
//! Capture computes a [`SourceContentDigest`] and associates original-byte
//! [`ByteSpan`]s with it through [`SourceLocation`]. Later phases use these same
//! coordinates in [`Diagnostic`]s rather than substituting normalized text or
//! machine-local paths. [`VocabularyContentDigest`] identifies separately captured
//! bundle bytes; [`SemanticDigest`] identifies an explicitly constructed transcript.
//!
//! Request owners supply [`StructuralLimits`] and a [`CancellationToken`]. These
//! values describe policy; they do not automatically count work or stop a phase.
//! The [`allocation`] module makes storage failure returnable, while [`ordered`]
//! supplies deterministic indexes whose insertion cost callers must budget.
//! [`profile`] reports source-language availability independently of the Cargo
//! package version and the more specific artifact/expression feature contracts.
//!
//! # Contract boundaries
//!
//! Source and vocabulary digests identify exact bytes; semantic digests identify
//! explicitly framed meaning. They are distinct types so callers cannot quietly
//! replace one identity layer with another. Locations likewise retain original
//! byte offsets, not decoded-character or editor-display coordinates.
//!
//! This crate supplies primitives, not a validator for complete programs. Hosts
//! own acquisition, higher layers enforce their work budgets, and consumers must
//! validate external values before treating these contracts as authoritative.
//!
//! # Failure and publication
//!
//! [`CoreError`] describes invalid foundational values; [`ResultClass`] gives
//! higher layers a coarse request outcome. Neither establishes that an artifact
//! is valid. Fallible helpers may discard their temporary storage on failure,
//! but complete-request atomicity belongs to the caller: validate all companions,
//! observe cancellation, and only then publish a result or cache generation.
//! Fallible allocation is explicit, not a property of every API in this crate;
//! for example, ordinary diagnostic construction accepts already-owned strings.

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
