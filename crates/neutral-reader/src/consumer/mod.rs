// SPDX-License-Identifier: Apache-2.0

//! Closed, data-only consumer envelopes over independently validated public values.
//!
//! Validation binds reports to the complete logical project, not to a selected
//! view. It does not establish the truth of a consumer decision, authorize effects,
//! discover entries, choose alternatives or evaluate deferred values. Transport
//! codecs must independently reject unknown fields and bound raw input bytes.

#![doc = include_str!("README.md")]

use crate::composition::CompositionLogicalIdentity;
use neutral_core::SourceLocation;
use neutral_ir::{
    ModuleSymbolIdentity, composition::ValuePathSegment, project_interface::ProjectPublicType,
};

mod validation;
pub use validation::ConsumerBoundary;

/// Frozen description envelope schema, independent of language and wire versions.
pub const DESCRIPTION_SCHEMA: &str = "neutral.consumer-description/1";
/// Frozen report envelope schema.
pub const REPORT_SCHEMA: &str = "neutral.consumer-report/1";

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

/// Explicit consumer-owned registration; lists must be sorted and unique.
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
    /// Aggregate retained envelope data bytes (not transport encoding bytes).
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
    /// Retained diagnostic index bytes.
    pub output_bytes: u64,
}

impl Default for ConsumerLimits {
    fn default() -> Self {
        Self {
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

/// Public, opaque revision identifiers supplied explicitly by the consumer.
/// Never put credentials, host paths or private values in these identifiers.
#[derive(Clone, Copy)]
pub struct ConsumerContext<'a> {
    /// Adapter contract revision.
    pub adapter_revision: &'a str,
    /// Target contract revision.
    pub target_revision: &'a str,
    /// External input revision, separate from immutable project identity.
    pub input_revision: &'a str,
}

/// Closed description request; the payload names an already validated public binding.
#[derive(Clone, Copy)]
pub struct DescriptionRequest<'a> {
    /// Must equal [`DESCRIPTION_SCHEMA`].
    pub schema: &'a str,
    /// Exact public nominal payload type.
    pub contract: &'a ProjectPublicType,
    /// Public adapter revision.
    pub adapter_revision: &'a str,
    /// Public target revision.
    pub target_revision: &'a str,
    /// Sorted unique required features; unsupported features fail closed.
    pub required_features: &'a [&'a str],
    /// Existing public binding, not an unvalidated value bag.
    pub payload: &'a ModuleSymbolIdentity,
}

/// Explicit decision data, never an instruction to execute or approve an alternative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerStatus {
    /// The consumer reports acceptance in the exact supplied context.
    Accepted,
    /// The consumer reports rejection.
    Rejected,
    /// A new explicitly approved context and validation are required.
    AlternativeRequired,
}

/// Diagnostic phase is derived from its code, preventing compiler impersonation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConsumerDiagnosticCode<'a> {
    /// Frozen reader-contract classification.
    ReaderContract(ConsumerError),
    /// Exact code registered by this consumer protocol.
    Consumer(&'a str),
}
impl<'a> ConsumerDiagnosticCode<'a> {
    /// Returns the closed phase spelling.
    #[must_use]
    pub const fn phase(self) -> &'static str {
        match self {
            Self::ReaderContract(_) => "reader-contract",
            Self::Consumer(_) => "consumer",
        }
    }
    /// Returns the registered code spelling.
    #[must_use]
    pub const fn code(self) -> &'a str {
        match self {
            Self::ReaderContract(code) => code.code(),
            Self::Consumer(code) => code,
        }
    }
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
    /// Must exactly match a public declaration or retained source-origin occurrence.
    Available {
        /// Public binding owning the occurrence.
        subject: &'a ModuleSymbolIdentity,
        /// Materialized occurrence path.
        path: &'a [ValuePathSegment],
        /// Exact independently checked original-byte evidence.
        location: SourceLocation,
    },
}

/// Closed diagnostic data; text messages and private implementation bags are absent.
#[derive(Clone, Copy)]
pub struct ConsumerDiagnostic<'a> {
    /// Determines phase and registered code.
    pub code: ConsumerDiagnosticCode<'a>,
    /// Exact public binding owning this diagnostic.
    pub subject: &'a ModuleSymbolIdentity,
    /// Typed occurrence path, not a string selector.
    pub path: &'a [ValuePathSegment],
    /// Checked evidence or explicit absence.
    pub primary: ConsumerLocation<'a>,
    /// Independently checked related public evidence.
    pub related: &'a [ConsumerLocation<'a>],
}

/// Closed report bound to complete project identity and explicit external context.
#[derive(Clone, Copy)]
pub struct ReportRequest<'a> {
    /// Must equal [`REPORT_SCHEMA`].
    pub schema: &'a str,
    /// Must equal the registered exact public report contract.
    pub contract: &'a ProjectPublicType,
    /// Complete project logical identity, independently recomputed by the boundary.
    pub project_identity: CompositionLogicalIdentity,
    /// Explicit public entry or no selected entry; no discovery occurs here.
    pub entry: Option<&'a ModuleSymbolIdentity>,
    /// Adapter, target and input revisions; not project identity inputs.
    pub context: ConsumerContext<'a>,
    /// Consumer decision, not reader authorization.
    pub status: ConsumerStatus,
    /// Diagnostics canonicalized only after complete validation.
    pub diagnostics: &'a [ConsumerDiagnostic<'a>],
}

/// A validated description borrowing its exact public payload without copying it.
pub struct ValidatedConsumerDescription<'p, 'r> {
    /// Closed envelope checked against the registered protocol.
    pub(super) request: &'r DescriptionRequest<'r>,
    /// Borrowed reader-owned public value; no revalidation bypass or copy.
    pub(super) payload: &'p neutral_ir::composition::CompositionBinding,
}
impl ValidatedConsumerDescription<'_, '_> {
    /// Returns the checked public binding and its already materialized value.
    #[must_use]
    pub const fn payload(&self) -> &neutral_ir::composition::CompositionBinding {
        self.payload
    }
    /// Returns the closed checked envelope.
    #[must_use]
    pub const fn request(&self) -> &DescriptionRequest<'_> {
        self.request
    }
}
impl std::fmt::Debug for ValidatedConsumerDescription<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ValidatedConsumerDescription { .. }")
    }
}

/// Completely checked report with a fallibly retained canonical diagnostic index.
pub struct ValidatedConsumerReport<'a> {
    /// Validated context, identity, status and exact nominal contract.
    pub(super) request: &'a ReportRequest<'a>,
    /// Canonical allocation-aware index over completely checked diagnostics.
    pub(super) diagnostics: Vec<&'a ConsumerDiagnostic<'a>>,
}
impl<'a> ValidatedConsumerReport<'a> {
    /// Returns diagnostics in phase/subject/path/code order.
    #[must_use]
    pub fn diagnostics(&self) -> &[&'a ConsumerDiagnostic<'a>] {
        &self.diagnostics
    }
    /// Only `Accepted` is acceptance; alternatives are never implicitly approved.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.status() == ConsumerStatus::Accepted
    }
    /// Returns the complete project identity checked against reader authority.
    #[must_use]
    pub const fn project_identity(&self) -> CompositionLogicalIdentity {
        self.request.project_identity
    }
    /// Returns explicit decision data, not authorization.
    #[must_use]
    pub const fn status(&self) -> ConsumerStatus {
        self.request.status
    }
    /// Returns the exact registered nominal report contract.
    #[must_use]
    pub const fn contract(&self) -> &ProjectPublicType {
        self.request.contract
    }
    /// Returns the explicitly selected public binding, if any.
    #[must_use]
    pub const fn entry(&self) -> Option<&ModuleSymbolIdentity> {
        self.request.entry
    }
    /// Returns public consumer revision data, excluded from project identity.
    #[must_use]
    pub const fn context(&self) -> ConsumerContext<'a> {
        self.request.context
    }
}
impl std::fmt::Debug for ValidatedConsumerReport<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedConsumerReport")
            .field("diagnostics", &self.diagnostics.len())
            .finish_non_exhaustive()
    }
}
