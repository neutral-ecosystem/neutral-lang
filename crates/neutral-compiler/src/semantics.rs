// SPDX-License-Identifier: Apache-2.0

//! Private semantic validation and public IR lowering.

use crate::diagnostics;
use crate::frontend::{
    ParsedBinding, ParsedDeclaration, ParsedRecord, ParsedType, ParsedValue, ParsedValueField,
};
use crate::language::names;
use crate::{CompilationFailure, CompilationFailureDetail, LANGUAGE_BEHAVIOR_VERSION};
use neutral_core::{
    ByteSpan, Diagnostic, DiagnosticCode, DiagnosticLayer, DiagnosticSeverity, ResultClass,
    SourceContentDigest, SourceLocation, StructuralLimits,
};
use neutral_ir::{
    AcceptancePartition, CompilationArtifacts, Declaration, DeclarationFingerprint,
    DerivationManifest, ElementId, ExactNumber, FieldProvenanceRecord, LogicalDocument,
    LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity, NominalTypeIdentity, Normalization,
    ProvenanceRecord, RecordFieldSchema, RecordTypeDefinition, RecordValue, RecordValueField,
    ResolvedType, ResourceFacts, SourceMap, SourceMapEntry, ValueOrigin,
};
use std::collections::{BTreeMap, BTreeSet};

/// A private semantic failure before authoritative IR construction.
pub(super) struct SemanticError {
    /// Stable semantic diagnostic code.
    code: &'static str,
    /// Exact original-byte primary span.
    span: ByteSpan,
    /// Stable failure class.
    class: ResultClass,
    /// Public bounded failure detail.
    detail: CompilationFailureDetail,
    /// Diagnostic ownership layer.
    layer: DiagnosticLayer,
}

impl SemanticError {
    /// Creates one semantic type, name, or value failure.
    fn semantic(code: &'static str, span: ByteSpan) -> Self {
        Self {
            code,
            span,
            class: ResultClass::Semantics,
            detail: CompilationFailureDetail::SemanticRejected,
            layer: DiagnosticLayer::Semantics,
        }
    }

    /// Creates one deterministic captured resource-limit failure.
    fn resource(code: &'static str, span: ByteSpan) -> Self {
        Self {
            code,
            span,
            class: ResultClass::Resource,
            detail: CompilationFailureDetail::ResourceLimitExceeded,
            layer: DiagnosticLayer::Resource,
        }
    }

    /// Converts a private semantic failure into a bounded public failure.
    pub(super) fn into_failure(self, source: SourceContentDigest) -> CompilationFailure {
        let code = DiagnosticCode::new(self.code).expect("semantic diagnostic code must be ASCII");
        CompilationFailure {
            class: self.class,
            detail: self.detail,
            diagnostics: vec![Diagnostic::new(
                code,
                self.layer,
                DiagnosticSeverity::Error,
                SourceLocation::new(source, self.span),
                Vec::new(),
                false,
            )],
        }
    }
}

/// Root declaration categories used during the collection pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootKind {
    /// A nominal record type declaration.
    Record,
    /// An immutable value binding.
    Binding,
}

/// Collected one-scope declarations split by kind after duplicate validation.
type CollectedRoots = (
    BTreeMap<String, RootKind>,
    BTreeMap<String, ParsedRecord>,
    BTreeMap<String, ParsedBinding>,
);

/// Lowered binding products and their companion evidence.
type LoweredBindings = (
    Vec<Declaration>,
    Vec<SourceMapEntry>,
    Vec<ProvenanceRecord>,
    Vec<FieldProvenanceRecord>,
    u64,
);

/// One compiler-private resolved field plus its closed-default candidate.
#[derive(Clone, Debug)]
struct ResolvedSchemaField {
    /// Validated field name.
    name: String,
    /// Fully resolved field type.
    resolved_type: ResolvedType,
    /// Parsed closed-default candidate.
    default_value: Option<ParsedValue>,
    /// Exact default span, when present.
    default_span: Option<ByteSpan>,
}

/// Mutable context shared while lowering one binding value tree.
struct BindingValueContext<'a> {
    /// Public nominal record schemas indexed by name.
    records: &'a BTreeMap<String, &'a RecordTypeDefinition>,
    /// Captured deterministic limits.
    limits: StructuralLimits,
    /// Owning binding element identifier.
    element_id: ElementId,
    /// Accumulated canonical field provenance.
    field_provenance: &'a mut Vec<FieldProvenanceRecord>,
}

/// Validates the private model and lowers complete immutable artifacts.
pub(super) fn lower(
    unit: crate::frontend::ParsedUnit,
    source_digest: SourceContentDigest,
    source_length: usize,
    limits: StructuralLimits,
) -> Result<CompilationArtifacts, SemanticError> {
    let _retained_private_trivia_count = unit.trivia_count();
    validate_root_name(&unit.module.name, unit.module.name_span, false)?;
    let (root_kinds, records, bindings) = collect_roots(unit.declarations)?;
    let module_identity =
        LogicalModuleIdentity::new(LANGUAGE_BEHAVIOR_VERSION, unit.module.name.clone());
    validate_record_fields(&records)?;
    validate_embedded_record_graph(&records)?;
    let resolved_schemas = resolve_record_schemas(&records, &root_kinds, &module_identity)?;
    let element_ids = allocate_element_ids(&root_kinds);
    let (record_definitions, mut source_entries, default_string_bytes) = lower_record_definitions(
        &records,
        &resolved_schemas,
        &module_identity,
        &element_ids,
        limits,
    )?;
    let (declarations, binding_entries, mut provenance, field_provenance, decoded_string_bytes) =
        lower_bindings(
            &bindings,
            &root_kinds,
            &module_identity,
            &element_ids,
            &record_definitions,
            limits,
        )?;
    source_entries.extend(binding_entries);
    source_entries.sort_by_key(|entry| entry.element_id());
    provenance.sort_by_key(|record| record.element_id());
    let logical_document =
        LogicalDocument::with_record_types(module_identity, record_definitions, declarations);
    let source_byte_length = u64::try_from(source_length).unwrap_or(u64::MAX);
    let source_map = SourceMap::new(
        source_digest,
        source_byte_length,
        unit.module.span,
        source_entries,
    );
    let declaration_count = u64::try_from(root_kinds.len()).unwrap_or(u64::MAX);
    let resource_facts = ResourceFacts::new(
        source_byte_length,
        declaration_count,
        0,
        decoded_string_bytes.saturating_add(default_string_bytes),
    );
    let derivation = DerivationManifest::new(
        LANGUAGE_BEHAVIOR_VERSION,
        source_digest,
        AcceptancePartition::from_limits(limits),
        resource_facts,
    );
    Ok(
        CompilationArtifacts::new(logical_document, source_map, provenance, derivation)
            .with_field_provenance(field_provenance),
    )
}

/// Collects all root names before nominal resolution and enforces one scope.
fn collect_roots(declarations: Vec<ParsedDeclaration>) -> Result<CollectedRoots, SemanticError> {
    let mut root_kinds = BTreeMap::new();
    let mut records = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    for declaration in declarations {
        let kind = match &declaration {
            ParsedDeclaration::Record(_) => RootKind::Record,
            ParsedDeclaration::Binding(_) => RootKind::Binding,
        };
        validate_root_name(
            declaration.name(),
            declaration.name_span(),
            kind == RootKind::Record,
        )?;
        if root_kinds
            .insert(declaration.name().to_owned(), kind)
            .is_some()
        {
            return Err(SemanticError::semantic(
                diagnostics::DUPLICATE_DECLARATION,
                declaration.name_span(),
            ));
        }
        match declaration {
            ParsedDeclaration::Record(record) => {
                records.insert(record.name.clone(), record);
            }
            ParsedDeclaration::Binding(binding) => {
                bindings.insert(binding.name.clone(), binding);
            }
        }
    }
    Ok((root_kinds, records, bindings))
}

/// Allocates stable graph-local element IDs from canonical root-name order.
fn allocate_element_ids(root_kinds: &BTreeMap<String, RootKind>) -> BTreeMap<String, ElementId> {
    root_kinds
        .keys()
        .cloned()
        .enumerate()
        .map(|(index, name)| {
            (
                name,
                ElementId::new(u64::try_from(index).unwrap_or(u64::MAX)),
            )
        })
        .collect()
}

/// Lowers canonical public record definitions and their source entries.
fn lower_record_definitions(
    records: &BTreeMap<String, ParsedRecord>,
    schemas: &BTreeMap<String, Vec<ResolvedSchemaField>>,
    module: &LogicalModuleIdentity,
    element_ids: &BTreeMap<String, ElementId>,
    limits: StructuralLimits,
) -> Result<(Vec<RecordTypeDefinition>, Vec<SourceMapEntry>, u64), SemanticError> {
    let mut definitions = Vec::new();
    let mut source_entries = Vec::new();
    let mut decoded_string_bytes = 0_u64;
    for (name, record) in records {
        let fields = schemas[name]
            .iter()
            .map(|field| {
                let schema =
                    RecordFieldSchema::new(field.name.clone(), field.resolved_type.clone());
                match (&field.default_value, field.default_span) {
                    (Some(default), Some(span)) => {
                        lower_closed_default(&field.resolved_type, default, span, schemas, limits)
                            .map(|(value, bytes)| {
                                decoded_string_bytes = decoded_string_bytes.saturating_add(bytes);
                                schema.with_default(value)
                            })
                    }
                    (None, None) => Ok(schema),
                    _ => unreachable!("parsed defaults always carry matching spans"),
                }
            })
            .collect::<Result<Vec<_>, SemanticError>>()?;
        let fingerprint = DeclarationFingerprint::for_record(&fields)
            .map_err(|_| SemanticError::semantic(diagnostics::TYPE_MISMATCH, record.body_span))?;
        let element_id = element_ids[name];
        definitions.push(RecordTypeDefinition::new(
            element_id,
            ModuleSymbolIdentity::new(module.clone(), name.clone()),
            fingerprint,
            NominalTypeIdentity::new(module.clone(), name.clone()),
            fields,
        ));
        source_entries.push(SourceMapEntry::new(
            element_id,
            record.span,
            record.name_span,
            record.name_span,
            record.body_span,
        ));
    }
    Ok((definitions, source_entries, decoded_string_bytes))
}

/// Lowers canonical binding values and all binding-owned companion evidence.
fn lower_bindings(
    bindings: &BTreeMap<String, ParsedBinding>,
    root_kinds: &BTreeMap<String, RootKind>,
    module: &LogicalModuleIdentity,
    element_ids: &BTreeMap<String, ElementId>,
    record_definitions: &[RecordTypeDefinition],
    limits: StructuralLimits,
) -> Result<LoweredBindings, SemanticError> {
    let record_lookup = record_definitions
        .iter()
        .map(|record| (record.name().to_owned(), record))
        .collect::<BTreeMap<_, _>>();
    let mut declarations = Vec::new();
    let mut source_entries = Vec::new();
    let mut provenance = Vec::new();
    let mut field_provenance = Vec::new();
    let mut decoded_string_bytes = 0_u64;
    for (name, binding) in bindings {
        let resolved_type = resolve_type(
            &binding.declared_type,
            binding.type_span,
            root_kinds,
            module,
        )?;
        let mut context = BindingValueContext {
            records: &record_lookup,
            limits,
            element_id: element_ids[name],
            field_provenance: &mut field_provenance,
        };
        let mut field_path = Vec::new();
        let (value, normalization, decoded_bytes) = lower_value(
            &resolved_type,
            &binding.value,
            binding.value_span,
            &mut field_path,
            &mut context,
        )?;
        decoded_string_bytes = decoded_string_bytes.saturating_add(decoded_bytes);
        let fingerprint = DeclarationFingerprint::for_binding(&resolved_type, &value)
            .map_err(|_| SemanticError::semantic(diagnostics::TYPE_MISMATCH, binding.value_span))?;
        let element_id = element_ids[name];
        declarations.push(Declaration::new(
            element_id,
            ModuleSymbolIdentity::new(module.clone(), name.clone()),
            fingerprint,
            name.clone(),
            resolved_type,
            value,
        ));
        source_entries.push(SourceMapEntry::new(
            element_id,
            binding.span,
            binding.type_span,
            binding.name_span,
            binding.value_span,
        ));
        provenance.push(ProvenanceRecord::new(
            element_id,
            ValueOrigin::ExplicitSource,
            normalization,
        ));
    }
    Ok((
        declarations,
        source_entries,
        provenance,
        field_provenance,
        decoded_string_bytes,
    ))
}

/// Validates one root name against its required frozen category.
fn validate_root_name(value: &str, span: ByteSpan, record: bool) -> Result<(), SemanticError> {
    if names::is_protected_name(value) {
        return Err(SemanticError::semantic(diagnostics::PROTECTED_NAME, span));
    }
    let expected = if record {
        AsciiNameCategory::Upper
    } else {
        AsciiNameCategory::Snake
    };
    if classify_ascii_name(value) == expected {
        Ok(())
    } else {
        Err(SemanticError::semantic(diagnostics::INVALID_NAME, span))
    }
}

/// Validates required record field names and duplicate ownership.
fn validate_record_fields(records: &BTreeMap<String, ParsedRecord>) -> Result<(), SemanticError> {
    for record in records.values() {
        let mut names_seen = BTreeSet::new();
        for field in &record.fields {
            if names::is_protected_name(&field.name) {
                return Err(SemanticError::semantic(
                    diagnostics::PROTECTED_NAME,
                    field.name_span,
                ));
            }
            if classify_ascii_name(&field.name) != AsciiNameCategory::Snake {
                return Err(SemanticError::semantic(
                    diagnostics::INVALID_NAME,
                    field.name_span,
                ));
            }
            if !names_seen.insert(field.name.as_str()) {
                return Err(SemanticError::semantic(
                    diagnostics::DUPLICATE_RECORD_FIELD,
                    field.name_span,
                ));
            }
        }
    }
    Ok(())
}

/// Resolves every record field type after the complete root collection pass.
fn resolve_record_schemas(
    records: &BTreeMap<String, ParsedRecord>,
    root_kinds: &BTreeMap<String, RootKind>,
    module: &LogicalModuleIdentity,
) -> Result<BTreeMap<String, Vec<ResolvedSchemaField>>, SemanticError> {
    records
        .iter()
        .map(|(name, record)| {
            let mut fields = record
                .fields
                .iter()
                .map(|field| {
                    resolve_type(&field.declared_type, field.type_span, root_kinds, module).map(
                        |resolved_type| ResolvedSchemaField {
                            name: field.name.clone(),
                            resolved_type,
                            default_value: field.default_value.clone(),
                            default_span: field.default_span,
                        },
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            fields.sort_by(|left, right| left.name.cmp(&right.name));
            Ok((name.clone(), fields))
        })
        .collect()
}

/// Resolves one private scalar or nominal type into public IR identity.
fn resolve_type(
    parsed: &ParsedType,
    span: ByteSpan,
    root_kinds: &BTreeMap<String, RootKind>,
    module: &LogicalModuleIdentity,
) -> Result<ResolvedType, SemanticError> {
    match parsed {
        ParsedType::Num => Ok(ResolvedType::Num),
        ParsedType::String => Ok(ResolvedType::String),
        ParsedType::Bool => Ok(ResolvedType::Bool),
        ParsedType::List(inner) => {
            resolve_type(inner, span, root_kinds, module).map(ResolvedType::list)
        }
        ParsedType::Record(name) => match root_kinds.get(name) {
            Some(RootKind::Record) => Ok(ResolvedType::Record(NominalTypeIdentity::new(
                module.clone(),
                name.clone(),
            ))),
            Some(RootKind::Binding) => Err(SemanticError::semantic(
                diagnostics::WRONG_DECLARATION_KIND,
                span,
            )),
            None => Err(SemanticError::semantic(diagnostics::UNKNOWN_TYPE, span)),
        },
        ParsedType::Nullable(inner) => {
            resolve_type(inner, span, root_kinds, module).map(ResolvedType::nullable)
        }
    }
}

/// Rejects every embedded nominal record cycle, including nullable edges.
fn validate_embedded_record_graph(
    records: &BTreeMap<String, ParsedRecord>,
) -> Result<(), SemanticError> {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for name in records.keys() {
        visit_record(name, records, &mut visiting, &mut visited)?;
    }
    Ok(())
}

/// Visits one nominal record in deterministic depth-first order.
fn visit_record(
    name: &str,
    records: &BTreeMap<String, ParsedRecord>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), SemanticError> {
    if visited.contains(name) {
        return Ok(());
    }
    visiting.insert(name.to_owned());
    let record = records
        .get(name)
        .expect("record graph visits only collected record names");
    for field in &record.fields {
        let Some(target) = embedded_record_target(&field.declared_type) else {
            continue;
        };
        if !records.contains_key(target) {
            continue;
        }
        if visiting.contains(target) {
            return Err(SemanticError::semantic(
                diagnostics::EMBEDDED_RECORD_RECURSION,
                field.type_span,
            ));
        }
        visit_record(target, records, visiting, visited)?;
    }
    visiting.remove(name);
    visited.insert(name.to_owned());
    Ok(())
}

/// Returns the nominal target embedded by a currently active type.
fn embedded_record_target(parsed: &ParsedType) -> Option<&str> {
    match parsed {
        ParsedType::Record(name) => Some(name),
        ParsedType::Nullable(inner) | ParsedType::List(inner) => embedded_record_target(inner),
        ParsedType::Num | ParsedType::String | ParsedType::Bool => None,
    }
}

/// Lowers one closed default without creating value or reference dependencies.
fn lower_closed_default(
    expected: &ResolvedType,
    value: &ParsedValue,
    value_span: ByteSpan,
    schemas: &BTreeMap<String, Vec<ResolvedSchemaField>>,
    limits: StructuralLimits,
) -> Result<(LogicalValue, u64), SemanticError> {
    match (expected, value) {
        (_, ParsedValue::Name(_)) => Err(SemanticError::semantic(
            diagnostics::NON_CONSTANT_DEFAULT,
            value_span,
        )),
        (ResolvedType::Nullable(_), ParsedValue::Null) => Ok((LogicalValue::Null, 0)),
        (ResolvedType::Nullable(inner), value) => {
            lower_closed_default(inner, value, value_span, schemas, limits)
        }
        (ResolvedType::Num, ParsedValue::Number(spelling)) => {
            let number =
                ExactNumber::from_source(spelling, limits.numeric_digits(), limits.numeric_scale())
                    .map_err(|error| match error {
                        neutral_ir::IrError::InvalidExactNumber => {
                            SemanticError::semantic(diagnostics::INVALID_NUMBER, value_span)
                        }
                        neutral_ir::IrError::ExactNumberLimitExceeded => {
                            SemanticError::resource(diagnostics::NUMBER_LIMIT_EXCEEDED, value_span)
                        }
                    })?;
            Ok((LogicalValue::Number(number), 0))
        }
        (ResolvedType::String, ParsedValue::String(value)) => {
            let decoded_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
            if decoded_bytes > limits.string_bytes() {
                return Err(SemanticError::resource(
                    diagnostics::STRING_LIMIT_EXCEEDED,
                    value_span,
                ));
            }
            Ok((LogicalValue::String(value.clone()), decoded_bytes))
        }
        (ResolvedType::Bool, ParsedValue::Boolean(value)) => Ok((LogicalValue::Boolean(*value), 0)),
        (ResolvedType::Record(identity), ParsedValue::Record(fields)) => {
            lower_closed_record_default(identity, fields, value_span, schemas, limits)
        }
        (ResolvedType::List(inner), ParsedValue::List(items)) => {
            let mut lowered = Vec::with_capacity(items.len());
            let mut decoded_string_bytes = 0_u64;
            for item in items {
                let (value, bytes) =
                    lower_closed_default(inner, &item.value, item.span, schemas, limits)?;
                decoded_string_bytes = decoded_string_bytes.saturating_add(bytes);
                lowered.push(value);
            }
            Ok((LogicalValue::List(lowered), decoded_string_bytes))
        }
        _ => Err(SemanticError::semantic(
            diagnostics::TYPE_MISMATCH,
            value_span,
        )),
    }
}

/// Materializes one recursively closed contextual record default.
fn lower_closed_record_default(
    identity: &NominalTypeIdentity,
    fields: &[ParsedValueField],
    value_span: ByteSpan,
    schemas: &BTreeMap<String, Vec<ResolvedSchemaField>>,
    limits: StructuralLimits,
) -> Result<(LogicalValue, u64), SemanticError> {
    let schema = &schemas[identity.name()];
    let expected = schema
        .iter()
        .map(|field| (field.name.as_str(), field))
        .collect::<BTreeMap<_, _>>();
    let mut supplied = BTreeMap::new();
    for field in fields {
        if !expected.contains_key(field.name.as_str()) {
            return Err(SemanticError::semantic(
                diagnostics::UNKNOWN_RECORD_FIELD,
                field.name_span,
            ));
        }
        if supplied.insert(field.name.as_str(), field).is_some() {
            return Err(SemanticError::semantic(
                diagnostics::DUPLICATE_VALUE_FIELD,
                field.name_span,
            ));
        }
    }
    let mut lowered_fields = Vec::with_capacity(schema.len());
    let mut decoded_string_bytes = 0_u64;
    for schema_field in schema {
        let (value, decoded_bytes) = if let Some(field) = supplied.get(schema_field.name.as_str()) {
            lower_closed_default(
                &schema_field.resolved_type,
                &field.value,
                field.value_span,
                schemas,
                limits,
            )?
        } else if let (Some(default), Some(span)) =
            (&schema_field.default_value, schema_field.default_span)
        {
            lower_closed_default(&schema_field.resolved_type, default, span, schemas, limits)?
        } else {
            return Err(SemanticError::semantic(
                diagnostics::MISSING_RECORD_FIELD,
                value_span,
            ));
        };
        decoded_string_bytes = decoded_string_bytes.saturating_add(decoded_bytes);
        lowered_fields.push(RecordValueField::new(&schema_field.name, value));
    }
    Ok((
        LogicalValue::Record(RecordValue::new(identity.clone(), lowered_fields)),
        decoded_string_bytes,
    ))
}

/// Lowers one value against its completely resolved expected type.
fn lower_value(
    expected: &ResolvedType,
    value: &ParsedValue,
    value_span: ByteSpan,
    field_path: &mut Vec<String>,
    context: &mut BindingValueContext<'_>,
) -> Result<(LogicalValue, Normalization, u64), SemanticError> {
    match (expected, value) {
        (ResolvedType::Nullable(_), ParsedValue::Null) => {
            Ok((LogicalValue::Null, Normalization::NullIdentity, 0))
        }
        (ResolvedType::Nullable(inner), value) => {
            lower_value(inner, value, value_span, field_path, context)
        }
        (ResolvedType::Num, ParsedValue::Number(spelling)) => {
            let number = ExactNumber::from_source(
                spelling,
                context.limits.numeric_digits(),
                context.limits.numeric_scale(),
            )
            .map_err(|error| match error {
                neutral_ir::IrError::InvalidExactNumber => {
                    SemanticError::semantic(diagnostics::INVALID_NUMBER, value_span)
                }
                neutral_ir::IrError::ExactNumberLimitExceeded => {
                    SemanticError::resource(diagnostics::NUMBER_LIMIT_EXCEEDED, value_span)
                }
            })?;
            Ok((
                LogicalValue::Number(number),
                Normalization::ExactNumberCanonicalization,
                0,
            ))
        }
        (ResolvedType::String, ParsedValue::String(value)) => {
            let decoded_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
            if decoded_bytes > context.limits.string_bytes() {
                return Err(SemanticError::resource(
                    diagnostics::STRING_LIMIT_EXCEEDED,
                    value_span,
                ));
            }
            Ok((
                LogicalValue::String(value.clone()),
                Normalization::StringEscapeDecoding,
                decoded_bytes,
            ))
        }
        (ResolvedType::Bool, ParsedValue::Boolean(value)) => Ok((
            LogicalValue::Boolean(*value),
            Normalization::BooleanIdentity,
            0,
        )),
        (ResolvedType::Record(identity), ParsedValue::Record(fields)) => {
            lower_record_value(identity, fields, value_span, field_path, context)
        }
        (ResolvedType::List(inner), ParsedValue::List(items)) => {
            let mut lowered = Vec::with_capacity(items.len());
            let mut decoded_string_bytes = 0_u64;
            for (index, item) in items.iter().enumerate() {
                field_path.push(index.to_string());
                let (value, _, bytes) =
                    lower_value(inner, &item.value, item.span, field_path, context)?;
                field_path.pop();
                decoded_string_bytes = decoded_string_bytes.saturating_add(bytes);
                lowered.push(value);
            }
            Ok((
                LogicalValue::List(lowered),
                Normalization::ListContextualization,
                decoded_string_bytes,
            ))
        }
        _ => Err(SemanticError::semantic(
            diagnostics::TYPE_MISMATCH,
            value_span,
        )),
    }
}

/// Validates and lowers one contextual record value in canonical field order.
fn lower_record_value(
    identity: &NominalTypeIdentity,
    fields: &[ParsedValueField],
    value_span: ByteSpan,
    field_path: &mut Vec<String>,
    context: &mut BindingValueContext<'_>,
) -> Result<(LogicalValue, Normalization, u64), SemanticError> {
    let schema = context
        .records
        .get(identity.name())
        .expect("resolved record identities must have schemas");
    let expected = schema
        .fields()
        .iter()
        .map(|field| (field.name(), field))
        .collect::<BTreeMap<_, _>>();
    let mut supplied = BTreeMap::new();
    for field in fields {
        if !expected.contains_key(field.name.as_str()) {
            return Err(SemanticError::semantic(
                diagnostics::UNKNOWN_RECORD_FIELD,
                field.name_span,
            ));
        }
        if supplied.insert(field.name.as_str(), field).is_some() {
            return Err(SemanticError::semantic(
                diagnostics::DUPLICATE_VALUE_FIELD,
                field.name_span,
            ));
        }
    }
    for field in schema.fields() {
        if !supplied.contains_key(field.name()) && field.is_required() {
            return Err(SemanticError::semantic(
                diagnostics::MISSING_RECORD_FIELD,
                value_span,
            ));
        }
    }

    let mut lowered_fields = Vec::with_capacity(schema.fields().len());
    let mut decoded_string_bytes = 0_u64;
    for schema_field in schema.fields() {
        field_path.push(schema_field.name().to_owned());
        let (value, decoded_bytes) = if let Some(supplied_field) = supplied.get(schema_field.name())
        {
            context.field_provenance.push(FieldProvenanceRecord::new(
                context.element_id,
                field_path.clone(),
                ValueOrigin::ExplicitRecordField,
            ));
            let (value, _, decoded_bytes) = lower_value(
                schema_field.resolved_type(),
                &supplied_field.value,
                supplied_field.value_span,
                field_path,
                context,
            )?;
            (value, decoded_bytes)
        } else {
            let default = schema_field
                .default_value()
                .expect("only defaulted fields may be omitted after validation")
                .clone();
            context.field_provenance.push(FieldProvenanceRecord::new(
                context.element_id,
                field_path.clone(),
                ValueOrigin::UserRecordDefault,
            ));
            let decoded_bytes = logical_string_bytes(&default);
            (default, decoded_bytes)
        };
        field_path.pop();
        decoded_string_bytes = decoded_string_bytes.saturating_add(decoded_bytes);
        lowered_fields.push(RecordValueField::new(schema_field.name(), value));
    }
    Ok((
        LogicalValue::Record(RecordValue::new(identity.clone(), lowered_fields)),
        Normalization::RecordContextualization,
        decoded_string_bytes,
    ))
}

/// Counts decoded string bytes retained recursively by one final logical value.
fn logical_string_bytes(value: &LogicalValue) -> u64 {
    match value {
        LogicalValue::String(value) => u64::try_from(value.len()).unwrap_or(u64::MAX),
        LogicalValue::Record(record) => record
            .fields()
            .iter()
            .map(|field| logical_string_bytes(field.value()))
            .fold(0_u64, u64::saturating_add),
        LogicalValue::List(items) => items
            .iter()
            .map(logical_string_bytes)
            .fold(0_u64, u64::saturating_add),
        LogicalValue::Number(_) | LogicalValue::Boolean(_) | LogicalValue::Null => 0,
    }
}

/// Complete frozen ASCII identifier categories, independent of locale behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AsciiNameCategory {
    /// `[a-z][a-z0-9]*("_"[a-z][a-z0-9]*)*`.
    Snake,
    /// `[A-Z][A-Za-z0-9]*`.
    Upper,
    /// Any other spelling.
    Invalid,
}

/// Classifies one spelling into the frozen ASCII identifier categories.
fn classify_ascii_name(value: &str) -> AsciiNameCategory {
    let snake = !value.is_empty()
        && value.split('_').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if snake {
        return AsciiNameCategory::Snake;
    }
    let mut bytes = value.bytes();
    let upper = bytes.next().is_some_and(|byte| byte.is_ascii_uppercase())
        && bytes.all(|byte| byte.is_ascii_alphanumeric());
    if upper {
        AsciiNameCategory::Upper
    } else {
        AsciiNameCategory::Invalid
    }
}

#[cfg(test)]
/// Unit tests for semantic name validation and collection helpers.
mod tests {
    use super::{AsciiNameCategory, classify_ascii_name};
    use crate::language::names;

    #[test]
    /// Verifies exact ASCII snake and uppercase-leading name categories.
    fn unit_semantics_validates_frozen_names() {
        assert_eq!(classify_ascii_name("answer_two2"), AsciiNameCategory::Snake);
        assert_eq!(classify_ascii_name("Record2"), AsciiNameCategory::Upper);
        assert_eq!(
            classify_ascii_name("Record_Name"),
            AsciiNameCategory::Invalid
        );
        assert_eq!(
            classify_ascii_name("answer__two"),
            AsciiNameCategory::Invalid
        );
    }

    #[test]
    /// Verifies the frozen core namespace cannot be redeclared.
    fn unit_semantics_protects_core_names() {
        assert!(names::is_protected_name(names::NUM));
        assert!(names::is_protected_name(names::RECORD));
        assert!(!names::is_protected_name("answer"));
    }
}
