// SPDX-License-Identifier: Apache-2.0

//! Shared scalar, nominal, source and raw `/3` expression IR contracts.
//!
//! Producers may construct these data structures; construction alone does not
//! establish validity. The independent reader checks hostile complete artifacts
//! before exposing authoritative traversal. This crate owns no acquisition,
//! compiler parser state, host I/O or evaluation effects.
//!
//! # Model layers
//!
//! The root exports reusable numbers, nominal identities, materialized values
//! and source/provenance companions. [`composition`] adds the shared record,
//! variant, presence, default and restriction model used by source declarations
//! and captured vocabulary contracts. [`expression`] retains typed inputs,
//! output selectors, branches and public templates as data in a complete project.
//! [`project`] supplies module, source and resource companions common to that model.
//!
//! [`project_interface`] defines alias-independent public types and structural
//! edges. [`project_identity`] constructs separate captured, logical, interface,
//! derivation and artifact transcripts; these identities answer different questions
//! and cannot be substituted merely because each ultimately contains a SHA-256 hash.
//!
//! # Assembly is not validation
//!
//! Many constructors accept already-owned fields without checking their mutual
//! consistency. A name called canonical in a field's documentation is an invariant
//! required of valid data, not proof supplied by its Rust type. Raw projects can
//! contain forged type annotations, counts, fingerprints or source evidence.
//! Shape/retention inspection bounds the work needed to examine that data; the
//! semantic and independent reader layers establish typing, closure and authority.
//!
//! # Ownership and compatibility
//!
//! Fallible copy paths retain complete data under explicit budgets; ordinary
//! `Clone`, formatting and older scalar helpers are not universally allocation-aware.
//! Profile constants and transcript tags describe governed contracts, independently
//! of Cargo release versions. Keeping shared scalar types or historical selectors
//! does not make an obsolete complete transport accepted by current readers.

mod symbol;
pub use symbol::{
    ElementId, LogicalModuleIdentity, ModuleSymbolIdentity, NominalTypeIdentity,
    VocabularyIdentity, VocabularyTypeIdentity,
};
mod exact_number;
pub use exact_number::ExactNumber;
mod value;
pub use value::{
    Declaration, IdentityReference, LogicalValue, RecordFieldSchema, RecordTypeDefinition,
    RecordValue, RecordValueField, ResolvedType, VocabularyContract, VocabularyFieldContract,
    VocabularyRecordValue, VocabularyTypeContract,
};
mod fingerprint;
pub use fingerprint::DeclarationFingerprint;
mod source_map;
pub use source_map::{SourceMap, SourceMapEntry};
mod provenance;
pub use provenance::{
    FieldProvenanceRecord, Normalization, ProvenanceRecord, ReferenceProvenanceRecord,
    ReuseProvenanceRecord, ValueOrigin,
};
mod derivation;
pub use derivation::{
    AcceptancePartition, DerivationManifest, DiagnosticPartition, MeaningPartition, ResourceFacts,
};
pub mod composition;
pub mod expression;
pub mod language;
pub mod profile;
pub mod project;
pub mod project_identity;
pub mod project_interface;

/// Fallible owned copies shared by bounded producers and independent validators.
mod fallible;

pub use profile::{
    DERIVATION_SCHEMA_VERSION, LANGUAGE_BEHAVIOR_VERSION, LOGICAL_IR_SCHEMA_VERSION,
    PROVENANCE_VERSION, SOURCE_MAP_VERSION,
};

/// A failure while constructing validated logical IR values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrError {
    /// A source numeric spelling was empty or contained non-digits.
    InvalidExactNumber,
    /// A source numeric spelling exceeded an explicit deterministic bound.
    ExactNumberLimitExceeded,
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
