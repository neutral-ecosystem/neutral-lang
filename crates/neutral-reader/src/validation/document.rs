// SPDX-License-Identifier: Apache-2.0

//! Coordinate consistency checks across a document and its companions.
//!
//! Recompute declaration fingerprints, reject duplicate names/element IDs, check
//! values against their contracts and require source-map/provenance coverage.
//! Specialized validators check nested schemas and evidence edges. Any mismatch
//! rejects the result before the reader publishes a validated document.

use super::{
    BTreeMap, BTreeSet, CompilationArtifacts, DeclarationFingerprint, ReaderError,
    validate_field_provenance, validate_record_schemas, validate_reference_provenance,
    validate_reuse_provenance, validate_value, validate_vocabulary_contract,
    validate_vocabulary_value,
};

/// Validates relationships among logical declarations and companion artifacts.
pub(crate) fn validate_artifacts(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    super::companions::validate_derivation(artifacts)?;
    let mut element_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let records = artifacts
        .logical_document()
        .record_types()
        .iter()
        .map(|record| (record.name(), record))
        .collect::<BTreeMap<_, _>>();
    validate_record_schemas(artifacts, &records)?;
    let logical_vocabulary = artifacts.logical_document().vocabulary();
    validate_vocabulary_contract(logical_vocabulary)?;
    for record in artifacts.logical_document().record_types() {
        if DeclarationFingerprint::for_record(record.fields())
            .map_err(|_| ReaderError::InvalidDeclarationFingerprint)?
            != record.fingerprint()
        {
            return Err(ReaderError::InvalidDeclarationFingerprint);
        }
        if !element_ids.insert(record.element_id()) {
            return Err(ReaderError::DuplicateElementId);
        }
        if !names.insert(record.name()) {
            return Err(ReaderError::DuplicateDeclarationName);
        }
        if artifacts.source_map().entry(record.element_id()).is_none() {
            return Err(ReaderError::MissingSourceMapEntry);
        }
    }
    for declaration in artifacts.logical_document().declarations() {
        if DeclarationFingerprint::for_binding(declaration.resolved_type(), declaration.value())
            .map_err(|_| ReaderError::InvalidDeclarationFingerprint)?
            != declaration.fingerprint()
        {
            return Err(ReaderError::InvalidDeclarationFingerprint);
        }
        if !validate_value(declaration.resolved_type(), declaration.value(), &records) {
            return Err(ReaderError::TypeValueMismatch);
        }
        if !validate_vocabulary_value(
            declaration.resolved_type(),
            declaration.value(),
            artifacts.logical_document().vocabulary(),
        ) {
            return Err(ReaderError::TypeValueMismatch);
        }
        if !element_ids.insert(declaration.element_id()) {
            return Err(ReaderError::DuplicateElementId);
        }
        if !names.insert(declaration.name()) {
            return Err(ReaderError::DuplicateDeclarationName);
        }
        if artifacts
            .source_map()
            .entry(declaration.element_id())
            .is_none()
        {
            return Err(ReaderError::MissingSourceMapEntry);
        }
        if !artifacts
            .provenance()
            .iter()
            .any(|record| record.element_id() == declaration.element_id())
        {
            return Err(ReaderError::MissingProvenanceRecord);
        }
    }
    validate_field_provenance(artifacts, &records)?;
    validate_reuse_provenance(artifacts, &records)?;
    validate_reference_provenance(artifacts, &records)?;
    super::companions::validate_document_companions(artifacts)?;
    Ok(())
}
