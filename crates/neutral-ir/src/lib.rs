// SPDX-License-Identifier: Apache-2.0

//! Public logical representation contracts for Neutral artifacts.
//!
//! This crate owns the logical IR, source maps, provenance, and derivation
//! records. It must not acquire source input, expose compiler-private models, or
//! perform host I/O.
//!
//! # Reading the model
//! Scalar/document contracts are re-exported here from focused private modules;
//! [`project`] retains complete multi-unit meaning and its separate
//! source/processing companions. [`project_interface`]
//! describes only the public signature surface, while [`project_identity`]
//! defines content-sensitive identity layers. Do not substitute an interface
//! fingerprint or graph-local label for a complete project identity.
//!
//! Public data structures let producers construct values; construction alone
//! does not establish validity. The independent reader boundary is responsible
//! for checking hostile artifacts before exposing authoritative traversal.

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
mod document;
pub use document::LogicalDocument;
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
mod artifacts;
pub use artifacts::CompilationArtifacts;

pub mod composition;
pub mod language;
pub mod project;
pub mod project_identity;
pub mod project_interface;

/// Fallible owned copies shared by bounded producers and independent validators.
mod fallible;

/// Frozen logical IR schema version for the minimal v0 artifact.
pub const LOGICAL_IR_SCHEMA_VERSION: &str = "0.1.0";
/// Frozen v0 language-behavior version shared by artifacts and producers.
pub const LANGUAGE_BEHAVIOR_VERSION: &str = "0.1.0";
/// Frozen source-map schema version for the minimal v0 artifact.
pub const SOURCE_MAP_VERSION: &str = "0.1.0";
/// Frozen provenance schema version for the minimal v0 artifact.
pub const PROVENANCE_VERSION: &str = "0.1.0";
/// Frozen derivation schema version for the minimal v0 artifact.
pub const DERIVATION_SCHEMA_VERSION: &str = "0.1.0";

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
