// SPDX-License-Identifier: Apache-2.0

//! Closed-schema reconstruction and cross-section validation.

use crate::{
    DecodeError, DecodeErrorClass, DecodeLimits, TraversalBudget, capability, constants,
    decoder::{CborValue, LocatedValue, parse_section},
    derive_capabilities,
};
use neutral_core::{
    ByteSpan, CancellationToken, EncodedSectionDigest, SemanticDigest, SourceContentDigest,
    StructuralLimits, VocabularyContentDigest,
};
use neutral_ir::{
    AcceptancePartition, CompilationArtifacts, Declaration, DeclarationFingerprint,
    DerivationManifest, ElementId, ExactNumber, FieldProvenanceRecord, IdentityReference,
    LogicalDocument, LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity,
    NominalTypeIdentity, Normalization, ProvenanceRecord, RecordFieldSchema, RecordTypeDefinition,
    RecordValue, RecordValueField, ReferenceProvenanceRecord, ResolvedType, ResourceFacts,
    ReuseProvenanceRecord, SourceMap, SourceMapEntry, ValueOrigin, VocabularyContract,
    VocabularyFieldContract, VocabularyIdentity, VocabularyRecordValue, VocabularyTypeContract,
    VocabularyTypeIdentity, language,
};
use neutral_reader::{ReaderError, ValidatedDocument};
use std::{collections::BTreeSet, ops::Range, sync::Arc};

/// Checked fixed frame metadata and borrowed section ranges.
struct Frame {
    /// Header capability mask.
    capabilities: u64,
    /// Exact ranges for all fixed sections.
    sections: [Range<usize>; constants::SECTION_COUNT],
}

/// Decoded derivation plus its reconstructed acceptance limits.
struct DecodedDerivation {
    /// Immutable typed derivation.
    manifest: DerivationManifest,
    /// Captured compiler acceptance limits.
    limits: StructuralLimits,
}

/// Four companion provenance collections decoded as one closed section.
struct DecodedProvenance {
    /// Root value provenance.
    values: Vec<ProvenanceRecord>,
    /// Nested field provenance.
    fields: Vec<FieldProvenanceRecord>,
    /// Ordinary reuse provenance.
    reuses: Vec<ReuseProvenanceRecord>,
    /// Identity-reference provenance.
    references: Vec<ReferenceProvenanceRecord>,
}

/// Mutable logical schema budget derived from host and captured limits.
struct SchemaBudget<'a> {
    /// Captured limits from the derivation section.
    limits: StructuralLimits,
    /// Logical values/types visited.
    nodes: u64,
    /// Cooperative cancellation signal.
    cancellation: &'a CancellationToken,
}

impl SchemaBudget<'_> {
    /// Accounts for one typed node before construction.
    fn visit(&mut self, depth: u64) -> Result<(), DecodeError> {
        self.check_cancelled()?;
        if depth > self.limits.nesting_depth() {
            return Err(error(DecodeErrorClass::EncodedSizeLimit));
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| error(DecodeErrorClass::EncodedSizeLimit))?;
        if self.nodes > self.limits.traversal_nodes() {
            return Err(error(DecodeErrorClass::EncodedSizeLimit));
        }
        Ok(())
    }

    /// Checks caller cancellation between schema nodes.
    fn check_cancelled(&self) -> Result<(), DecodeError> {
        if self.cancellation.is_cancelled() {
            Err(error(DecodeErrorClass::Cancelled))
        } else {
            Ok(())
        }
    }
}

/// Closed validated view over one duplicate-preserving untrusted map.
struct Object<'a> {
    /// Original member vector after exact-key validation.
    members: &'a [(String, LocatedValue)],
}

impl<'a> Object<'a> {
    /// Validates an exact closed key set before semantic construction.
    fn exact(value: &'a LocatedValue, keys: &[&str]) -> Result<Self, DecodeError> {
        let CborValue::Map(members) = &value.value else {
            return Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset));
        };
        if members.len() != keys.len() {
            return Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset));
        }
        let mut observed = BTreeSet::new();
        for (key, member) in members {
            if !keys.contains(&key.as_str()) || !observed.insert(key.as_str()) {
                return Err(at(DecodeErrorClass::InvalidEncodedSchema, member.offset));
            }
        }
        if keys.iter().any(|key| !observed.contains(key)) {
            return Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset));
        }
        Ok(Self { members })
    }

    /// Returns one required already-validated member.
    fn get(&self, key: &str) -> Result<&'a LocatedValue, DecodeError> {
        self.members
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value))
            .ok_or_else(|| error(DecodeErrorClass::InternalDefect))
    }
}

/// Decodes hostile bytes into a reader view only after complete validation.
///
/// # Errors
///
/// Returns one bounded classified [`DecodeError`] for every malformed,
/// unsupported, oversized, inconsistent, or cancelled input.
pub fn decode(
    bytes: &[u8],
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedDocument, DecodeError> {
    check_cancelled(cancellation)?;
    let frame = decode_frame(bytes, limits)?;
    let envelope = parse(bytes, &frame, 0, limits, cancellation)?;
    validate_envelope(&envelope, &frame, bytes)?;

    let derivation_value = parse(bytes, &frame, 4, limits, cancellation)?;
    let derivation = decode_derivation(&derivation_value)?;
    let effective = effective_limits(limits, derivation.limits);
    let logical_value = parse(bytes, &frame, 1, effective, cancellation)?;
    let source_value = parse(bytes, &frame, 2, effective, cancellation)?;
    let provenance_value = parse(bytes, &frame, 3, effective, cancellation)?;

    let mut budget = SchemaBudget {
        limits: derivation.limits,
        nodes: 0,
        cancellation,
    };
    let logical = decode_logical(&logical_value, &mut budget)?;
    let source_map = decode_source_map(&source_value)?;
    let provenance = decode_provenance(&provenance_value)?;
    let artifacts =
        CompilationArtifacts::new(logical, source_map, provenance.values, derivation.manifest)
            .with_field_provenance(provenance.fields)
            .with_reuse_provenance(provenance.reuses)
            .with_reference_provenance(provenance.references);
    validate_cross_sections(&artifacts)?;
    let mut capability_budget = TraversalBudget::new();
    let actual = derive_capabilities(&artifacts, &mut capability_budget)
        .map_err(|_| error(DecodeErrorClass::EncodedSizeLimit))?;
    if actual != frame.capabilities {
        return Err(error(DecodeErrorClass::UnsupportedCapability));
    }
    check_cancelled(cancellation)?;
    ValidatedDocument::from_compiler_output(Arc::new(artifacts)).map_err(map_reader_error)
}

/// Parses one fixed section using its absolute range.
fn parse(
    bytes: &[u8],
    frame: &Frame,
    index: usize,
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<LocatedValue, DecodeError> {
    let range = frame
        .sections
        .get(index)
        .ok_or_else(|| error(DecodeErrorClass::InternalDefect))?;
    parse_section(&bytes[range.clone()], range.start, limits, cancellation)
}

/// Validates the outer header and directory before any CBOR allocation.
fn decode_frame(bytes: &[u8], limits: DecodeLimits) -> Result<Frame, DecodeError> {
    if bytes.len() > limits.maximum_artifact_bytes() {
        return Err(error(DecodeErrorClass::EncodedSizeLimit));
    }
    if bytes.len() < constants::HEADER_BYTES {
        return Err(error(DecodeErrorClass::MalformedFrame));
    }
    if bytes.get(..constants::MAGIC.len()) != Some(constants::MAGIC.as_slice()) {
        return Err(at(DecodeErrorClass::MalformedFrame, 0));
    }
    if read_u16(bytes, 8)? != constants::FRAMING_REVISION {
        return Err(at(DecodeErrorClass::UnsupportedVersion, 8));
    }
    if usize::from(read_u16(bytes, 10)?) != constants::HEADER_BYTES
        || read_u32(bytes, 12)? != 0
        || usize_from_u64(read_u64(bytes, 24)?)? != constants::HEADER_BYTES
        || usize_from_u32(read_u32(bytes, 32)?)? != constants::SECTION_COUNT
        || usize::from(read_u16(bytes, 36)?) != constants::DIRECTORY_ENTRY_BYTES
        || read_u16(bytes, 38)? != 0
    {
        return Err(error(DecodeErrorClass::MalformedFrame));
    }
    if usize_from_u64(read_u64(bytes, 16)?)? != bytes.len() {
        return Err(at(DecodeErrorClass::MalformedFrame, 16));
    }
    let capabilities = read_u64(bytes, 40)?;
    if capabilities & !constants::KNOWN_CAPABILITY_MASK != 0 {
        return Err(at(DecodeErrorClass::UnsupportedCapability, 40));
    }
    let directory_size = constants::DIRECTORY_ENTRY_BYTES
        .checked_mul(constants::SECTION_COUNT)
        .ok_or_else(|| error(DecodeErrorClass::InternalDefect))?;
    let first_section = constants::HEADER_BYTES
        .checked_add(directory_size)
        .ok_or_else(|| error(DecodeErrorClass::InternalDefect))?;
    if bytes.len() < first_section {
        return Err(error(DecodeErrorClass::MalformedFrame));
    }
    let mut sections: [Range<usize>; constants::SECTION_COUNT] = std::array::from_fn(|_| 0..0);
    let mut expected_offset = first_section;
    for (index, range) in sections.iter_mut().enumerate() {
        let entry = constants::HEADER_BYTES
            .checked_add(index * constants::DIRECTORY_ENTRY_BYTES)
            .ok_or_else(|| error(DecodeErrorClass::InternalDefect))?;
        let expected_kind =
            u16::try_from(index + 1).map_err(|_| error(DecodeErrorClass::InternalDefect))?;
        if read_u16(bytes, entry)? != expected_kind || read_u32(bytes, entry + 4)? != 0 {
            return Err(at(DecodeErrorClass::MalformedFrame, u64_from_usize(entry)?));
        }
        if read_u16(bytes, entry + 2)? != constants::SECTION_SCHEMA_REVISION {
            return Err(at(
                DecodeErrorClass::UnsupportedVersion,
                u64_from_usize(entry + 2)?,
            ));
        }
        let offset = usize_from_u64(read_u64(bytes, entry + 8)?)?;
        let length = usize_from_u64(read_u64(bytes, entry + 16)?)?;
        if offset != expected_offset || length == 0 || length > limits.maximum_section_bytes() {
            return Err(at(
                if length > limits.maximum_section_bytes() {
                    DecodeErrorClass::EncodedSizeLimit
                } else {
                    DecodeErrorClass::MalformedFrame
                },
                u64_from_usize(entry + 8)?,
            ));
        }
        let end = offset
            .checked_add(length)
            .ok_or_else(|| error(DecodeErrorClass::MalformedFrame))?;
        if end > bytes.len() {
            return Err(at(
                DecodeErrorClass::MalformedFrame,
                u64_from_usize(entry + 16)?,
            ));
        }
        *range = offset..end;
        expected_offset = end;
    }
    if expected_offset != bytes.len() {
        return Err(error(DecodeErrorClass::MalformedFrame));
    }
    Ok(Frame {
        capabilities,
        sections,
    })
}

/// Validates envelope versions, capabilities, producer shape, and integrity.
fn validate_envelope(value: &LocatedValue, frame: &Frame, bytes: &[u8]) -> Result<(), DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::ENCODING,
            constants::key::FRAMING_REVISION,
            constants::key::REQUIRED_CAPABILITIES,
            constants::key::VERSIONS,
            constants::key::PRODUCER,
            constants::key::INTEGRITY,
        ],
    )?;
    require_version(object.get(constants::key::ENCODING)?, constants::ENCODING)?;
    if unsigned(object.get(constants::key::FRAMING_REVISION)?)?
        != u64::from(constants::FRAMING_REVISION)
    {
        return Err(at(
            DecodeErrorClass::UnsupportedVersion,
            object.get(constants::key::FRAMING_REVISION)?.offset,
        ));
    }
    validate_capability_names(
        object.get(constants::key::REQUIRED_CAPABILITIES)?,
        frame.capabilities,
    )?;
    validate_versions(object.get(constants::key::VERSIONS)?)?;
    validate_producer(object.get(constants::key::PRODUCER)?)?;
    validate_integrity(object.get(constants::key::INTEGRITY)?, frame, bytes)
}

/// Validates capability names in exact ascending-bit order.
fn validate_capability_names(value: &LocatedValue, mask: u64) -> Result<(), DecodeError> {
    let values = array(value)?;
    let expected_count =
        usize::try_from(mask.count_ones()).map_err(|_| error(DecodeErrorClass::InternalDefect))?;
    if values.len() != expected_count {
        return Err(at(DecodeErrorClass::UnsupportedCapability, value.offset));
    }
    let mut index = 0;
    for (bit, expected) in (0_u32..).zip(constants::CAPABILITY_NAMES) {
        if mask & capability(bit) != 0 {
            let actual = text(
                values
                    .get(index)
                    .ok_or_else(|| error(DecodeErrorClass::InternalDefect))?,
            )?;
            if actual != expected {
                return Err(at(
                    DecodeErrorClass::UnsupportedCapability,
                    values[index].offset,
                ));
            }
            index += 1;
        }
    }
    Ok(())
}

/// Validates the exact envelope contract versions.
fn validate_versions(value: &LocatedValue) -> Result<(), DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::LANGUAGE_BEHAVIOR,
            constants::key::LOGICAL_IR,
            constants::key::SOURCE_MAP,
            constants::key::PROVENANCE,
            constants::key::DERIVATION,
            constants::key::VOCABULARY_SCHEMA,
        ],
    )?;
    require_version(
        object.get(constants::key::LANGUAGE_BEHAVIOR)?,
        neutral_ir::LANGUAGE_BEHAVIOR_VERSION,
    )?;
    require_version(
        object.get(constants::key::LOGICAL_IR)?,
        neutral_ir::LOGICAL_IR_SCHEMA_VERSION,
    )?;
    require_version(
        object.get(constants::key::SOURCE_MAP)?,
        neutral_ir::SOURCE_MAP_VERSION,
    )?;
    require_version(
        object.get(constants::key::PROVENANCE)?,
        neutral_ir::PROVENANCE_VERSION,
    )?;
    require_version(
        object.get(constants::key::DERIVATION)?,
        neutral_ir::DERIVATION_SCHEMA_VERSION,
    )?;
    require_version(
        object.get(constants::key::VOCABULARY_SCHEMA)?,
        neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
    )
}

/// Validates envelope-only producer/build scalar shapes.
fn validate_producer(value: &LocatedValue) -> Result<(), DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::NAME,
            constants::key::VERSION,
            constants::key::BUILD,
        ],
    )?;
    text(object.get(constants::key::NAME)?)?;
    text(object.get(constants::key::VERSION)?)?;
    let build = object.get(constants::key::BUILD)?;
    if !matches!(build.value, CborValue::Null) {
        text(build)?;
    }
    Ok(())
}

/// Validates exact hashes for section kinds 2 through 5.
fn validate_integrity(
    value: &LocatedValue,
    frame: &Frame,
    bytes: &[u8],
) -> Result<(), DecodeError> {
    let records = array(value)?;
    if records.len() != constants::INTEGRITY_SECTION_COUNT {
        return Err(at(DecodeErrorClass::IntegrityMismatch, value.offset));
    }
    for (index, record) in records.iter().enumerate() {
        let object = Object::exact(record, &[constants::key::SECTION, constants::key::SHA256])
            .map_err(|_| at(DecodeErrorClass::IntegrityMismatch, record.offset))?;
        let kind = u64::try_from(index + 2).map_err(|_| error(DecodeErrorClass::InternalDefect))?;
        if unsigned(object.get(constants::key::SECTION)?)? != kind {
            return Err(at(DecodeErrorClass::IntegrityMismatch, record.offset));
        }
        let expected = digest_bytes(object.get(constants::key::SHA256)?)?;
        let section = frame
            .sections
            .get(index + 1)
            .ok_or_else(|| error(DecodeErrorClass::InternalDefect))?;
        let actual = EncodedSectionDigest::from_bytes(&bytes[section.clone()]).as_bytes();
        if expected != actual {
            return Err(at(DecodeErrorClass::IntegrityMismatch, record.offset));
        }
    }
    Ok(())
}

/// Decodes the complete logical payload.
fn decode_logical(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
) -> Result<LogicalDocument, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::LOGICAL_IR_SCHEMA_VERSION,
            constants::key::MODULE,
            constants::key::RECORD_TYPES,
            constants::key::VOCABULARY,
            constants::key::DECLARATIONS,
        ],
    )?;
    require_version(
        object.get(constants::key::LOGICAL_IR_SCHEMA_VERSION)?,
        neutral_ir::LOGICAL_IR_SCHEMA_VERSION,
    )?;
    let module = decode_module(object.get(constants::key::MODULE)?)?;
    let record_values = array(object.get(constants::key::RECORD_TYPES)?)?;
    check_collection(record_values.len(), budget.limits.declarations())?;
    let mut records = Vec::new();
    for record in record_values {
        records.push(decode_record_type(record, budget, 1)?);
    }
    let declaration_values = array(object.get(constants::key::DECLARATIONS)?)?;
    let total = record_values
        .len()
        .checked_add(declaration_values.len())
        .ok_or_else(|| error(DecodeErrorClass::EncodedSizeLimit))?;
    check_collection(total, budget.limits.declarations())?;
    let mut declarations = Vec::new();
    for declaration in declaration_values {
        declarations.push(decode_declaration(declaration, budget, 1)?);
    }
    let mut document = LogicalDocument::with_record_types(module, records, declarations);
    let vocabulary = object.get(constants::key::VOCABULARY)?;
    if !matches!(vocabulary.value, CborValue::Null) {
        document = document.with_vocabulary(decode_vocabulary_contract(vocabulary, budget, 1)?);
    }
    Ok(document)
}

/// Decodes one module identity and requires the frozen language version.
fn decode_module(value: &LocatedValue) -> Result<LogicalModuleIdentity, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::LANGUAGE_BEHAVIOR_VERSION,
            constants::key::NAME,
        ],
    )?;
    require_version(
        object.get(constants::key::LANGUAGE_BEHAVIOR_VERSION)?,
        neutral_ir::LANGUAGE_BEHAVIOR_VERSION,
    )?;
    Ok(LogicalModuleIdentity::new(
        neutral_ir::LANGUAGE_BEHAVIOR_VERSION,
        snake_name(object.get(constants::key::NAME)?)?,
    ))
}

/// Decodes one module-symbol identity.
fn decode_symbol(value: &LocatedValue) -> Result<ModuleSymbolIdentity, DecodeError> {
    let object = Object::exact(value, &[constants::key::MODULE, constants::key::NAME])?;
    Ok(ModuleSymbolIdentity::new(
        decode_module(object.get(constants::key::MODULE)?)?,
        text(object.get(constants::key::NAME)?)?,
    ))
}

/// Decodes one module-owned nominal identity.
fn decode_nominal(value: &LocatedValue) -> Result<NominalTypeIdentity, DecodeError> {
    let object = Object::exact(value, &[constants::key::MODULE, constants::key::NAME])?;
    Ok(NominalTypeIdentity::new(
        decode_module(object.get(constants::key::MODULE)?)?,
        upper_name(object.get(constants::key::NAME)?)?,
    ))
}

/// Decodes one vocabulary-owned type identity.
fn decode_vocabulary_type(value: &LocatedValue) -> Result<VocabularyTypeIdentity, DecodeError> {
    let object = Object::exact(value, &[constants::key::VOCABULARY, constants::key::NAME])?;
    Ok(VocabularyTypeIdentity::new(
        upper_name(object.get(constants::key::VOCABULARY)?)?,
        upper_name(object.get(constants::key::NAME)?)?,
    ))
}

/// Decodes one record type definition.
fn decode_record_type(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<RecordTypeDefinition, DecodeError> {
    budget.visit(depth)?;
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::SYMBOL,
            constants::key::FINGERPRINT,
            constants::key::IDENTITY,
            constants::key::FIELDS,
        ],
    )?;
    let fields = array(object.get(constants::key::FIELDS)?)?;
    check_collection(fields.len(), budget.limits.record_fields())?;
    let mut decoded_fields = Vec::new();
    for field in fields {
        decoded_fields.push(decode_record_field(field, budget, depth + 1)?);
    }
    Ok(RecordTypeDefinition::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        decode_symbol(object.get(constants::key::SYMBOL)?)?,
        decode_fingerprint(object.get(constants::key::FINGERPRINT)?)?,
        decode_nominal(object.get(constants::key::IDENTITY)?)?,
        decoded_fields,
    ))
}

/// Decodes one user-record field schema.
fn decode_record_field(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<RecordFieldSchema, DecodeError> {
    budget.visit(depth)?;
    let object = Object::exact(
        value,
        &[
            constants::key::NAME,
            constants::key::TYPE,
            constants::key::DEFAULT,
        ],
    )?;
    let mut field = RecordFieldSchema::new(
        snake_name(object.get(constants::key::NAME)?)?,
        decode_type(object.get(constants::key::TYPE)?, budget, depth + 1)?,
    );
    let default = object.get(constants::key::DEFAULT)?;
    if !matches!(default.value, CborValue::Null) {
        field = field.with_default(decode_value(default, budget, depth + 1)?);
    }
    Ok(field)
}

/// Decodes one immutable binding declaration.
fn decode_declaration(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<Declaration, DecodeError> {
    budget.visit(depth)?;
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::SYMBOL,
            constants::key::FINGERPRINT,
            constants::key::NAME,
            constants::key::TYPE,
            constants::key::VALUE,
        ],
    )?;
    Ok(Declaration::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        decode_symbol(object.get(constants::key::SYMBOL)?)?,
        decode_fingerprint(object.get(constants::key::FINGERPRINT)?)?,
        snake_name(object.get(constants::key::NAME)?)?,
        decode_type(object.get(constants::key::TYPE)?, budget, depth + 1)?,
        decode_value(object.get(constants::key::VALUE)?, budget, depth + 1)?,
    ))
}

/// Decodes a complete captured vocabulary contract.
fn decode_vocabulary_contract(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<VocabularyContract, DecodeError> {
    budget.visit(depth)?;
    let object = Object::exact(value, &[constants::key::IDENTITY, constants::key::TYPES])?;
    let identity = decode_vocabulary_identity(object.get(constants::key::IDENTITY)?)?;
    let types = array(object.get(constants::key::TYPES)?)?;
    check_collection(types.len(), budget.limits.declarations())?;
    let mut decoded = Vec::new();
    for value in types {
        decoded.push(decode_vocabulary_type_contract(value, budget, depth + 1)?);
    }
    Ok(VocabularyContract::new(identity, decoded))
}

/// Decodes exact captured vocabulary identity facts.
fn decode_vocabulary_identity(value: &LocatedValue) -> Result<VocabularyIdentity, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::IDENTITY,
            constants::key::VERSION,
            constants::key::SCHEMA_VERSION,
            constants::key::ENCODING_VERSION,
            constants::key::CONTENT_DIGEST,
            constants::key::REQUIRED_FEATURES,
        ],
    )?;
    require_version(
        object.get(constants::key::SCHEMA_VERSION)?,
        neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
    )?;
    require_version(
        object.get(constants::key::ENCODING_VERSION)?,
        neutral_vocabulary::VOCABULARY_ENCODING_VERSION,
    )?;
    let features = text_array(object.get(constants::key::REQUIRED_FEATURES)?)?;
    if !is_strictly_ordered(&features)
        || features
            .iter()
            .any(|feature| !language::is_feature_id(feature))
    {
        return Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset));
    }
    let identity = upper_name(object.get(constants::key::IDENTITY)?)?;
    let version = text(object.get(constants::key::VERSION)?)?;
    if !language::is_exact_release_version(version) {
        return Err(at(DecodeErrorClass::InvalidLogicalIr, value.offset));
    }
    Ok(VocabularyIdentity::new(
        identity,
        version,
        neutral_vocabulary::VOCABULARY_SCHEMA_VERSION,
        neutral_vocabulary::VOCABULARY_ENCODING_VERSION,
        VocabularyContentDigest::from_raw_bytes(digest_bytes(
            object.get(constants::key::CONTENT_DIGEST)?,
        )?),
        features,
    ))
}

/// Decodes one vocabulary-owned type contract.
fn decode_vocabulary_type_contract(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<VocabularyTypeContract, DecodeError> {
    budget.visit(depth)?;
    let object = Object::exact(value, &[constants::key::IDENTITY, constants::key::FIELDS])?;
    let fields = array(object.get(constants::key::FIELDS)?)?;
    check_collection(fields.len(), budget.limits.record_fields())?;
    let mut decoded = Vec::new();
    for field in fields {
        decoded.push(decode_vocabulary_field(field, budget, depth + 1)?);
    }
    Ok(VocabularyTypeContract::new(
        decode_vocabulary_type(object.get(constants::key::IDENTITY)?)?,
        decoded,
    ))
}

/// Decodes one vocabulary field contract.
fn decode_vocabulary_field(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<VocabularyFieldContract, DecodeError> {
    budget.visit(depth)?;
    let object = Object::exact(
        value,
        &[
            constants::key::NAME,
            constants::key::TYPE,
            constants::key::DEFAULT,
        ],
    )?;
    let default = object.get(constants::key::DEFAULT)?;
    Ok(VocabularyFieldContract::new(
        snake_name(object.get(constants::key::NAME)?)?,
        decode_type(object.get(constants::key::TYPE)?, budget, depth + 1)?,
        if matches!(default.value, CborValue::Null) {
            None
        } else {
            Some(decode_value(default, budget, depth + 1)?)
        },
    ))
}

/// Decodes one closed resolved-type sum.
fn decode_type(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<ResolvedType, DecodeError> {
    budget.visit(depth)?;
    let kind = discriminator(value)?;
    match kind {
        candidate if candidate == constants::kind::NUM => {
            kind_object(value)?;
            Ok(ResolvedType::Num)
        }
        candidate if candidate == constants::kind::STRING => {
            kind_object(value)?;
            Ok(ResolvedType::String)
        }
        candidate if candidate == constants::kind::BOOL => {
            kind_object(value)?;
            Ok(ResolvedType::Bool)
        }
        candidate if candidate == constants::kind::RECORD => {
            let object = Object::exact(value, &[constants::key::KIND, constants::key::IDENTITY])?;
            Ok(ResolvedType::Record(decode_nominal(
                object.get(constants::key::IDENTITY)?,
            )?))
        }
        candidate if candidate == constants::kind::VOCABULARY_RECORD => {
            let object = Object::exact(value, &[constants::key::KIND, constants::key::IDENTITY])?;
            Ok(ResolvedType::VocabularyRecord(decode_vocabulary_type(
                object.get(constants::key::IDENTITY)?,
            )?))
        }
        candidate
            if candidate == constants::kind::LIST
                || candidate == constants::kind::REF
                || candidate == constants::kind::NULLABLE =>
        {
            let object = Object::exact(value, &[constants::key::KIND, constants::key::INNER])?;
            let inner = decode_type(object.get(constants::key::INNER)?, budget, depth + 1)?;
            Ok(if candidate == constants::kind::LIST {
                ResolvedType::list(inner)
            } else if candidate == constants::kind::REF {
                ResolvedType::reference(inner)
            } else {
                ResolvedType::nullable(inner)
            })
        }
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Decodes one closed final logical-value sum.
fn decode_value(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<LogicalValue, DecodeError> {
    budget.visit(depth)?;
    let kind = discriminator(value)?;
    match kind {
        candidate if candidate == constants::kind::NUM => decode_number(value, budget),
        candidate if candidate == constants::kind::STRING => {
            let object = Object::exact(value, &[constants::key::KIND, constants::key::VALUE])?;
            let text = text(object.get(constants::key::VALUE)?)?;
            check_collection(text.len(), budget.limits.string_bytes())?;
            Ok(LogicalValue::String(text.to_owned()))
        }
        candidate if candidate == constants::kind::BOOL => {
            let object = Object::exact(value, &[constants::key::KIND, constants::key::VALUE])?;
            Ok(LogicalValue::Boolean(boolean(
                object.get(constants::key::VALUE)?,
            )?))
        }
        candidate if candidate == constants::kind::NULL => {
            kind_object(value)?;
            Ok(LogicalValue::Null)
        }
        candidate if candidate == constants::kind::RECORD => {
            let object = Object::exact(
                value,
                &[
                    constants::key::KIND,
                    constants::key::IDENTITY,
                    constants::key::FIELDS,
                ],
            )?;
            Ok(LogicalValue::Record(RecordValue::new(
                decode_nominal(object.get(constants::key::IDENTITY)?)?,
                decode_value_fields(object.get(constants::key::FIELDS)?, budget, depth + 1)?,
            )))
        }
        candidate if candidate == constants::kind::VOCABULARY_RECORD => {
            let object = Object::exact(
                value,
                &[
                    constants::key::KIND,
                    constants::key::IDENTITY,
                    constants::key::FIELDS,
                ],
            )?;
            Ok(LogicalValue::VocabularyRecord(VocabularyRecordValue::new(
                decode_vocabulary_type(object.get(constants::key::IDENTITY)?)?,
                decode_value_fields(object.get(constants::key::FIELDS)?, budget, depth + 1)?,
            )))
        }
        candidate if candidate == constants::kind::LIST => {
            let object = Object::exact(value, &[constants::key::KIND, constants::key::ITEMS])?;
            let items = array(object.get(constants::key::ITEMS)?)?;
            check_collection(items.len(), budget.limits.list_items())?;
            let mut decoded = Vec::new();
            for item in items {
                decoded.push(decode_value(item, budget, depth + 1)?);
            }
            Ok(LogicalValue::List(decoded))
        }
        candidate if candidate == constants::kind::REF => {
            let object = Object::exact(
                value,
                &[
                    constants::key::KIND,
                    constants::key::TARGET_ELEMENT_ID,
                    constants::key::TARGET_SYMBOL,
                    constants::key::TARGET_TYPE,
                ],
            )?;
            Ok(LogicalValue::Reference(IdentityReference::new(
                ElementId::new(unsigned(object.get(constants::key::TARGET_ELEMENT_ID)?)?),
                decode_symbol(object.get(constants::key::TARGET_SYMBOL)?)?,
                decode_type(object.get(constants::key::TARGET_TYPE)?, budget, depth + 1)?,
            )))
        }
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Decodes one exact normalized number without host floating point.
fn decode_number(
    value: &LocatedValue,
    budget: &SchemaBudget<'_>,
) -> Result<LogicalValue, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::KIND,
            constants::key::NEGATIVE,
            constants::key::COEFFICIENT,
            constants::key::SCALE,
        ],
    )?;
    let number = ExactNumber::from_normalized_parts(
        boolean(object.get(constants::key::NEGATIVE)?)?,
        text(object.get(constants::key::COEFFICIENT)?)?,
        signed(object.get(constants::key::SCALE)?)?,
        budget
            .limits
            .numeric_digits()
            .min(u64::try_from(constants::MAXIMUM_EXACT_NUMBER_DIGITS).unwrap_or(u64::MAX)),
        budget.limits.numeric_scale(),
    );
    match number {
        Ok(number) => Ok(LogicalValue::Number(number)),
        Err(neutral_ir::IrError::ExactNumberLimitExceeded) => {
            Err(at(DecodeErrorClass::EncodedSizeLimit, value.offset))
        }
        Err(neutral_ir::IrError::InvalidExactNumber) => {
            Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset))
        }
    }
}

/// Decodes canonical record-value fields.
fn decode_value_fields(
    value: &LocatedValue,
    budget: &mut SchemaBudget<'_>,
    depth: u64,
) -> Result<Vec<RecordValueField>, DecodeError> {
    let fields = array(value)?;
    check_collection(fields.len(), budget.limits.record_fields())?;
    let mut decoded = Vec::new();
    for field in fields {
        let object = Object::exact(field, &[constants::key::NAME, constants::key::VALUE])?;
        decoded.push(RecordValueField::new(
            snake_name(object.get(constants::key::NAME)?)?,
            decode_value(object.get(constants::key::VALUE)?, budget, depth + 1)?,
        ));
    }
    Ok(decoded)
}

/// Decodes the complete source-map section.
fn decode_source_map(value: &LocatedValue) -> Result<SourceMap, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::SOURCE_MAP_VERSION,
            constants::key::SOURCE_DIGEST,
            constants::key::SOURCE_BYTE_LENGTH,
            constants::key::MODULE_SPAN,
            constants::key::ENTRIES,
        ],
    )?;
    require_version(
        object.get(constants::key::SOURCE_MAP_VERSION)?,
        neutral_ir::SOURCE_MAP_VERSION,
    )?;
    let source_length = unsigned(object.get(constants::key::SOURCE_BYTE_LENGTH)?)?;
    let module_span = decode_span(object.get(constants::key::MODULE_SPAN)?, source_length)?;
    let entries = array(object.get(constants::key::ENTRIES)?)?;
    let mut decoded = Vec::new();
    for entry in entries {
        decoded.push(decode_source_entry(entry, source_length)?);
    }
    Ok(SourceMap::new(
        SourceContentDigest::from_raw_bytes(digest_bytes(
            object.get(constants::key::SOURCE_DIGEST)?,
        )?),
        source_length,
        module_span,
        decoded,
    ))
}

/// Decodes one bounded half-open byte span.
fn decode_span(value: &LocatedValue, source_length: u64) -> Result<ByteSpan, DecodeError> {
    let object = Object::exact(value, &[constants::key::START, constants::key::END])?;
    let start = unsigned(object.get(constants::key::START)?)?;
    let end = unsigned(object.get(constants::key::END)?)?;
    if end > source_length {
        return Err(at(DecodeErrorClass::InvalidSourceMap, value.offset));
    }
    ByteSpan::new(start, end).map_err(|_| at(DecodeErrorClass::InvalidSourceMap, value.offset))
}

/// Decodes one source-map entry.
fn decode_source_entry(
    value: &LocatedValue,
    source_length: u64,
) -> Result<SourceMapEntry, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::DECLARATION_SPAN,
            constants::key::TYPE_SPAN,
            constants::key::NAME_SPAN,
            constants::key::VALUE_SPAN,
        ],
    )?;
    let declaration = decode_span(object.get(constants::key::DECLARATION_SPAN)?, source_length)?;
    let type_span = decode_span(object.get(constants::key::TYPE_SPAN)?, source_length)?;
    let name = decode_span(object.get(constants::key::NAME_SPAN)?, source_length)?;
    let value_span = decode_span(object.get(constants::key::VALUE_SPAN)?, source_length)?;
    if !contains(declaration, type_span)
        || !contains(declaration, name)
        || !contains(declaration, value_span)
    {
        return Err(at(DecodeErrorClass::InvalidSourceMap, value.offset));
    }
    Ok(SourceMapEntry::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        declaration,
        type_span,
        name,
        value_span,
    ))
}

/// Returns whether an outer span contains an inner span.
const fn contains(outer: ByteSpan, inner: ByteSpan) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

/// Decodes all four provenance arrays.
fn decode_provenance(value: &LocatedValue) -> Result<DecodedProvenance, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::PROVENANCE_VERSION,
            constants::key::VALUES,
            constants::key::FIELDS,
            constants::key::REUSES,
            constants::key::REFERENCES,
        ],
    )?;
    require_version(
        object.get(constants::key::PROVENANCE_VERSION)?,
        neutral_ir::PROVENANCE_VERSION,
    )?;
    let mut values = Vec::new();
    for record in array(object.get(constants::key::VALUES)?)? {
        values.push(decode_value_provenance(record)?);
    }
    let mut fields = Vec::new();
    for record in array(object.get(constants::key::FIELDS)?)? {
        fields.push(decode_field_provenance(record)?);
    }
    let mut reuses = Vec::new();
    for record in array(object.get(constants::key::REUSES)?)? {
        reuses.push(decode_reuse_provenance(record)?);
    }
    let mut references = Vec::new();
    for record in array(object.get(constants::key::REFERENCES)?)? {
        references.push(decode_reference_provenance(record)?);
    }
    Ok(DecodedProvenance {
        values,
        fields,
        reuses,
        references,
    })
}

/// Decodes one root value-provenance record.
fn decode_value_provenance(value: &LocatedValue) -> Result<ProvenanceRecord, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::ORIGIN,
            constants::key::NORMALIZATION,
        ],
    )?;
    Ok(ProvenanceRecord::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        decode_origin(object.get(constants::key::ORIGIN)?)?,
        decode_normalization(object.get(constants::key::NORMALIZATION)?)?,
    ))
}

/// Decodes one field-provenance record.
fn decode_field_provenance(value: &LocatedValue) -> Result<FieldProvenanceRecord, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::FIELD_PATH,
            constants::key::ORIGIN,
        ],
    )?;
    Ok(FieldProvenanceRecord::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        text_array(object.get(constants::key::FIELD_PATH)?)?,
        decode_origin(object.get(constants::key::ORIGIN)?)?,
    ))
}

/// Decodes one ordinary-reuse provenance record.
fn decode_reuse_provenance(value: &LocatedValue) -> Result<ReuseProvenanceRecord, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::VALUE_PATH,
            constants::key::SOURCE_ELEMENT_ID,
        ],
    )?;
    Ok(ReuseProvenanceRecord::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        text_array(object.get(constants::key::VALUE_PATH)?)?,
        ElementId::new(unsigned(object.get(constants::key::SOURCE_ELEMENT_ID)?)?),
    ))
}

/// Decodes one identity-reference provenance record.
fn decode_reference_provenance(
    value: &LocatedValue,
) -> Result<ReferenceProvenanceRecord, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::ELEMENT_ID,
            constants::key::VALUE_PATH,
            constants::key::TARGET_ELEMENT_ID,
        ],
    )?;
    Ok(ReferenceProvenanceRecord::new(
        ElementId::new(unsigned(object.get(constants::key::ELEMENT_ID)?)?),
        text_array(object.get(constants::key::VALUE_PATH)?)?,
        ElementId::new(unsigned(object.get(constants::key::TARGET_ELEMENT_ID)?)?),
    ))
}

/// Decodes a stable value-origin spelling through its owning IR constants.
fn decode_origin(value: &LocatedValue) -> Result<ValueOrigin, DecodeError> {
    let value_text = text(value)?;
    [
        ValueOrigin::ExplicitSource,
        ValueOrigin::OrdinaryReuse,
        ValueOrigin::IdentityReference,
        ValueOrigin::ExplicitRecordField,
        ValueOrigin::UserRecordDefault,
        ValueOrigin::VocabularyDefault,
    ]
    .into_iter()
    .find(|candidate| candidate.as_str() == value_text)
    .ok_or_else(|| at(DecodeErrorClass::InvalidEncodedSchema, value.offset))
}

/// Decodes a stable normalization spelling through its owning IR constants.
fn decode_normalization(value: &LocatedValue) -> Result<Normalization, DecodeError> {
    let value_text = text(value)?;
    [
        Normalization::ExactNumberCanonicalization,
        Normalization::StringEscapeDecoding,
        Normalization::BooleanIdentity,
        Normalization::NullIdentity,
        Normalization::RecordContextualization,
        Normalization::ListContextualization,
        Normalization::ImmutableValueReuse,
        Normalization::IdentityReferenceResolution,
    ]
    .into_iter()
    .find(|candidate| candidate.as_str() == value_text)
    .ok_or_else(|| at(DecodeErrorClass::InvalidEncodedSchema, value.offset))
}

/// Decodes derivation facts early so captured limits govern later construction.
fn decode_derivation(value: &LocatedValue) -> Result<DecodedDerivation, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::DERIVATION_SCHEMA_VERSION,
            constants::key::LANGUAGE_BEHAVIOR_VERSION,
            constants::key::LOGICAL_IR_SCHEMA_VERSION,
            constants::key::SOURCE_MAP_VERSION,
            constants::key::PROVENANCE_VERSION,
            constants::key::MEANING,
            constants::key::ACCEPTANCE,
            constants::key::DIAGNOSTIC_POLICY,
            constants::key::RESOURCE_FACTS,
            constants::key::VOCABULARY,
        ],
    )?;
    for (key, expected) in [
        (
            constants::key::DERIVATION_SCHEMA_VERSION,
            neutral_ir::DERIVATION_SCHEMA_VERSION,
        ),
        (
            constants::key::LANGUAGE_BEHAVIOR_VERSION,
            neutral_ir::LANGUAGE_BEHAVIOR_VERSION,
        ),
        (
            constants::key::LOGICAL_IR_SCHEMA_VERSION,
            neutral_ir::LOGICAL_IR_SCHEMA_VERSION,
        ),
        (
            constants::key::SOURCE_MAP_VERSION,
            neutral_ir::SOURCE_MAP_VERSION,
        ),
        (
            constants::key::PROVENANCE_VERSION,
            neutral_ir::PROVENANCE_VERSION,
        ),
    ] {
        require_version(object.get(key)?, expected)?;
    }
    let meaning = Object::exact(
        object.get(constants::key::MEANING)?,
        &[constants::key::SOURCE_DIGEST],
    )?;
    let source_digest = SourceContentDigest::from_raw_bytes(digest_bytes(
        meaning.get(constants::key::SOURCE_DIGEST)?,
    )?);
    let limits = decode_acceptance(object.get(constants::key::ACCEPTANCE)?)?;
    let policy = Object::exact(
        object.get(constants::key::DIAGNOSTIC_POLICY)?,
        &[constants::key::SAFE_BOUNDED_OUTPUT],
    )?;
    if !boolean(policy.get(constants::key::SAFE_BOUNDED_OUTPUT)?)? {
        return Err(at(DecodeErrorClass::InvalidDerivation, value.offset));
    }
    let facts = decode_resource_facts(object.get(constants::key::RESOURCE_FACTS)?)?;
    if facts.diagnostics() != 0 {
        return Err(at(DecodeErrorClass::InvalidDerivation, value.offset));
    }
    let mut manifest = DerivationManifest::new(
        neutral_ir::LANGUAGE_BEHAVIOR_VERSION,
        source_digest,
        AcceptancePartition::from_limits(limits),
        facts,
    );
    let vocabulary = object.get(constants::key::VOCABULARY)?;
    if !matches!(vocabulary.value, CborValue::Null) {
        manifest = manifest.with_vocabulary(decode_vocabulary_identity(vocabulary)?);
    }
    Ok(DecodedDerivation { manifest, limits })
}

/// Decodes captured acceptance limits using the owning core builder.
fn decode_acceptance(value: &LocatedValue) -> Result<StructuralLimits, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::SOURCE_BYTES,
            constants::key::DIAGNOSTICS,
            constants::key::STRING_BYTES,
            constants::key::NUMERIC_DIGITS,
            constants::key::NUMERIC_SCALE,
            constants::key::DECLARATIONS,
            constants::key::RECORD_FIELDS,
            constants::key::NESTING_DEPTH,
            constants::key::LIST_ITEMS,
            constants::key::TRAVERSAL_NODES,
        ],
    )?;
    let diagnostics = u32::try_from(unsigned(object.get(constants::key::DIAGNOSTICS)?)?)
        .map_err(|_| at(DecodeErrorClass::InvalidDerivation, value.offset))?;
    let source_bytes = unsigned(object.get(constants::key::SOURCE_BYTES)?)?;
    let string_bytes = unsigned(object.get(constants::key::STRING_BYTES)?)?;
    let numeric_digits = unsigned(object.get(constants::key::NUMERIC_DIGITS)?)?;
    let numeric_scale = unsigned(object.get(constants::key::NUMERIC_SCALE)?)?;
    let declarations = unsigned(object.get(constants::key::DECLARATIONS)?)?;
    let record_fields = unsigned(object.get(constants::key::RECORD_FIELDS)?)?;
    let nesting_depth = unsigned(object.get(constants::key::NESTING_DEPTH)?)?;
    let list_items = unsigned(object.get(constants::key::LIST_ITEMS)?)?;
    let traversal_nodes = unsigned(object.get(constants::key::TRAVERSAL_NODES)?)?;
    let invalid = |_| at(DecodeErrorClass::InvalidDerivation, value.offset);
    let limits = StructuralLimits::new(source_bytes, diagnostics).map_err(invalid)?;
    let limits = limits.with_string_bytes(string_bytes).map_err(invalid)?;
    let limits = limits
        .with_numeric_digits(numeric_digits)
        .map_err(invalid)?;
    let limits = limits.with_numeric_scale(numeric_scale).map_err(invalid)?;
    let limits = limits.with_declarations(declarations).map_err(invalid)?;
    let limits = limits.with_record_fields(record_fields).map_err(invalid)?;
    let limits = limits.with_nesting_depth(nesting_depth).map_err(invalid)?;
    let limits = limits.with_list_items(list_items).map_err(invalid)?;
    limits
        .with_traversal_nodes(traversal_nodes)
        .map_err(invalid)
}

/// Decodes successful-compilation resource facts.
fn decode_resource_facts(value: &LocatedValue) -> Result<ResourceFacts, DecodeError> {
    let object = Object::exact(
        value,
        &[
            constants::key::SOURCE_BYTES,
            constants::key::DECLARATIONS,
            constants::key::DIAGNOSTICS,
            constants::key::DECODED_STRING_BYTES,
        ],
    )?;
    Ok(ResourceFacts::new(
        unsigned(object.get(constants::key::SOURCE_BYTES)?)?,
        unsigned(object.get(constants::key::DECLARATIONS)?)?,
        unsigned(object.get(constants::key::DIAGNOSTICS)?)?,
        unsigned(object.get(constants::key::DECODED_STRING_BYTES)?)?,
    ))
}

/// Applies captured logical limits to subsequent CBOR parsing ceilings.
fn effective_limits(host: DecodeLimits, captured: StructuralLimits) -> DecodeLimits {
    host.with_nesting_depth(saturating_usize(captured.nesting_depth()))
        .with_container_items(saturating_usize(captured.traversal_nodes()))
        .with_text_bytes(saturating_usize(captured.string_bytes()))
        .with_traversal_nodes(saturating_usize(captured.traversal_nodes()))
}

/// Validates source, derivation, identity, and resource cross-section facts.
fn validate_cross_sections(artifacts: &CompilationArtifacts) -> Result<(), DecodeError> {
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
        return Err(error(DecodeErrorClass::InvalidDerivation));
    }
    validate_logical_identities(logical)?;
    validate_source_coverage(artifacts)?;
    validate_provenance_coverage(artifacts)?;
    Ok(())
}

/// Validates root module, symbol, identity, and canonical-name consistency.
fn validate_logical_identities(logical: &LogicalDocument) -> Result<(), DecodeError> {
    let module = logical.module();
    let mut previous_record = None;
    for record in logical.record_types() {
        if record.symbol_identity().module() != module
            || record.nominal_identity().module() != module
            || record.symbol_identity().declaration_name() != record.name()
            || previous_record.is_some_and(|name| name >= record.name())
        {
            return Err(error(DecodeErrorClass::InvalidLogicalIr));
        }
        previous_record = Some(record.name());
    }
    let mut previous_declaration = None;
    for declaration in logical.declarations() {
        if declaration.symbol_identity().module() != module
            || declaration.symbol_identity().declaration_name() != declaration.name()
            || previous_declaration.is_some_and(|name| name >= declaration.name())
        {
            return Err(error(DecodeErrorClass::InvalidLogicalIr));
        }
        previous_declaration = Some(declaration.name());
    }
    Ok(())
}

/// Validates exact one-to-one source-map ownership and span coverage.
fn validate_source_coverage(artifacts: &CompilationArtifacts) -> Result<(), DecodeError> {
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
        return Err(error(DecodeErrorClass::InvalidSourceMap));
    }
    Ok(())
}

/// Validates exact root coverage and canonical ordering for every provenance array.
fn validate_provenance_coverage(artifacts: &CompilationArtifacts) -> Result<(), DecodeError> {
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
        return Err(error(DecodeErrorClass::InvalidProvenance));
    }
    Ok(())
}

/// Maps trusted-model validation failures into frozen external result classes.
fn map_reader_error(error_value: ReaderError) -> DecodeError {
    let class = match error_value {
        ReaderError::MissingSourceMapEntry => DecodeErrorClass::InvalidSourceMap,
        ReaderError::InvalidFieldProvenance
        | ReaderError::InvalidReuseProvenance
        | ReaderError::InvalidReferenceEdge
        | ReaderError::MissingProvenanceRecord => DecodeErrorClass::InvalidProvenance,
        ReaderError::InvalidVocabularyContract
        | ReaderError::DuplicateElementId
        | ReaderError::DuplicateDeclarationName
        | ReaderError::TypeValueMismatch
        | ReaderError::InvalidRecordSchema
        | ReaderError::InvalidDeclarationFingerprint => DecodeErrorClass::InvalidLogicalIr,
    };
    error(class)
}

/// Returns the discriminator text without accepting a non-map sum.
fn discriminator(value: &LocatedValue) -> Result<&str, DecodeError> {
    let CborValue::Map(members) = &value.value else {
        return Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset));
    };
    let mut found = None;
    for (key, candidate) in members {
        if key == constants::key::KIND {
            if found.is_some() {
                return Err(at(DecodeErrorClass::InvalidEncodedSchema, candidate.offset));
            }
            found = Some(text(candidate)?);
        }
    }
    found.ok_or_else(|| at(DecodeErrorClass::InvalidEncodedSchema, value.offset))
}

/// Validates a discriminator-only sum object.
fn kind_object(value: &LocatedValue) -> Result<(), DecodeError> {
    Object::exact(value, &[constants::key::KIND]).map(|_| ())
}

/// Decodes a fingerprint byte string into its typed representation.
fn decode_fingerprint(value: &LocatedValue) -> Result<DeclarationFingerprint, DecodeError> {
    Ok(DeclarationFingerprint::from_digest(
        SemanticDigest::from_raw_bytes(digest_bytes(value)?),
    ))
}

/// Returns an exact text scalar.
fn text(value: &LocatedValue) -> Result<&str, DecodeError> {
    match &value.value {
        CborValue::Text(text_value) => Ok(text_value),
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Returns one valid non-protected lower-snake logical name.
fn snake_name(value: &LocatedValue) -> Result<&str, DecodeError> {
    let name = text(value)?;
    if language::is_snake_name(name) && !language::is_protected_name(name) {
        Ok(name)
    } else {
        Err(at(DecodeErrorClass::InvalidLogicalIr, value.offset))
    }
}

/// Returns one valid non-protected uppercase-leading logical name.
fn upper_name(value: &LocatedValue) -> Result<&str, DecodeError> {
    let name = text(value)?;
    if language::is_upper_name(name) && !language::is_protected_name(name) {
        Ok(name)
    } else {
        Err(at(DecodeErrorClass::InvalidLogicalIr, value.offset))
    }
}

/// Returns an exact unsigned scalar.
fn unsigned(value: &LocatedValue) -> Result<u64, DecodeError> {
    match value.value {
        CborValue::Unsigned(number) => Ok(number),
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Returns an exact signed scalar.
fn signed(value: &LocatedValue) -> Result<i64, DecodeError> {
    match value.value {
        CborValue::Unsigned(number) => i64::try_from(number)
            .map_err(|_| at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
        CborValue::Negative(number) => Ok(number),
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Returns an exact Boolean scalar.
fn boolean(value: &LocatedValue) -> Result<bool, DecodeError> {
    match value.value {
        CborValue::Boolean(boolean_value) => Ok(boolean_value),
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Returns an exact array.
fn array(value: &LocatedValue) -> Result<&[LocatedValue], DecodeError> {
    match &value.value {
        CborValue::Array(values) => Ok(values),
        _ => Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset)),
    }
}

/// Returns a copied exact 32-byte digest.
fn digest_bytes(value: &LocatedValue) -> Result<[u8; 32], DecodeError> {
    let CborValue::Bytes(bytes) = &value.value else {
        return Err(at(DecodeErrorClass::InvalidEncodedSchema, value.offset));
    };
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| at(DecodeErrorClass::InvalidEncodedSchema, value.offset))
}

/// Decodes one array of text values.
fn text_array(value: &LocatedValue) -> Result<Vec<String>, DecodeError> {
    array(value)?
        .iter()
        .map(|item| text(item).map(str::to_owned))
        .collect()
}

/// Requires one exact supported version value.
fn require_version(value: &LocatedValue, expected: &str) -> Result<(), DecodeError> {
    if text(value)? == expected {
        Ok(())
    } else {
        Err(at(DecodeErrorClass::UnsupportedVersion, value.offset))
    }
}

/// Returns whether strings are unique and strictly increasing.
fn is_strictly_ordered(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

/// Checks one collection length against a captured u64 limit.
fn check_collection(length: usize, limit: u64) -> Result<(), DecodeError> {
    if u64::try_from(length).unwrap_or(u64::MAX) > limit {
        Err(error(DecodeErrorClass::EncodedSizeLimit))
    } else {
        Ok(())
    }
}

/// Checks cooperative cancellation before major phases.
fn check_cancelled(cancellation: &CancellationToken) -> Result<(), DecodeError> {
    if cancellation.is_cancelled() {
        Err(error(DecodeErrorClass::Cancelled))
    } else {
        Ok(())
    }
}

/// Reads one big-endian u16 from checked frame bytes.
fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DecodeError> {
    Ok(u16::from_be_bytes(read_fixed(bytes, offset)?))
}

/// Reads one big-endian u32 from checked frame bytes.
fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DecodeError> {
    Ok(u32::from_be_bytes(read_fixed(bytes, offset)?))
}

/// Reads one big-endian u64 from checked frame bytes.
fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, DecodeError> {
    Ok(u64::from_be_bytes(read_fixed(bytes, offset)?))
}

/// Reads one fixed frame field without unchecked slicing.
fn read_fixed<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], DecodeError> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| error(DecodeErrorClass::MalformedFrame))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| {
            at(
                DecodeErrorClass::MalformedFrame,
                u64_from_usize(offset).unwrap_or(0),
            )
        })?
        .try_into()
        .map_err(|_| error(DecodeErrorClass::InternalDefect))
}

/// Converts u64 framing values to host offsets without truncation.
fn usize_from_u64(value: u64) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| error(DecodeErrorClass::MalformedFrame))
}

/// Converts u32 framing values to host offsets without truncation.
fn usize_from_u32(value: u32) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| error(DecodeErrorClass::MalformedFrame))
}

/// Converts host offsets to bounded diagnostic offsets.
fn u64_from_usize(value: usize) -> Result<u64, DecodeError> {
    u64::try_from(value).map_err(|_| error(DecodeErrorClass::InternalDefect))
}

/// Saturates a captured u64 limit to the host word width.
fn saturating_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// Constructs an offset-free bounded error.
const fn error(class: DecodeErrorClass) -> DecodeError {
    DecodeError::new(class, None)
}

/// Constructs a bounded error at one absolute byte offset.
const fn at(class: DecodeErrorClass, offset: u64) -> DecodeError {
    DecodeError::new(class, Some(offset))
}
