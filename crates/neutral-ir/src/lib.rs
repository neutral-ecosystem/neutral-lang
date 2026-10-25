// SPDX-License-Identifier: Apache-2.0

//! Shared scalar, nominal, source and raw `/3` expression IR contracts.
//!
//! Producers may construct these data structures; construction alone does not
//! establish validity. The independent reader checks hostile complete artifacts
//! before exposing authoritative traversal. This crate owns no acquisition,
//! compiler parser state, host I/O or evaluation effects.

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
