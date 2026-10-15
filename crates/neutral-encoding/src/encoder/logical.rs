// SPDX-License-Identifier: Apache-2.0

//! Project typed document meaning into the logical CBOR section.
//!
//! Modules, symbols, records, declarations, vocabulary contracts and recursive
//! values follow the wire schema here. Source locations and producer metadata
//! are intentionally emitted elsewhere. Deterministic serialization preserves
//! the supplied logical representation; it is not a replacement for identity framing.

use super::{
    CborWriter, Declaration, EncodingError, IdentityReference, LogicalDocument,
    LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity, NominalTypeIdentity,
    RecordFieldSchema, RecordTypeDefinition, RecordValueField, ResolvedType, VocabularyContract,
    VocabularyFieldContract, VocabularyIdentity, VocabularyTypeContract, VocabularyTypeIdentity,
    check_depth, constants, encode_text_array, field_key, field_text, field_unsigned, kind_only,
};

/// Encodes the complete logical payload section.
pub(super) fn encode_logical(document: &LogicalDocument) -> Result<Vec<u8>, EncodingError> {
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
    writer.map(5)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        record.element_id().get(),
    )?;
    field_key(writer, constants::key::SYMBOL)?;
    encode_symbol(writer, record.symbol_identity())?;
    field_key(writer, constants::key::FINGERPRINT)?;
    writer.bytes(&record.fingerprint().digest().as_bytes())?;
    field_key(writer, constants::key::IDENTITY)?;
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
    writer.map(6)?;
    field_unsigned(
        writer,
        constants::key::ELEMENT_ID,
        declaration.element_id().get(),
    )?;
    field_key(writer, constants::key::SYMBOL)?;
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
    writer.map(2)?;
    field_key(writer, constants::key::IDENTITY)?;
    encode_vocabulary_identity(writer, vocabulary.identity())?;
    field_key(writer, constants::key::TYPES)?;
    writer.array(vocabulary.types().len())?;
    for contract in vocabulary.types() {
        encode_vocabulary_type_contract(writer, contract, depth + 2)?;
    }
    Ok(())
}

/// Encodes one vocabulary identity.
pub(super) fn encode_vocabulary_identity(
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
    writer.map(2)?;
    field_key(writer, constants::key::IDENTITY)?;
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
