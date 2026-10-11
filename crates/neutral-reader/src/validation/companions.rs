// SPDX-License-Identifier: Apache-2.0

//! Shared model-level validation for in-process and independently decoded documents.
//! Wire parsing stays in the encoding crate; companion authority is checked here.

use super::{
    BTreeSet, CompilationArtifacts, ReaderError, RecordTypeDefinition, VocabularyContract,
};
use neutral_ir::{Declaration, LogicalDocument};

/// Validates cross-section facts before semantic checks, preserving wire failure classification.
pub(super) fn validate_derivation(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    let logical = artifacts.logical_document();
    let source = artifacts.source_map();
    let derivation = artifacts.derivation();
    if derivation.meaning().source_digest() != source.source_digest()
        || derivation.language_behavior_version() != logical.module().language_behavior_version()
        || derivation.vocabulary() != logical.vocabulary().map(VocabularyContract::identity)
        || derivation.resource_facts().source_bytes() != source.source_byte_length()
        || derivation.resource_facts().diagnostics() != 0
        || derivation.resource_facts().declarations()
            != u64::try_from(logical.record_types().len() + logical.declarations().len())
                .unwrap_or(u64::MAX)
        || source.source_byte_length() > derivation.acceptance().source_byte_limit()
        || derivation.resource_facts().decoded_string_bytes()
            > derivation.acceptance().string_byte_limit()
    {
        return Err(ReaderError::InvalidDerivation);
    }
    Ok(())
}

/// Checks document companion consistency without conferring validated reader authority.
///
/// Codecs can run this before format-specific capability checks to preserve error
/// precedence. Full reader construction still validates semantics and these facts.
///
/// # Errors
///
/// Returns an error for inconsistent derivation, logical identities, source-map
/// spans/coverage or provenance coverage/ordering.
pub fn validate_document_companions(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    validate_derivation(artifacts)?;
    validate_logical_identities(artifacts.logical_document())?;
    validate_source_coverage(artifacts)?;
    validate_provenance_coverage(artifacts)?;
    Ok(())
}

/// Validates root module, symbol, identity, and canonical-name consistency.
fn validate_logical_identities(logical: &LogicalDocument) -> Result<(), ReaderError> {
    let module = logical.module();
    let mut previous_record = None;
    for record in logical.record_types() {
        if record.symbol_identity().module() != module
            || record.nominal_identity().module() != module
            || record.symbol_identity().declaration_name() != record.name()
            || previous_record.is_some_and(|name| name >= record.name())
        {
            return Err(ReaderError::InvalidLogicalIdentity);
        }
        previous_record = Some(record.name());
    }
    let mut previous_declaration = None;
    for declaration in logical.declarations() {
        if declaration.symbol_identity().module() != module
            || declaration.symbol_identity().declaration_name() != declaration.name()
            || previous_declaration.is_some_and(|name| name >= declaration.name())
        {
            return Err(ReaderError::InvalidLogicalIdentity);
        }
        previous_declaration = Some(declaration.name());
    }
    Ok(())
}

/// Validates exact one-to-one source-map ownership and span coverage.
fn validate_source_coverage(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    let source = artifacts.source_map();
    if source.module_span().end() > source.source_byte_length()
        || source.entries().iter().any(|entry| {
            let declaration = entry.declaration_span();
            declaration.end() > source.source_byte_length()
                || [entry.type_span(), entry.name_span(), entry.value_span()]
                    .iter()
                    .any(|span| {
                        span.start() < declaration.start() || span.end() > declaration.end()
                    })
        })
    {
        return Err(ReaderError::InvalidSourceMap);
    }
    let logical = artifacts.logical_document();
    let expected = logical
        .record_types()
        .iter()
        .map(RecordTypeDefinition::element_id)
        .chain(logical.declarations().iter().map(Declaration::element_id))
        .collect::<BTreeSet<_>>();
    let observed = artifacts
        .source_map()
        .entries()
        .iter()
        .map(|entry| entry.element_id())
        .collect::<BTreeSet<_>>();
    if expected.len() != artifacts.source_map().entries().len()
        || expected != observed
        || !artifacts
            .source_map()
            .entries()
            .windows(2)
            .all(|pair| pair[0].element_id() < pair[1].element_id())
    {
        return Err(ReaderError::InvalidSourceMap);
    }
    Ok(())
}

/// Validates exact root coverage and canonical ordering for every provenance array.
fn validate_provenance_coverage(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    let expected = artifacts
        .logical_document()
        .declarations()
        .iter()
        .map(Declaration::element_id)
        .collect::<BTreeSet<_>>();
    let observed = artifacts
        .provenance()
        .iter()
        .map(|record| record.element_id())
        .collect::<BTreeSet<_>>();
    let values_ordered = artifacts
        .provenance()
        .windows(2)
        .all(|pair| pair[0].element_id() < pair[1].element_id());
    let fields_ordered = artifacts.field_provenance().windows(2).all(|pair| {
        (pair[0].element_id(), pair[0].field_path()) < (pair[1].element_id(), pair[1].field_path())
    });
    let reuses_ordered = artifacts.reuse_provenance().windows(2).all(|pair| {
        (
            pair[0].element_id(),
            pair[0].value_path(),
            pair[0].source_element_id(),
        ) < (
            pair[1].element_id(),
            pair[1].value_path(),
            pair[1].source_element_id(),
        )
    });
    let references_ordered = artifacts.reference_provenance().windows(2).all(|pair| {
        (
            pair[0].element_id(),
            pair[0].value_path(),
            pair[0].target_element_id(),
        ) < (
            pair[1].element_id(),
            pair[1].value_path(),
            pair[1].target_element_id(),
        )
    });
    if expected.len() != artifacts.provenance().len()
        || expected != observed
        || !values_ordered
        || !fields_ordered
        || !reuses_ordered
        || !references_ordered
    {
        return Err(ReaderError::InvalidProvenance);
    }
    Ok(())
}
