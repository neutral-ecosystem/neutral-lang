// SPDX-License-Identifier: Apache-2.0

//! Explicit consumer policy over current independently validated expression projects.

use neutral_core::SourceLocation;
use neutral_ir::{
    ModuleSymbolIdentity, composition::ValuePathSegment, project_interface::ProjectPublicType,
};

mod budget;
pub mod expression;

/// Frozen typed request for selecting current expression-project entries.
pub const ENTRY_REQUEST_SCHEMA: &str = "neutral.entry-request/1";

/// Entry cardinality and explicit-root policy shared by expression reader and probe.
#[derive(Clone, Copy)]
pub enum EntrySelection<'a> {
    /// Enumerate all public bindings of the exact nominal type.
    Enumerate,
    /// Require exactly one public binding of the exact nominal type.
    Unique,
    /// Select these exact public binding symbols; an empty set stays empty.
    Explicit(&'a [ModuleSymbolIdentity]),
}

/// Typed entry-selection request; it carries no execution or acquisition authority.
#[derive(Clone, Copy)]
pub struct EntryRequest<'a> {
    /// Must equal [`ENTRY_REQUEST_SCHEMA`].
    pub schema: &'a str,
    /// Exact public nominal contract, including locked vocabulary revision.
    pub contract: &'a ProjectPublicType,
    /// Explicit selection policy.
    pub selection: EntrySelection<'a>,
}

/// Safe classifications; errors never include supplied text or private identifiers.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConsumerError {
    /// Required schema or feature is unsupported.
    Unsupported,
    /// Envelope or protocol registration is malformed.
    InvalidInput,
    /// A requested member is unavailable.
    MissingMember,
    /// The payload does not have the exact declared nominal type.
    TypeMismatch,
    /// A target is private or absent, without distinguishing the two.
    PrivateTarget,
    /// No explicit entry was supplied where required by a consumer.
    MissingEntry,
    /// More than one entry remains where a consumer requires one.
    AmbiguousEntry,
    /// Location evidence is not a checked public occurrence.
    InvalidAttribution,
    /// Required interpretive contract facts are unavailable.
    IncompleteContract,
    /// A bound, checked arithmetic or fallible allocation failed.
    Limit,
    /// The request was cancelled before complete publication.
    Cancelled,
}

impl ConsumerError {
    /// Returns the frozen reader-contract diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "NEU-CONS-001",
            Self::InvalidInput => "NEU-CONS-002",
            Self::MissingMember => "NEU-CONS-003",
            Self::TypeMismatch => "NEU-CONS-004",
            Self::PrivateTarget => "NEU-CONS-005",
            Self::MissingEntry => "NEU-CONS-006",
            Self::AmbiguousEntry => "NEU-CONS-007",
            Self::InvalidAttribution => "NEU-CONS-008",
            Self::IncompleteContract => "NEU-CONS-009",
            Self::Limit => "NEU-CONS-010",
            Self::Cancelled => "NEU-CONS-011",
        }
    }
}
impl std::fmt::Display for ConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for ConsumerError {}

/// Explicit current consumer contract and feature/code registration.
#[derive(Clone, Copy)]
pub struct ConsumerProtocol<'a> {
    /// Exact public nominal contract identifying reports, not a structural shape.
    pub report_contract: &'a ProjectPublicType,
    /// Feature IDs this consumer explicitly supports.
    pub supported_features: &'a [&'a str],
    /// Registered consumer codes; the reserved `NEU-` namespace is unavailable.
    pub diagnostic_codes: &'a [&'a str],
}

/// Independent positive envelope controls, intersected with frozen hard ceilings.
#[derive(Clone, Copy, Debug)]
pub struct ConsumerLimits {
    /// Independently bounded selected roots per request.
    pub roots: u64,
    /// Independently bounded matching candidates per enumeration.
    pub candidates: u64,
    /// Interpretive dependency edges followed during public closure.
    pub closure_edges: u64,
    /// Aggregate retained envelope data bytes.
    pub bytes: u64,
    /// Individual identifier/text bytes.
    pub text_bytes: u64,
    /// Feature registrations and requested feature count.
    pub features: u64,
    /// Report diagnostic count and registered code count.
    pub diagnostics: u64,
    /// Total available/unavailable location entries per report.
    pub locations: u64,
    /// Segments in each occurrence path.
    pub path_segments: u64,
    /// Validation visits and conservative comparison work.
    pub work: u64,
    /// Retained projection, obligation and diagnostic index bytes.
    pub output_bytes: u64,
}
impl Default for ConsumerLimits {
    fn default() -> Self {
        Self {
            roots: 1_000_000,
            candidates: 1_000_000,
            closure_edges: 1_000_000,
            bytes: 67_108_864,
            text_bytes: 1_048_576,
            features: 1_000_000,
            diagnostics: 1_000_000,
            locations: 1_000_000,
            path_segments: 64,
            work: 100_000_000,
            output_bytes: 100_000_000,
        }
    }
}
impl ConsumerLimits {
    /// Intersects independent caller/reader/hard controls before any request can retain data.
    pub(super) fn effective(self, work: u64) -> Result<Self, ConsumerError> {
        let hard = Self::default();
        let limits = Self {
            roots: self.roots.min(hard.roots),
            candidates: self.candidates.min(hard.candidates),
            closure_edges: self.closure_edges.min(hard.closure_edges),
            bytes: self.bytes.min(hard.bytes),
            text_bytes: self.text_bytes.min(hard.text_bytes),
            features: self.features.min(hard.features),
            diagnostics: self.diagnostics.min(hard.diagnostics),
            locations: self.locations.min(hard.locations),
            path_segments: self.path_segments.min(hard.path_segments),
            work: self.work.min(hard.work).min(work),
            output_bytes: self.output_bytes.min(hard.output_bytes),
        };
        if [
            limits.roots,
            limits.candidates,
            limits.closure_edges,
            limits.bytes,
            limits.text_bytes,
            limits.features,
            limits.diagnostics,
            limits.locations,
            limits.path_segments,
            limits.work,
            limits.output_bytes,
        ]
        .contains(&0)
        {
            return Err(ConsumerError::Limit);
        }
        Ok(limits)
    }
}

/// Public, opaque revision identifiers supplied explicitly by the consumer.
#[derive(Clone, Copy)]
pub struct ConsumerContext<'a> {
    /// Adapter contract revision.
    pub adapter_revision: &'a str,
    /// Target contract revision.
    pub target_revision: &'a str,
    /// External input revision, separate from immutable project identity.
    pub input_revision: &'a str,
}

/// Explicit reasons for unavailable evidence, without private identifiers.
#[derive(Clone, Copy, Debug)]
pub enum LocationUnavailable {
    /// Public projection redacted the evidence.
    Redacted,
    /// Evidence was not retained.
    NotRetained,
    /// This value has no source occurrence.
    NotSourceOrigin,
}

/// Checked public source evidence, or an explicit absence; never a host locator.
#[derive(Clone, Copy)]
pub enum ConsumerLocation<'a> {
    /// No location is asserted.
    Unavailable(LocationUnavailable),
    /// Exact evidence retained for the public selected binding.
    Available {
        /// Public binding owning the occurrence.
        subject: &'a ModuleSymbolIdentity,
        /// Materialized occurrence path.
        path: &'a [ValuePathSegment],
        /// Exact original-byte evidence.
        location: SourceLocation,
    },
}
