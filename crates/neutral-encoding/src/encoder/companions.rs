// SPDX-License-Identifier: Apache-2.0

//! Serialize the nonlogical sections accompanying a document.
//!
//! Source maps preserve original-byte spans; provenance distinguishes origins,
//! reuse and references; derivation retains processing inputs and resource facts.
//! This module projects existing records into the wire schema. It does not infer
//! missing evidence or merge companion data into the logical section.

use super::{
    AcceptancePartition, ByteSpan, CborWriter, CompilationArtifacts, DERIVATION_SCHEMA_VERSION,
    DerivationManifest, DiagnosticPartition, EncodingError, FieldProvenanceRecord,
    ProvenanceRecord, ReferenceProvenanceRecord, ResourceFacts, ReuseProvenanceRecord, SourceMap,
    SourceMapEntry, constants, encode_text_array, encode_vocabulary_identity, field_key,
    field_text, field_unsigned,
};

/// Encodes the complete source-map section.
pub(super) fn encode_source_map(source_map: &SourceMap) -> Result<Vec<u8>, EncodingError> {
    let mut writer = CborWriter::new();
    writer.map(5)?;
    field_text(
        &mut writer,
        constants::key::SOURCE_MAP_VERSION,
        neutral_ir::SOURCE_MAP_VERSION,
    )?;
    field_key(&mut writer, constants::key::SOURCE_DIGEST)?;
    writer.bytes(&source_map.source_digest().as_bytes())?;
    field_unsigned(
        &mut writer,
        constants::key::SOURCE_BYTE_LENGTH,
        source_map.source_byte_length(),
    )?;
    field_key(&mut writer, constants::key::MODULE_SPAN)?;
    encode_span(&mut writer, source_map.module_span())?;
    field_key(&mut writer, constants::key::ENTRIES)?;
    writer.array(source_map.entries().len())?;
    for entry in source_map.entries() {
        encode_source_map_entry(&mut writer, *entry)?;
    }
    writer.finish()
}

/// Encodes one half-open byte span.
fn encode_span(writer: &mut CborWriter, span: ByteSpan) -> Result<(), EncodingError> {
    writer.map(2)?;
    field_unsigned(writer, constants::key::START, span.start())?;
    field_unsigned(writer, constants::key::END, span.end())
}

/// Encodes one complete source-map entry.
fn encode_source_map_entry(
    writer: &mut CborWriter,
    entry: SourceMapEntry,
) -> Result<(), EncodingError> {
    writer.map(5)?;
    field_unsigned(writer, constants::key::ELEMENT_ID, entry.element_id().get())?;
    field_key(writer, constants::key::DECLARATION_SPAN)?;
    encode_span(writer, entry.declaration_span())?;
    field_key(writer, constants::key::TYPE_SPAN)?;
    encode_span(writer, entry.type_span())?;
    field_key(writer, constants::key::NAME_SPAN)?;
    encode_span(writer, entry.name_span())?;
    field_key(writer, constants::key::VALUE_SPAN)?;
    encode_span(writer, entry.value_span())
}

/// Encodes the complete provenance section.
pub(super) fn encode_provenance(
    artifacts: &CompilationArtifacts,
) -> Result<Vec<u8>, EncodingError> {
    let mut writer = CborWriter::new();
    writer.map(5)?;
    field_text(
        &mut writer,
        constants::key::PROVENANCE_VERSION,
        neutral_ir::PROVENANCE_VERSION,
    )?;
    field_key(&mut writer, constants::key::VALUES)?;
    writer.array(artifacts.provenance().len())?;
    for record in artifacts.provenance() {
        encode_value_provenance(&mut writer, *record)?;
    }
    field_key(&mut writer, constants::key::FIELDS)?;
    writer.array(artifacts.field_provenance().len())?;
    for record in artifacts.field_provenance() {
        encode_field_provenance(&mut writer, record)?;
    }
    field_key(&mut writer, constants::key::REUSES)?;
    writer.array(artifacts.reuse_provenance().len())?;
    for record in artifacts.reuse_provenance() {
        encode_reuse_provenance(&mut writer, record)?;
    }
    field_key(&mut writer, constants::key::REFERENCES)?;
    writer.array(artifacts.reference_provenance().len())?;
    for record in artifacts.reference_provenance() {
        encode_reference_provenance(&mut writer, record)?;
    }
    writer.finish()
}

/// Encodes one root value-provenance record.
fn encode_value_provenance(
    writer: &mut CborWriter,
    record: ProvenanceRecord,
) -> Result<(), EncodingError> {
    writer.map(3)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        record.element_id().get(),
    )?;
    field_text(writer, constants::key::ORIGIN, record.origin().as_str())?;
    field_text(
        writer,
        constants::key::NORMALIZATION,
        record.normalization().as_str(),
    )
}

/// Encodes one field-provenance record.
fn encode_field_provenance(
    writer: &mut CborWriter,
    record: &FieldProvenanceRecord,
) -> Result<(), EncodingError> {
    writer.map(3)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        record.element_id().get(),
    )?;
    field_key(writer, constants::key::FIELD_PATH)?;
    encode_text_array(writer, record.field_path())?;
    field_text(writer, constants::key::ORIGIN, record.origin().as_str())
}

/// Encodes one ordinary-reuse provenance record.
fn encode_reuse_provenance(
    writer: &mut CborWriter,
    record: &ReuseProvenanceRecord,
) -> Result<(), EncodingError> {
    writer.map(3)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        record.element_id().get(),
    )?;
    field_key(writer, constants::key::VALUE_PATH)?;
    encode_text_array(writer, record.value_path())?;
    field_unsigned(
        writer,
        constants::key::SOURCE_ELEMENT_ID,
        record.source_element_id().get(),
    )
}

/// Encodes one reference-provenance record.
fn encode_reference_provenance(
    writer: &mut CborWriter,
    record: &ReferenceProvenanceRecord,
) -> Result<(), EncodingError> {
    writer.map(3)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        record.element_id().get(),
    )?;
    field_key(writer, constants::key::VALUE_PATH)?;
    encode_text_array(writer, record.value_path())?;
    field_unsigned(
        writer,
        constants::key::TARGET_ELEMENT_ID,
        record.target_element_id().get(),
    )
}

/// Encodes the complete derivation section.
pub(super) fn encode_derivation(derivation: &DerivationManifest) -> Result<Vec<u8>, EncodingError> {
    let mut writer = CborWriter::new();
    writer.map(10)?;
    field_text(
        &mut writer,
        constants::key::DERIVATION_SCHEMA_VERSION,
        DERIVATION_SCHEMA_VERSION,
    )?;
    field_text(
        &mut writer,
        constants::key::LANGUAGE_BEHAVIOR_VERSION,
        derivation.language_behavior_version(),
    )?;
    field_text(
        &mut writer,
        constants::key::LOGICAL_IR_SCHEMA_VERSION,
        derivation.logical_ir_schema_version(),
    )?;
    field_text(
        &mut writer,
        constants::key::SOURCE_MAP_VERSION,
        derivation.source_map_version(),
    )?;
    field_text(
        &mut writer,
        constants::key::PROVENANCE_VERSION,
        derivation.provenance_version(),
    )?;
    field_key(&mut writer, constants::key::MEANING)?;
    writer.map(1)?;
    field_key(&mut writer, constants::key::SOURCE_DIGEST)?;
    writer.bytes(&derivation.meaning().source_digest().as_bytes())?;
    field_key(&mut writer, constants::key::ACCEPTANCE)?;
    encode_acceptance(&mut writer, derivation.acceptance())?;
    field_key(&mut writer, constants::key::DIAGNOSTIC_POLICY)?;
    encode_diagnostic_policy(&mut writer, derivation.diagnostics())?;
    field_key(&mut writer, constants::key::RESOURCE_FACTS)?;
    encode_resource_facts(&mut writer, derivation.resource_facts())?;
    field_key(&mut writer, constants::key::VOCABULARY)?;
    match derivation.vocabulary() {
        Some(identity) => encode_vocabulary_identity(&mut writer, identity)?,
        None => writer.null()?,
    }
    writer.finish()
}

/// Encodes captured acceptance limits.
fn encode_acceptance(
    writer: &mut CborWriter,
    acceptance: AcceptancePartition,
) -> Result<(), EncodingError> {
    writer.map(10)?;
    field_unsigned(
        writer,
        constants::key::SOURCE_BYTES,
        acceptance.source_byte_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::DIAGNOSTICS,
        u64::from(acceptance.diagnostic_limit()),
    )?;
    field_unsigned(
        writer,
        constants::key::STRING_BYTES,
        acceptance.string_byte_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::NUMERIC_DIGITS,
        acceptance.numeric_digit_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::NUMERIC_SCALE,
        acceptance.numeric_scale_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::DECLARATIONS,
        acceptance.declaration_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::RECORD_FIELDS,
        acceptance.record_field_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::NESTING_DEPTH,
        acceptance.nesting_depth_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::LIST_ITEMS,
        acceptance.list_item_limit(),
    )?;
    field_unsigned(
        writer,
        constants::key::TRAVERSAL_NODES,
        acceptance.traversal_node_limit(),
    )
}

/// Encodes captured diagnostic policy.
fn encode_diagnostic_policy(
    writer: &mut CborWriter,
    diagnostics: DiagnosticPartition,
) -> Result<(), EncodingError> {
    writer.map(1)?;
    field_key(writer, constants::key::SAFE_BOUNDED_OUTPUT)?;
    writer.boolean(diagnostics.safe_bounded_output())
}

/// Encodes observed resource facts.
fn encode_resource_facts(
    writer: &mut CborWriter,
    facts: ResourceFacts,
) -> Result<(), EncodingError> {
    writer.map(4)?;
    field_unsigned(writer, constants::key::SOURCE_BYTES, facts.source_bytes())?;
    field_unsigned(writer, constants::key::DECLARATIONS, facts.declarations())?;
    field_unsigned(writer, constants::key::DIAGNOSTICS, facts.diagnostics())?;
    field_unsigned(
        writer,
        constants::key::DECODED_STRING_BYTES,
        facts.decoded_string_bytes(),
    )
}
