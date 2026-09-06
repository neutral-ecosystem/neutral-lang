// SPDX-License-Identifier: Apache-2.0

//! External encoding and hostile decoding of validated Neutral documents.
//!
//! This codec implements Neutral IR Framed CBOR 0.1. The encoder accepts only
//! trusted reader views; the bounds-first decoder returns one only after full
//! validation. Deterministic emitted field order is an implementation property:
//! encoded bytes are not logical identity, and producer/build facts remain
//! confined to the envelope.

mod cbor;
pub mod constants;
mod decode;
mod decoder;
pub mod diagnostics;

use cbor::CborWriter;
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

pub use decode::decode;

/// Complete bounded result class for hostile external artifact decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeErrorClass {
    /// An artifact, section, string, container, depth, or traversal limit failed.
    EncodedSizeLimit,
    /// Fixed framing or directory structure was malformed.
    MalformedFrame,
    /// A fixed framing, section, or contract version is unsupported.
    UnsupportedVersion,
    /// Capability declarations are unsupported or inconsistent with content.
    UnsupportedCapability,
    /// Exact section integrity verification failed.
    IntegrityMismatch,
    /// Restricted-CBOR lexical/container validation failed.
    MalformedCbor,
    /// A closed encoded schema was malformed.
    InvalidEncodedSchema,
    /// Logical IR validation failed.
    InvalidLogicalIr,
    /// Source-map validation failed.
    InvalidSourceMap,
    /// Provenance validation failed.
    InvalidProvenance,
    /// Derivation or cross-section validation failed.
    InvalidDerivation,
    /// Cooperative caller cancellation was observed.
    Cancelled,
    /// An implementation invariant failed independently of input validity.
    InternalDefect,
}

impl DecodeErrorClass {
    /// Returns the stable bounded diagnostic code for this result class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EncodedSizeLimit => diagnostics::ENCODED_SIZE_LIMIT,
            Self::MalformedFrame => diagnostics::MALFORMED_FRAME,
            Self::UnsupportedVersion => diagnostics::UNSUPPORTED_VERSION,
            Self::UnsupportedCapability => diagnostics::UNSUPPORTED_CAPABILITY,
            Self::IntegrityMismatch => diagnostics::INTEGRITY_MISMATCH,
            Self::MalformedCbor => diagnostics::MALFORMED_CBOR,
            Self::InvalidEncodedSchema => diagnostics::INVALID_ENCODED_SCHEMA,
            Self::InvalidLogicalIr => diagnostics::INVALID_LOGICAL_IR,
            Self::InvalidSourceMap => diagnostics::INVALID_SOURCE_MAP,
            Self::InvalidProvenance => diagnostics::INVALID_PROVENANCE,
            Self::InvalidDerivation => diagnostics::INVALID_DERIVATION,
            Self::Cancelled => diagnostics::CANCELLED,
            Self::InternalDefect => diagnostics::INTERNAL_DEFECT,
        }
    }
}

/// Bounded safe decoder failure with an optional encoded-byte offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeError {
    /// Stable failure class.
    class: DecodeErrorClass,
    /// Absolute byte offset when one is available.
    offset: Option<u64>,
}

impl DecodeError {
    /// Constructs one bounded internal decoder error.
    const fn new(class: DecodeErrorClass, offset: Option<u64>) -> Self {
        Self { class, offset }
    }

    /// Returns the stable result class.
    #[must_use]
    pub const fn class(self) -> DecodeErrorClass {
        self.class
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.class.code()
    }

    /// Returns the absolute encoded-byte offset when available.
    #[must_use]
    pub const fn offset(self) -> Option<u64> {
        self.offset
    }
}

/// Host-configurable decoder ceilings, always clamped to frozen hard limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeLimits {
    /// Complete artifact bytes.
    artifact_bytes: usize,
    /// Bytes in any one section.
    section_bytes: usize,
    /// Nested CBOR/logical containers.
    nesting_depth: usize,
    /// Items in one container.
    container_items: usize,
    /// Bytes in one text string.
    text_bytes: usize,
    /// Bytes in one byte string.
    byte_string_bytes: usize,
    /// Total traversed nodes.
    traversal_nodes: usize,
}

impl DecodeLimits {
    /// Returns the immutable frozen hard ceilings.
    #[must_use]
    pub const fn hard() -> Self {
        Self {
            artifact_bytes: constants::MAXIMUM_ARTIFACT_BYTES,
            section_bytes: constants::MAXIMUM_SECTION_BYTES,
            nesting_depth: constants::MAXIMUM_NESTING_DEPTH,
            container_items: constants::MAXIMUM_CONTAINER_ITEMS,
            text_bytes: constants::MAXIMUM_TEXT_BYTES,
            byte_string_bytes: constants::MAXIMUM_BYTE_STRING_BYTES,
            traversal_nodes: constants::MAXIMUM_TRAVERSAL_NODES,
        }
    }

    /// Applies a lower host artifact-byte ceiling; zero rejects every artifact.
    #[must_use]
    pub const fn with_artifact_bytes(mut self, value: usize) -> Self {
        self.artifact_bytes = min_usize(self.artifact_bytes, value);
        self
    }

    /// Applies a lower host section-byte ceiling.
    #[must_use]
    pub const fn with_section_bytes(mut self, value: usize) -> Self {
        self.section_bytes = min_usize(self.section_bytes, value);
        self
    }

    /// Applies a lower host nesting-depth ceiling.
    #[must_use]
    pub const fn with_nesting_depth(mut self, value: usize) -> Self {
        self.nesting_depth = min_usize(self.nesting_depth, value);
        self
    }

    /// Applies a lower host per-container item ceiling.
    #[must_use]
    pub const fn with_container_items(mut self, value: usize) -> Self {
        self.container_items = min_usize(self.container_items, value);
        self
    }

    /// Applies a lower host text-string byte ceiling.
    #[must_use]
    pub const fn with_text_bytes(mut self, value: usize) -> Self {
        self.text_bytes = min_usize(self.text_bytes, value);
        self
    }

    /// Applies a lower host byte-string ceiling.
    #[must_use]
    pub const fn with_byte_string_bytes(mut self, value: usize) -> Self {
        self.byte_string_bytes = min_usize(self.byte_string_bytes, value);
        self
    }

    /// Applies a lower host total traversal ceiling.
    #[must_use]
    pub const fn with_traversal_nodes(mut self, value: usize) -> Self {
        self.traversal_nodes = min_usize(self.traversal_nodes, value);
        self
    }

    /// Returns the effective artifact-byte ceiling.
    pub(crate) const fn maximum_artifact_bytes(self) -> usize {
        self.artifact_bytes
    }
    /// Returns the effective per-section byte ceiling.
    pub(crate) const fn maximum_section_bytes(self) -> usize {
        self.section_bytes
    }
    /// Returns the effective nesting-depth ceiling.
    pub(crate) const fn maximum_nesting_depth(self) -> usize {
        self.nesting_depth
    }
    /// Returns the effective per-container item ceiling.
    pub(crate) const fn maximum_container_items(self) -> usize {
        self.container_items
    }
    /// Returns the effective text-string byte ceiling.
    pub(crate) const fn maximum_text_bytes(self) -> usize {
        self.text_bytes
    }
    /// Returns the effective byte-string ceiling.
    pub(crate) const fn maximum_byte_string_bytes(self) -> usize {
        self.byte_string_bytes
    }
    /// Returns the effective traversal ceiling.
    pub(crate) const fn maximum_traversal_nodes(self) -> usize {
        self.traversal_nodes
    }
}

/// Returns the smaller of two `usize` values in constant contexts.
const fn min_usize(left: usize, right: usize) -> usize {
    if left < right { left } else { right }
}

/// Envelope-only facts describing the implementation that produced an artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerInfo {
    /// Producer name.
    name: String,
    /// Producer version.
    version: String,
    /// Optional producer build identity.
    build: Option<String>,
}

impl ProducerInfo {
    /// Creates producer facts without a build identity.
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            build: None,
        }
    }

    /// Attaches an envelope-only build identity.
    #[must_use]
    pub fn with_build(mut self, build: impl Into<String>) -> Self {
        self.build = Some(build.into());
        self
    }

    /// Returns the producer name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the producer version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the optional producer build identity.
    #[must_use]
    pub fn build(&self) -> Option<&str> {
        self.build.as_deref()
    }
}

/// One fixed external section kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SectionKind {
    /// Encoding and producer envelope.
    Envelope = 1,
    /// Authoritative logical payload.
    LogicalPayload = 2,
    /// Original-byte source map.
    SourceMap = 3,
    /// Complete provenance companions.
    Provenance = 4,
    /// Complete derivation companions.
    Derivation = 5,
}

impl SectionKind {
    /// Returns the zero-based section position in the fixed directory.
    const fn index(self) -> usize {
        self as usize - 1
    }
}

/// Complete immutable encoded Neutral artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedArtifact {
    /// Complete framed bytes.
    bytes: Vec<u8>,
    /// Exact byte range for each fixed section kind.
    sections: [Range<usize>; constants::SECTION_COUNT],
}

impl EncodedArtifact {
    /// Returns the complete framed artifact bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the exact restricted-CBOR bytes for one section.
    #[must_use]
    pub fn section_bytes(&self, kind: SectionKind) -> &[u8] {
        &self.bytes[self.sections[kind.index()].clone()]
    }

    /// Consumes the wrapper and returns the complete frame.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A bounded failure while projecting validated memory into external bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingError {
    /// An immutable external encoding ceiling was exceeded.
    EncodedSizeLimit,
    /// Validated input contradicted an encoder-only structural invariant.
    InternalDefect,
}

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

/// Tracks the hard recursive traversal ceiling while inspecting capabilities.
struct TraversalBudget {
    /// Logical nodes visited so far.
    nodes: usize,
}

impl TraversalBudget {
    /// Creates an empty traversal budget.
    const fn new() -> Self {
        Self { nodes: 0 }
    }

    /// Accounts for one node and validates its nesting depth.
    fn visit(&mut self, depth: usize) -> Result<(), EncodingError> {
        if depth > constants::MAXIMUM_NESTING_DEPTH {
            return Err(EncodingError::EncodedSizeLimit);
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or(EncodingError::EncodedSizeLimit)?;
        if self.nodes > constants::MAXIMUM_TRAVERSAL_NODES {
            return Err(EncodingError::EncodedSizeLimit);
        }
        Ok(())
    }
}

/// Derives the exact capability mask from validated content.
fn derive_capabilities(
    artifacts: &CompilationArtifacts,
    budget: &mut TraversalBudget,
) -> Result<u64, EncodingError> {
    let document = artifacts.logical_document();
    let mut mask = 0_u64;
    if document.vocabulary().is_some() {
        mask |= capability(5);
    }
    for record in document.record_types() {
        mask |= capability(2);
        for field in record.fields() {
            collect_type_capabilities(field.resolved_type(), 1, budget, &mut mask)?;
            if let Some(value) = field.default_value() {
                collect_value_capabilities(value, 1, budget, &mut mask)?;
            }
        }
    }
    if let Some(vocabulary) = document.vocabulary() {
        for contract in vocabulary.types() {
            for field in contract.fields() {
                collect_type_capabilities(field.resolved_type(), 1, budget, &mut mask)?;
                if let Some(value) = field.default_value() {
                    collect_value_capabilities(value, 1, budget, &mut mask)?;
                }
            }
        }
    }
    for declaration in document.declarations() {
        collect_type_capabilities(declaration.resolved_type(), 1, budget, &mut mask)?;
        collect_value_capabilities(declaration.value(), 1, budget, &mut mask)?;
    }
    if !artifacts.reference_provenance().is_empty() {
        mask |= capability(4);
    }
    if !artifacts.reuse_provenance().is_empty() {
        mask |= capability(6);
    }
    if artifacts.field_provenance().iter().any(|record| {
        matches!(
            record.origin(),
            neutral_ir::ValueOrigin::UserRecordDefault | neutral_ir::ValueOrigin::VocabularyDefault
        )
    }) {
        mask |= capability(7);
    }
    Ok(mask)
}

/// Returns one capability bit.
const fn capability(bit: u32) -> u64 {
    1_u64 << bit
}

/// Collects capability bits from one resolved type.
fn collect_type_capabilities(
    resolved_type: &ResolvedType,
    depth: usize,
    budget: &mut TraversalBudget,
    mask: &mut u64,
) -> Result<(), EncodingError> {
    budget.visit(depth)?;
    match resolved_type {
        ResolvedType::Num => *mask |= capability(0),
        ResolvedType::Record(_) => *mask |= capability(2),
        ResolvedType::VocabularyRecord(_) => *mask |= capability(5),
        ResolvedType::List(inner) => {
            *mask |= capability(3);
            collect_type_capabilities(inner, depth + 1, budget, mask)?;
        }
        ResolvedType::Ref(inner) => {
            *mask |= capability(4);
            collect_type_capabilities(inner, depth + 1, budget, mask)?;
        }
        ResolvedType::Nullable(inner) => {
            *mask |= capability(1);
            collect_type_capabilities(inner, depth + 1, budget, mask)?;
        }
        ResolvedType::String | ResolvedType::Bool => {}
    }
    Ok(())
}

/// Collects capability bits from one logical value.
fn collect_value_capabilities(
    value: &LogicalValue,
    depth: usize,
    budget: &mut TraversalBudget,
    mask: &mut u64,
) -> Result<(), EncodingError> {
    budget.visit(depth)?;
    match value {
        LogicalValue::Number(_) => *mask |= capability(0),
        LogicalValue::Null => *mask |= capability(1),
        LogicalValue::Record(record) => {
            *mask |= capability(2);
            for field in record.fields() {
                collect_value_capabilities(field.value(), depth + 1, budget, mask)?;
            }
        }
        LogicalValue::VocabularyRecord(record) => {
            *mask |= capability(5);
            for field in record.fields() {
                collect_value_capabilities(field.value(), depth + 1, budget, mask)?;
            }
        }
        LogicalValue::List(items) => {
            *mask |= capability(3);
            for item in items {
                collect_value_capabilities(item, depth + 1, budget, mask)?;
            }
        }
        LogicalValue::Reference(reference) => {
            *mask |= capability(4);
            collect_type_capabilities(reference.target_type(), depth + 1, budget, mask)?;
        }
        LogicalValue::String(_) | LogicalValue::Boolean(_) => {}
    }
    Ok(())
}

/// Encodes the complete logical payload section.
fn encode_logical(document: &LogicalDocument) -> Result<Vec<u8>, EncodingError> {
    let mut writer = CborWriter::new();
    writer.map(5)?;
    field_text(
        &mut writer,
        constants::key::LOGICAL_IR_SCHEMA_VERSION,
        neutral_ir::LOGICAL_IR_SCHEMA_VERSION,
    )?;
    field_key(&mut writer, constants::key::MODULE)?;
    encode_module(&mut writer, document.module())?;
    field_key(&mut writer, constants::key::RECORD_TYPES)?;
    writer.array(document.record_types().len())?;
    for record in document.record_types() {
        encode_record_type(&mut writer, record, 3)?;
    }
    field_key(&mut writer, constants::key::VOCABULARY)?;
    encode_optional_vocabulary_contract(&mut writer, document.vocabulary(), 2)?;
    field_key(&mut writer, constants::key::DECLARATIONS)?;
    writer.array(document.declarations().len())?;
    for declaration in document.declarations() {
        encode_declaration(&mut writer, declaration, 3)?;
    }
    writer.finish()
}

/// Encodes one logical module identity.
fn encode_module(
    writer: &mut CborWriter,
    module: &LogicalModuleIdentity,
) -> Result<(), EncodingError> {
    writer.map(2)?;
    field_text(
        writer,
        constants::key::LANGUAGE_BEHAVIOR_VERSION,
        module.language_behavior_version(),
    )?;
    field_text(writer, constants::key::NAME, module.module_name())
}

/// Encodes one module-symbol identity.
fn encode_symbol(
    writer: &mut CborWriter,
    symbol: &ModuleSymbolIdentity,
) -> Result<(), EncodingError> {
    writer.map(2)?;
    field_key(writer, constants::key::MODULE)?;
    encode_module(writer, symbol.module())?;
    field_text(writer, constants::key::NAME, symbol.declaration_name())
}

/// Encodes one nominal record identity.
fn encode_nominal(
    writer: &mut CborWriter,
    identity: &NominalTypeIdentity,
) -> Result<(), EncodingError> {
    writer.map(2)?;
    field_key(writer, constants::key::MODULE)?;
    encode_module(writer, identity.module())?;
    field_text(writer, constants::key::NAME, identity.name())
}

/// Encodes one vocabulary-owned type identity.
fn encode_vocabulary_type(
    writer: &mut CborWriter,
    identity: &VocabularyTypeIdentity,
) -> Result<(), EncodingError> {
    writer.map(2)?;
    field_text(writer, constants::key::VOCABULARY, identity.vocabulary())?;
    field_text(writer, constants::key::NAME, identity.name())
}

/// Encodes one record type definition.
fn encode_record_type(
    writer: &mut CborWriter,
    record: &RecordTypeDefinition,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    writer.map(5)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        record.element_id().get(),
    )?;
    field_key(writer, constants::key::SYMBOL)?;
    check_depth(depth + 2)?;
    encode_symbol(writer, record.symbol_identity())?;
    field_key(writer, constants::key::FINGERPRINT)?;
    writer.bytes(&record.fingerprint().digest().as_bytes())?;
    field_key(writer, constants::key::IDENTITY)?;
    check_depth(depth + 1)?;
    encode_nominal(writer, record.nominal_identity())?;
    field_key(writer, constants::key::FIELDS)?;
    writer.array(record.fields().len())?;
    for field in record.fields() {
        encode_record_field(writer, field, depth + 2)?;
    }
    Ok(())
}

/// Encodes one record field schema.
fn encode_record_field(
    writer: &mut CborWriter,
    field: &RecordFieldSchema,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    writer.map(3)?;
    field_text(writer, constants::key::NAME, field.name())?;
    field_key(writer, constants::key::TYPE)?;
    encode_type(writer, field.resolved_type(), depth + 1)?;
    field_key(writer, constants::key::DEFAULT)?;
    encode_optional_value(writer, field.default_value(), depth + 1)
}

/// Encodes one declaration.
fn encode_declaration(
    writer: &mut CborWriter,
    declaration: &Declaration,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    writer.map(6)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        declaration.element_id().get(),
    )?;
    field_key(writer, constants::key::SYMBOL)?;
    check_depth(depth + 2)?;
    encode_symbol(writer, declaration.symbol_identity())?;
    field_key(writer, constants::key::FINGERPRINT)?;
    writer.bytes(&declaration.fingerprint().digest().as_bytes())?;
    field_text(writer, constants::key::NAME, declaration.name())?;
    field_key(writer, constants::key::TYPE)?;
    encode_type(writer, declaration.resolved_type(), depth + 1)?;
    field_key(writer, constants::key::VALUE)?;
    encode_value(writer, declaration.value(), depth + 1)
}

/// Encodes one optional vocabulary contract.
fn encode_optional_vocabulary_contract(
    writer: &mut CborWriter,
    vocabulary: Option<&VocabularyContract>,
    depth: usize,
) -> Result<(), EncodingError> {
    match vocabulary {
        Some(vocabulary) => encode_vocabulary_contract(writer, vocabulary, depth),
        None => writer.null(),
    }
}

/// Encodes one complete vocabulary contract.
fn encode_vocabulary_contract(
    writer: &mut CborWriter,
    vocabulary: &VocabularyContract,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    writer.map(2)?;
    field_key(writer, constants::key::IDENTITY)?;
    check_depth(depth + 1)?;
    encode_vocabulary_identity(writer, vocabulary.identity())?;
    field_key(writer, constants::key::TYPES)?;
    writer.array(vocabulary.types().len())?;
    for contract in vocabulary.types() {
        encode_vocabulary_type_contract(writer, contract, depth + 2)?;
    }
    Ok(())
}

/// Encodes one vocabulary identity.
fn encode_vocabulary_identity(
    writer: &mut CborWriter,
    identity: &VocabularyIdentity,
) -> Result<(), EncodingError> {
    writer.map(6)?;
    field_text(writer, constants::key::IDENTITY, identity.identity())?;
    field_text(writer, constants::key::VERSION, identity.version())?;
    field_text(
        writer,
        constants::key::SCHEMA_VERSION,
        identity.schema_version(),
    )?;
    field_text(
        writer,
        constants::key::ENCODING_VERSION,
        identity.encoding_version(),
    )?;
    field_key(writer, constants::key::CONTENT_DIGEST)?;
    writer.bytes(&identity.content_digest().as_bytes())?;
    field_key(writer, constants::key::REQUIRED_FEATURES)?;
    encode_text_array(writer, identity.required_features())
}

/// Encodes one vocabulary type contract.
fn encode_vocabulary_type_contract(
    writer: &mut CborWriter,
    contract: &VocabularyTypeContract,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    writer.map(2)?;
    field_key(writer, constants::key::IDENTITY)?;
    check_depth(depth + 1)?;
    encode_vocabulary_type(writer, contract.identity())?;
    field_key(writer, constants::key::FIELDS)?;
    writer.array(contract.fields().len())?;
    for field in contract.fields() {
        encode_vocabulary_field(writer, field, depth + 2)?;
    }
    Ok(())
}

/// Encodes one vocabulary field contract.
fn encode_vocabulary_field(
    writer: &mut CborWriter,
    field: &VocabularyFieldContract,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    writer.map(3)?;
    field_text(writer, constants::key::NAME, field.name())?;
    field_key(writer, constants::key::TYPE)?;
    encode_type(writer, field.resolved_type(), depth + 1)?;
    field_key(writer, constants::key::DEFAULT)?;
    encode_optional_value(writer, field.default_value(), depth + 1)
}

/// Encodes one resolved type.
fn encode_type(
    writer: &mut CborWriter,
    resolved_type: &ResolvedType,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    match resolved_type {
        ResolvedType::Num => kind_only(writer, constants::kind::NUM),
        ResolvedType::String => kind_only(writer, constants::kind::STRING),
        ResolvedType::Bool => kind_only(writer, constants::kind::BOOL),
        ResolvedType::Record(identity) => {
            check_depth(depth + 2)?;
            writer.map(2)?;
            field_text(writer, constants::key::KIND, constants::kind::RECORD)?;
            field_key(writer, constants::key::IDENTITY)?;
            encode_nominal(writer, identity)
        }
        ResolvedType::VocabularyRecord(identity) => {
            check_depth(depth + 1)?;
            writer.map(2)?;
            field_text(
                writer,
                constants::key::KIND,
                constants::kind::VOCABULARY_RECORD,
            )?;
            field_key(writer, constants::key::IDENTITY)?;
            encode_vocabulary_type(writer, identity)
        }
        ResolvedType::List(inner) => encode_inner_type(writer, constants::kind::LIST, inner, depth),
        ResolvedType::Ref(inner) => encode_inner_type(writer, constants::kind::REF, inner, depth),
        ResolvedType::Nullable(inner) => {
            encode_inner_type(writer, constants::kind::NULLABLE, inner, depth)
        }
    }
}

/// Encodes a type variant containing one inner type.
fn encode_inner_type(
    writer: &mut CborWriter,
    kind: &str,
    inner: &ResolvedType,
    depth: usize,
) -> Result<(), EncodingError> {
    writer.map(2)?;
    field_text(writer, constants::key::KIND, kind)?;
    field_key(writer, constants::key::INNER)?;
    encode_type(writer, inner, depth + 1)
}

/// Encodes one optional logical value.
fn encode_optional_value(
    writer: &mut CborWriter,
    value: Option<&LogicalValue>,
    depth: usize,
) -> Result<(), EncodingError> {
    match value {
        Some(value) => encode_value(writer, value, depth),
        None => writer.null(),
    }
}

/// Encodes one final logical value.
fn encode_value(
    writer: &mut CborWriter,
    value: &LogicalValue,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth)?;
    match value {
        LogicalValue::Number(number) => {
            if number.coefficient().len() > constants::MAXIMUM_EXACT_NUMBER_DIGITS {
                return Err(EncodingError::EncodedSizeLimit);
            }
            writer.map(4)?;
            field_text(writer, constants::key::KIND, constants::kind::NUM)?;
            field_key(writer, constants::key::NEGATIVE)?;
            writer.boolean(number.is_negative())?;
            field_text(writer, constants::key::COEFFICIENT, number.coefficient())?;
            field_key(writer, constants::key::SCALE)?;
            writer.signed(number.scale())
        }
        LogicalValue::String(value) => {
            writer.map(2)?;
            field_text(writer, constants::key::KIND, constants::kind::STRING)?;
            field_text(writer, constants::key::VALUE, value)
        }
        LogicalValue::Boolean(value) => {
            writer.map(2)?;
            field_text(writer, constants::key::KIND, constants::kind::BOOL)?;
            field_key(writer, constants::key::VALUE)?;
            writer.boolean(*value)
        }
        LogicalValue::Null => kind_only(writer, constants::kind::NULL),
        LogicalValue::Record(record) => {
            check_depth(depth + 2)?;
            writer.map(3)?;
            field_text(writer, constants::key::KIND, constants::kind::RECORD)?;
            field_key(writer, constants::key::IDENTITY)?;
            encode_nominal(writer, record.nominal_type())?;
            field_key(writer, constants::key::FIELDS)?;
            encode_value_fields(writer, record.fields(), depth + 1)
        }
        LogicalValue::VocabularyRecord(record) => {
            check_depth(depth + 1)?;
            writer.map(3)?;
            field_text(
                writer,
                constants::key::KIND,
                constants::kind::VOCABULARY_RECORD,
            )?;
            field_key(writer, constants::key::IDENTITY)?;
            encode_vocabulary_type(writer, record.nominal_type())?;
            field_key(writer, constants::key::FIELDS)?;
            encode_value_fields(writer, record.fields(), depth + 1)
        }
        LogicalValue::List(items) => {
            writer.map(2)?;
            field_text(writer, constants::key::KIND, constants::kind::LIST)?;
            field_key(writer, constants::key::ITEMS)?;
            writer.array(items.len())?;
            for item in items {
                encode_value(writer, item, depth + 2)?;
            }
            Ok(())
        }
        LogicalValue::Reference(reference) => encode_reference(writer, reference, depth),
    }
}

/// Encodes one array of contextual record fields.
fn encode_value_fields(
    writer: &mut CborWriter,
    fields: &[RecordValueField],
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth + 1)?;
    writer.array(fields.len())?;
    for field in fields {
        writer.map(2)?;
        field_text(writer, constants::key::NAME, field.name())?;
        field_key(writer, constants::key::VALUE)?;
        encode_value(writer, field.value(), depth + 2)?;
    }
    Ok(())
}

/// Encodes one identity-reference value.
fn encode_reference(
    writer: &mut CborWriter,
    reference: &IdentityReference,
    depth: usize,
) -> Result<(), EncodingError> {
    check_depth(depth + 2)?;
    writer.map(4)?;
    field_text(writer, constants::key::KIND, constants::kind::REF)?;
    field_unsigned(
        writer,
        constants::key::TARGET_ELEMENT_ID,
        reference.target_element_id().get(),
    )?;
    field_key(writer, constants::key::TARGET_SYMBOL)?;
    encode_symbol(writer, reference.target_symbol_identity())?;
    field_key(writer, constants::key::TARGET_TYPE)?;
    encode_type(writer, reference.target_type(), depth + 1)
}

/// Encodes the complete source-map section.
fn encode_source_map(source_map: &SourceMap) -> Result<Vec<u8>, EncodingError> {
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
fn encode_provenance(artifacts: &CompilationArtifacts) -> Result<Vec<u8>, EncodingError> {
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
fn encode_derivation(derivation: &DerivationManifest) -> Result<Vec<u8>, EncodingError> {
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

/// Encodes the envelope after the four hashed sections are complete.
fn encode_envelope(
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
    writer.array(
        usize::try_from(capabilities.count_ones()).map_err(|_| EncodingError::InternalDefect)?,
    )?;
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

/// Assembles the fixed header, directory, and contiguous sections.
fn assemble_frame(
    capabilities: u64,
    sections: [Vec<u8>; constants::SECTION_COUNT],
) -> Result<EncodedArtifact, EncodingError> {
    let directory_bytes = constants::DIRECTORY_ENTRY_BYTES
        .checked_mul(constants::SECTION_COUNT)
        .ok_or(EncodingError::EncodedSizeLimit)?;
    let first_offset = constants::HEADER_BYTES
        .checked_add(directory_bytes)
        .ok_or(EncodingError::EncodedSizeLimit)?;
    let total_length = sections.iter().try_fold(first_offset, |length, section| {
        length
            .checked_add(section.len())
            .ok_or(EncodingError::EncodedSizeLimit)
    })?;
    if total_length > constants::MAXIMUM_ARTIFACT_BYTES {
        return Err(EncodingError::EncodedSizeLimit);
    }
    let mut ranges: [Range<usize>; constants::SECTION_COUNT] = array::from_fn(|_| 0..0);
    let mut offset = first_offset;
    for (index, section) in sections.iter().enumerate() {
        let end = offset
            .checked_add(section.len())
            .ok_or(EncodingError::EncodedSizeLimit)?;
        ranges[index] = offset..end;
        offset = end;
    }
    let mut bytes = Vec::with_capacity(total_length);
    bytes.extend_from_slice(&constants::MAGIC);
    bytes.extend_from_slice(&constants::FRAMING_REVISION.to_be_bytes());
    bytes.extend_from_slice(
        &u16::try_from(constants::HEADER_BYTES)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(total_length)
            .map_err(|_| EncodingError::EncodedSizeLimit)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(
        &u64::try_from(constants::HEADER_BYTES)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(
        &u32::try_from(constants::SECTION_COUNT)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(
        &u16::try_from(constants::DIRECTORY_ENTRY_BYTES)
            .map_err(|_| EncodingError::InternalDefect)?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&capabilities.to_be_bytes());
    for (index, range) in ranges.iter().enumerate() {
        bytes.extend_from_slice(
            &u16::try_from(index + 1)
                .map_err(|_| EncodingError::InternalDefect)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(&constants::SECTION_SCHEMA_REVISION.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(
            &u64::try_from(range.start)
                .map_err(|_| EncodingError::EncodedSizeLimit)?
                .to_be_bytes(),
        );
        bytes.extend_from_slice(
            &u64::try_from(range.len())
                .map_err(|_| EncodingError::EncodedSizeLimit)?
                .to_be_bytes(),
        );
    }
    for section in sections {
        bytes.extend_from_slice(&section);
    }
    if bytes.len() != total_length {
        return Err(EncodingError::InternalDefect);
    }
    Ok(EncodedArtifact {
        bytes,
        sections: ranges,
    })
}

/// Encodes a map containing only a `kind` discriminator.
fn kind_only(writer: &mut CborWriter, kind: &str) -> Result<(), EncodingError> {
    writer.map(1)?;
    field_text(writer, constants::key::KIND, kind)
}

/// Encodes one map key.
fn field_key(writer: &mut CborWriter, key: &str) -> Result<(), EncodingError> {
    writer.text(key)
}

/// Encodes one text-valued map field.
fn field_text(writer: &mut CborWriter, key: &str, value: &str) -> Result<(), EncodingError> {
    field_key(writer, key)?;
    writer.text(value)
}

/// Encodes one unsigned-integer-valued map field.
fn field_unsigned(writer: &mut CborWriter, key: &str, value: u64) -> Result<(), EncodingError> {
    field_key(writer, key)?;
    writer.unsigned(value)
}

/// Encodes one array of UTF-8 strings.
fn encode_text_array<T: AsRef<str>>(
    writer: &mut CborWriter,
    values: &[T],
) -> Result<(), EncodingError> {
    writer.array(values.len())?;
    for value in values {
        writer.text(value.as_ref())?;
    }
    Ok(())
}

/// Enforces the fixed logical nesting ceiling.
fn check_depth(depth: usize) -> Result<(), EncodingError> {
    if depth > constants::MAXIMUM_NESTING_DEPTH {
        Err(EncodingError::EncodedSizeLimit)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
