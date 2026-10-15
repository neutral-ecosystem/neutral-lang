// SPDX-License-Identifier: Apache-2.0

//! Build the envelope describing versions, producer and section integrity.
//!
//! Digests bind the envelope to the exact encoded logical and companion bytes.
//! Producer information remains envelope metadata, separate from semantic
//! identity. This module emits that section only; physical offsets and the final
//! binary layout are assembled by the frame module.

use super::{
    CborWriter, CompilationArtifacts, DERIVATION_SCHEMA_VERSION, EncodedSectionDigest,
    EncodingError, ProducerInfo, VOCABULARY_SCHEMA_VERSION, capability, constants, field_key,
    field_text, field_unsigned,
};

/// Encodes the envelope after the four hashed sections are complete.
pub(super) fn encode_envelope(
    artifacts: &CompilationArtifacts,
    producer: &ProducerInfo,
    capabilities: u64,
    hashed_sections: [&[u8]; 4],
) -> Result<Vec<u8>, EncodingError> {
    let mut writer = CborWriter::new();
    writer.map(6)?;
    field_text(&mut writer, constants::key::ENCODING, constants::ENCODING)?;
    field_unsigned(
        &mut writer,
        constants::key::FRAMING_REVISION,
        u64::from(constants::FRAMING_REVISION),
    )?;
    field_key(&mut writer, constants::key::REQUIRED_CAPABILITIES)?;
    writer.array(capabilities.count_ones() as usize)?;
    for (bit, name) in (0_u32..).zip(constants::CAPABILITY_NAMES) {
        if capabilities & capability(bit) != 0 {
            writer.text(name)?;
        }
    }
    field_key(&mut writer, constants::key::VERSIONS)?;
    encode_versions(&mut writer, artifacts)?;
    field_key(&mut writer, constants::key::PRODUCER)?;
    encode_producer(&mut writer, producer)?;
    field_key(&mut writer, constants::key::INTEGRITY)?;
    writer.array(hashed_sections.len())?;
    for (index, bytes) in hashed_sections.iter().enumerate() {
        writer.map(2)?;
        field_unsigned(
            &mut writer,
            constants::key::SECTION,
            u64::try_from(index + 2).map_err(|_| EncodingError::InternalDefect)?,
        )?;
        field_key(&mut writer, constants::key::SHA256)?;
        writer.bytes(&EncodedSectionDigest::from_bytes(bytes).as_bytes())?;
    }
    writer.finish()
}

/// Encodes the exact frozen version map.
fn encode_versions(
    writer: &mut CborWriter,
    artifacts: &CompilationArtifacts,
) -> Result<(), EncodingError> {
    let derivation = artifacts.derivation();
    writer.map(6)?;
    field_text(
        writer,
        constants::key::LANGUAGE_BEHAVIOR,
        derivation.language_behavior_version(),
    )?;
    field_text(
        writer,
        constants::key::LOGICAL_IR,
        derivation.logical_ir_schema_version(),
    )?;
    field_text(
        writer,
        constants::key::SOURCE_MAP,
        derivation.source_map_version(),
    )?;
    field_text(
        writer,
        constants::key::PROVENANCE,
        derivation.provenance_version(),
    )?;
    field_text(
        writer,
        constants::key::DERIVATION,
        DERIVATION_SCHEMA_VERSION,
    )?;
    field_text(
        writer,
        constants::key::VOCABULARY_SCHEMA,
        VOCABULARY_SCHEMA_VERSION,
    )
}

/// Encodes producer/build facts exclusively inside the envelope.
fn encode_producer(writer: &mut CborWriter, producer: &ProducerInfo) -> Result<(), EncodingError> {
    writer.map(3)?;
    field_text(writer, constants::key::NAME, producer.name())?;
    field_text(writer, constants::key::VERSION, producer.version())?;
    field_key(writer, constants::key::BUILD)?;
    match producer.build() {
        Some(build) => writer.text(build),
        None => writer.null(),
    }
}
