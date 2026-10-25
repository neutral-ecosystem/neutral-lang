// SPDX-License-Identifier: Apache-2.0

//! Closed vocabulary contracts for Neutral.
//!
//! This crate validates already-captured, untrusted bundle bytes into immutable
//! logical contracts. It performs no filesystem, registry, environment,
//! network, dynamic-code, or other host acquisition.

mod bundle;
pub mod composition;
mod error;
mod json;
mod limits;
mod lock;
mod models;
mod project;
mod schema;
mod validation;

pub use bundle::validate_captured_bundle;
pub use error::VocabularyError;
pub(crate) use json::JsonValue;
pub use limits::{MAX_VOCABULARY_NESTING_DEPTH, VocabularyLimits};
pub use lock::VocabularyLock;
pub use models::{
    LogicalVocabulary, ValidatedVocabularyBundle, VocabularyField, VocabularyRecordValueField,
    VocabularyType, VocabularyTypeDefinition, VocabularyValue,
};

pub use project::{
    PROJECT_VOCABULARY_ENCODING_VERSION, PROJECT_VOCABULARY_SCHEMA_VERSION, ProjectVocabulary,
    ProjectVocabularyField, ProjectVocabularyType, ProjectVocabularyTypeDefinition,
    validate_project_bundle,
};

/// Frozen captured vocabulary bundle encoding version.
pub const VOCABULARY_ENCODING_VERSION: &str = schema::ENCODING_VERSION;
/// Frozen logical vocabulary schema version.
pub const VOCABULARY_SCHEMA_VERSION: &str = schema::SCHEMA_VERSION;

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
