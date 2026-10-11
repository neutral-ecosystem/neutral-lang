// SPDX-License-Identifier: Apache-2.0

//! Encode a reader-validated document into a framed binary artifact.
//!
//! The entry point derives capabilities, emits logical and companion sections,
//! then adds the integrity envelope and frame. Producer metadata affects only
//! the envelope, not logical meaning. Section helpers enforce encoding ceilings;
//! reconstruction and hostile-input validation remain in the independent decoder.

use crate::cbor::CborWriter;
use crate::{EncodedArtifact, EncodingError, ProducerInfo, constants};
use neutral_core::{ByteSpan, EncodedSectionDigest};
use neutral_ir::{
    AcceptancePartition, CompilationArtifacts, DERIVATION_SCHEMA_VERSION, Declaration,
    DerivationManifest, DiagnosticPartition, FieldProvenanceRecord, IdentityReference,
    LogicalDocument, LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity,
    NominalTypeIdentity, ProvenanceRecord, RecordFieldSchema, RecordTypeDefinition,
    RecordValueField, ReferenceProvenanceRecord, ResolvedType, ResourceFacts,
    ReuseProvenanceRecord, SourceMap, SourceMapEntry, VocabularyContract, VocabularyFieldContract,
    VocabularyIdentity, VocabularyTypeContract, VocabularyTypeIdentity,
};
use neutral_reader::ValidatedDocument;
use neutral_vocabulary::VOCABULARY_SCHEMA_VERSION;
use std::{array, ops::Range};

pub(crate) mod capabilities;
mod companions;
mod envelope;
mod fields;
mod frame;
mod logical;

use capabilities::{TraversalBudget, capability, derive_capabilities};
use companions::{encode_derivation, encode_provenance, encode_source_map};
use envelope::encode_envelope;
use fields::{check_depth, encode_text_array, field_key, field_text, field_unsigned, kind_only};
use frame::assemble_frame;
use logical::{encode_logical, encode_vocabulary_identity};

#[cfg(test)]
#[path = "../../tests/unit/encoder_companions.rs"]
mod tests;

/// Encodes one already validated document into Neutral IR Framed CBOR 0.1.
///
/// The function borrows and never mutates the input. Only envelope bytes depend
/// on `producer`; the logical and companion section projections do not.
///
/// # Errors
///
/// Returns [`EncodingError::EncodedSizeLimit`] if a frozen encoding ceiling is
/// exceeded, or [`EncodingError::InternalDefect`] if validated input contradicts
/// an encoder invariant.
pub fn encode(
    document: &ValidatedDocument,
    producer: &ProducerInfo,
) -> Result<EncodedArtifact, EncodingError> {
    let artifacts = document.artifacts().as_ref();
    let mut budget = TraversalBudget::new();
    let capabilities = derive_capabilities(artifacts, &mut budget)?;
    let logical = encode_logical(artifacts.logical_document())?;
    let source_map = encode_source_map(artifacts.source_map())?;
    let provenance = encode_provenance(artifacts)?;
    let derivation = encode_derivation(artifacts.derivation())?;
    let envelope = encode_envelope(
        artifacts,
        producer,
        capabilities,
        [&logical, &source_map, &provenance, &derivation],
    )?;
    assemble_frame(
        capabilities,
        [envelope, logical, source_map, provenance, derivation],
    )
}
