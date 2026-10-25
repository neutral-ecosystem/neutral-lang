// SPDX-License-Identifier: Apache-2.0

//! Neutral compilation pipeline boundary.
//!
//! This crate owns capture contracts, the private frontend and semantic model,
//! and lowering into public logical IR. Its pure captured-input compilation
//! path must not use filesystem, process, environment, network, locale, or clock
//! authority. Project capture and the private frontend preserve that same
//! effect-free boundary as language features evolve.
//!
//! # Pipeline map
//! Hosts supply bytes and locks; capture checks their closed envelope and freezes
//! exact inputs. The private frontend handles source spelling, then graph and
//! semantic phases resolve logical dependencies before contextual lowering
//! publishes complete IR. The independent reader is a separate validation
//! boundary, not a shortcut around compiler phases.
//!
//! Current complete expression projects are the host integration boundary.
//! Optional syntax caching reuses only private parsed units; graph, vocabulary,
//! semantics and source-accounting work remain fresh.

use neutral_core::CancellationToken;

mod frontend;
mod language;
mod project_capture;
mod project_identity;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

pub use frontend::{
    ExpressionCompilationCache, ExpressionCompileFailure, ProjectCacheLimits, ProjectCacheStats,
    compile_expression_project, format_expression_source,
};

pub use project_capture::{
    CAPTURE_REQUEST_VERSION, CapturedExpressionProject, CapturedExpressionProjectRequest,
    CapturedProjectRequest, CapturedProjectRequestBuilder, CapturedProjectSource,
    CapturedProjectVocabulary, CapturedSourceInput, CapturedVocabularyInput,
    CompositionCaptureFailure, ExpressionCaptureFailure, ProjectCaptureControls,
    ProjectCaptureError, ProjectCaptureLimitValues, ProjectCaptureLimits,
    ProjectCaptureResourceFacts, ProjectHostError, capture_expression_project, source_module_name,
};
pub(crate) use project_capture::{CapturedCompositionProject, CapturedInputs};

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
