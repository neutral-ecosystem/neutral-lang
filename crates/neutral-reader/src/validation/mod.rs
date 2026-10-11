// SPDX-License-Identifier: Apache-2.0

//! Validate document artifacts without relying on compiler-private state.
//!
//! The document pass coordinates schema/value checks and companion checks before
//! the public reader exposes validated authority. Separate modules own vocabulary
//! contracts, record/value compatibility and provenance completeness. An encoded
//! or producer-constructed object is not trusted merely because its fields exist.

use crate::{
    CompilationArtifacts, DeclarationFingerprint, ElementId, LogicalValue, ReaderError,
    RecordTypeDefinition, ResolvedType, ValueOrigin, VocabularyContract,
};
use std::collections::{BTreeMap, BTreeSet};

mod companions;
mod document;
mod provenance;
mod schema;
mod value;

pub use companions::validate_document_companions;
pub(super) use document::validate_artifacts;
use provenance::{
    validate_field_provenance, validate_reference_provenance, validate_reuse_provenance,
};
use schema::{validate_vocabulary_contract, validate_vocabulary_value};
use value::{is_closed_default, validate_record_schemas, validate_value};
