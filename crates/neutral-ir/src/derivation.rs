// SPDX-License-Identifier: Apache-2.0

//! Processing inputs and resource facts accompanying a compiled document.
//!
//! Meaning, acceptance limits and diagnostic policy occupy separate partitions
//! so consumers can distinguish semantic inputs from processing conditions.
//! The manifest records schema versions and captured vocabulary identity; it
//! describes how a result was derived, rather than replacing logical equality.
//!
//! Resource facts describe successful retained output; acceptance limits describe
//! what a request was allowed to process. They are not interchangeable counters.
//! The constructors snapshot supplied values without checking facts against limits
//! or source bytes. Independent consumers must recompute applicable facts rather
//! than trust the manifest's declarations.
//!
//! This module supplies the minimal document manifest. Complete expression-project
//! derivation identities use their own explicit processing context and profile
//! tuple in [`crate::project_identity`]. A manifest or digest records derivation
//! inputs; it does not attest that a particular producer actually ran them.

use crate::{
    LOGICAL_IR_SCHEMA_VERSION, PROVENANCE_VERSION, SOURCE_MAP_VERSION, VocabularyIdentity,
};
use neutral_core::{SourceContentDigest, StructuralLimits};

/// Deterministic resource facts recorded for successful compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceFacts {
    /// Exact captured source bytes.
    source_bytes: u64,
    /// Number of authoritative declarations.
    declarations: u64,
    /// Number of retained diagnostics.
    diagnostics: u64,
    /// Decoded UTF-8 bytes retained by string values.
    decoded_string_bytes: u64,
}

impl ResourceFacts {
    /// Creates successful-compilation resource accounting.
    #[must_use]
    pub const fn new(
        source_bytes: u64,
        declarations: u64,
        diagnostics: u64,
        decoded_string_bytes: u64,
    ) -> Self {
        Self {
            source_bytes,
            declarations,
            diagnostics,
            decoded_string_bytes,
        }
    }

    /// Returns the exact captured source byte count.
    #[must_use]
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }

    /// Returns the authoritative declaration count.
    #[must_use]
    pub const fn declarations(self) -> u64 {
        self.declarations
    }

    /// Returns the retained diagnostic count.
    #[must_use]
    pub const fn diagnostics(self) -> u64 {
        self.diagnostics
    }

    /// Returns decoded UTF-8 bytes retained by string values.
    #[must_use]
    pub const fn decoded_string_bytes(self) -> u64 {
        self.decoded_string_bytes
    }
}

/// Meaning-affecting derivation inputs for captured compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeaningPartition {
    /// Exact captured source content identity.
    source_digest: SourceContentDigest,
}

impl MeaningPartition {
    /// Returns the exact source content identity.
    #[must_use]
    pub const fn source_digest(self) -> SourceContentDigest {
        self.source_digest
    }
}

/// Acceptance and resource inputs for captured compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptancePartition {
    /// Maximum accepted captured source bytes.
    source_bytes: u64,
    /// Maximum retained diagnostics.
    diagnostics: u32,
    /// Maximum decoded UTF-8 bytes in one string scalar.
    string_bytes: u64,
    /// Maximum significant decimal digits in one exact number.
    numeric_digits: u64,
    /// Maximum absolute decimal scale in one exact number.
    numeric_scale: u64,
    /// Maximum root declarations.
    declarations: u64,
    /// Maximum record fields per declaration or value.
    record_fields: u64,
    /// Maximum contextual-record nesting depth.
    nesting_depth: u64,
    /// Maximum list items per value.
    list_items: u64,
    /// Maximum recursively traversed value nodes.
    traversal_nodes: u64,
}

impl AcceptancePartition {
    /// Captures the complete resource-limit partition from core limits.
    /// Copies policy values, not observed usage; later phases still enforce them.
    #[must_use]
    pub const fn from_limits(limits: StructuralLimits) -> Self {
        Self {
            source_bytes: limits.source_bytes(),
            diagnostics: limits.diagnostics(),
            string_bytes: limits.string_bytes(),
            numeric_digits: limits.numeric_digits(),
            numeric_scale: limits.numeric_scale(),
            declarations: limits.declarations(),
            record_fields: limits.record_fields(),
            nesting_depth: limits.nesting_depth(),
            list_items: limits.list_items(),
            traversal_nodes: limits.traversal_nodes(),
        }
    }

    /// Returns the captured source-byte limit.
    #[must_use]
    pub const fn source_byte_limit(self) -> u64 {
        self.source_bytes
    }

    /// Returns the captured diagnostic limit.
    #[must_use]
    pub const fn diagnostic_limit(self) -> u32 {
        self.diagnostics
    }

    /// Returns the decoded string-byte limit.
    #[must_use]
    pub const fn string_byte_limit(self) -> u64 {
        self.string_bytes
    }

    /// Returns the exact-number significant-digit limit.
    #[must_use]
    pub const fn numeric_digit_limit(self) -> u64 {
        self.numeric_digits
    }

    /// Returns the exact-number absolute-scale limit.
    #[must_use]
    pub const fn numeric_scale_limit(self) -> u64 {
        self.numeric_scale
    }

    /// Returns the root declaration-count limit.
    #[must_use]
    pub const fn declaration_limit(self) -> u64 {
        self.declarations
    }

    /// Returns the per-record field-count limit.
    #[must_use]
    pub const fn record_field_limit(self) -> u64 {
        self.record_fields
    }

    /// Returns the contextual-record nesting-depth limit.
    #[must_use]
    pub const fn nesting_depth_limit(self) -> u64 {
        self.nesting_depth
    }

    /// Returns the per-list item-count limit.
    #[must_use]
    pub const fn list_item_limit(self) -> u64 {
        self.list_items
    }

    /// Returns the recursive value traversal-node limit.
    #[must_use]
    pub const fn traversal_node_limit(self) -> u64 {
        self.traversal_nodes
    }
}

/// Diagnostic/output-policy inputs for captured compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticPartition {
    /// Whether safe bounded diagnostics are enabled.
    pub(super) safe_bounded_output: bool,
}

impl DiagnosticPartition {
    /// Returns whether diagnostics use the safe bounded output contract.
    #[must_use]
    pub const fn safe_bounded_output(self) -> bool {
        self.safe_bounded_output
    }
}

/// Immutable partitioned derivation manifest for successful compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivationManifest {
    /// Frozen language behavior version.
    language_behavior_version: String,
    /// Meaning-affecting inputs.
    meaning: MeaningPartition,
    /// Acceptance and resource inputs.
    acceptance: AcceptancePartition,
    /// Diagnostic and output-policy inputs.
    diagnostics: DiagnosticPartition,
    /// Observed deterministic resource facts.
    resource_facts: ResourceFacts,
    /// Exact vocabulary identity facts when the source uses one capture.
    vocabulary: Option<VocabularyIdentity>,
}

impl DerivationManifest {
    /// Creates the minimal partitioned derivation manifest.
    /// Initially records safe bounded diagnostic policy and no vocabulary; the
    /// caller supplies validated accounting and can attach exact vocabulary facts.
    #[must_use]
    pub fn new(
        language_behavior_version: impl Into<String>,
        source_digest: SourceContentDigest,
        acceptance: AcceptancePartition,
        resource_facts: ResourceFacts,
    ) -> Self {
        Self {
            language_behavior_version: language_behavior_version.into(),
            meaning: MeaningPartition { source_digest },
            acceptance,
            diagnostics: DiagnosticPartition {
                safe_bounded_output: true,
            },
            resource_facts,
            vocabulary: None,
        }
    }

    /// Attaches exact meaning-affecting captured vocabulary facts.
    #[must_use]
    pub fn with_vocabulary(mut self, vocabulary: VocabularyIdentity) -> Self {
        self.vocabulary = Some(vocabulary);
        self
    }

    /// Returns exact captured vocabulary derivation facts, when present.
    #[must_use]
    pub const fn vocabulary(&self) -> Option<&VocabularyIdentity> {
        self.vocabulary.as_ref()
    }

    /// Returns the language behavior version.
    #[must_use]
    pub fn language_behavior_version(&self) -> &str {
        &self.language_behavior_version
    }

    /// Returns the logical IR schema version.
    #[must_use]
    pub const fn logical_ir_schema_version(&self) -> &'static str {
        LOGICAL_IR_SCHEMA_VERSION
    }

    /// Returns the source-map schema version.
    #[must_use]
    pub const fn source_map_version(&self) -> &'static str {
        SOURCE_MAP_VERSION
    }

    /// Returns the provenance schema version.
    #[must_use]
    pub const fn provenance_version(&self) -> &'static str {
        PROVENANCE_VERSION
    }

    /// Returns the meaning-affecting partition.
    #[must_use]
    pub const fn meaning(&self) -> MeaningPartition {
        self.meaning
    }

    /// Returns the acceptance/resource partition.
    #[must_use]
    pub const fn acceptance(&self) -> AcceptancePartition {
        self.acceptance
    }

    /// Returns the diagnostic/output partition.
    #[must_use]
    pub const fn diagnostics(&self) -> DiagnosticPartition {
        self.diagnostics
    }

    /// Returns observed deterministic resource facts.
    #[must_use]
    pub const fn resource_facts(&self) -> ResourceFacts {
        self.resource_facts
    }
}
