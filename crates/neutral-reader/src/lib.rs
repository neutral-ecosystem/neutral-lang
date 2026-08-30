// SPDX-License-Identifier: Apache-2.0

//! Validated, immutable access to Neutral artifacts.
//!
//! This crate owns validation at the external artifact boundary and the reader
//! views exposed after validation. It treats encoded data as untrusted and must
//! not acquire inputs or depend on compiler-private representations.

use neutral_core::SourceLocation;
use neutral_ir::{
    CompilationArtifacts, Declaration, LogicalValue, RecordTypeDefinition, ResolvedType,
    ValueOrigin,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub use neutral_ir::ElementId;

/// Immutable typed traversal over already-validated in-process artifacts.
#[derive(Clone, Debug)]
pub struct ValidatedDocument {
    /// Shared immutable compiler artifacts.
    artifacts: Arc<CompilationArtifacts>,
}

impl ValidatedDocument {
    /// Validates cross-artifact indexes before exposing immutable reader views.
    ///
    /// # Errors
    ///
    /// Returns a fail-closed error for duplicate elements or missing source and
    /// provenance records.
    pub fn from_compiler_output(artifacts: Arc<CompilationArtifacts>) -> Result<Self, ReaderError> {
        validate_artifacts(&artifacts)?;
        Ok(Self { artifacts })
    }

    /// Returns the logical module name.
    #[must_use]
    pub fn module_name(&self) -> &str {
        self.artifacts.logical_document().module().module_name()
    }

    /// Returns immutable declarations in deterministic order.
    #[must_use]
    pub fn declarations(&self) -> &[Declaration] {
        self.artifacts.logical_document().declarations()
    }

    /// Returns nominal record schemas in canonical name order.
    #[must_use]
    pub fn record_types(&self) -> &[RecordTypeDefinition] {
        self.artifacts.logical_document().record_types()
    }

    /// Finds one nominal record schema by its validated name.
    #[must_use]
    pub fn record_type_by_name(&self, name: &str) -> Option<&RecordTypeDefinition> {
        self.artifacts.logical_document().record_type_by_name(name)
    }

    /// Finds one declaration by its validated source name.
    #[must_use]
    pub fn declaration_by_name(&self, name: &str) -> Option<&Declaration> {
        self.declarations()
            .iter()
            .find(|declaration| declaration.name() == name)
    }

    /// Maps a declaration element to its complete original-byte source location.
    #[must_use]
    pub fn source_location(&self, element_id: ElementId) -> Option<SourceLocation> {
        self.artifacts.source_map().entry(element_id).map(|entry| {
            SourceLocation::new(
                self.artifacts.source_map().source_digest(),
                entry.declaration_span(),
            )
        })
    }

    /// Returns the immutable validated compiler artifacts for advanced readers.
    #[must_use]
    pub const fn artifacts(&self) -> &Arc<CompilationArtifacts> {
        &self.artifacts
    }
}

/// A fail-closed in-process reader validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderError {
    /// Two declarations reused one graph-local element identifier.
    DuplicateElementId,
    /// Two declarations reused one source name.
    DuplicateDeclarationName,
    /// A declaration had no matching source-map entry.
    MissingSourceMapEntry,
    /// A declaration had no matching value-provenance record.
    MissingProvenanceRecord,
    /// A declaration value did not satisfy its resolved type and nullability.
    TypeValueMismatch,
    /// A nominal record schema violated ownership, ordering, or recursion rules.
    InvalidRecordSchema,
    /// Field provenance was dangling, duplicated, or used an invalid origin.
    InvalidFieldProvenance,
}

/// Validates relationships among logical declarations and companion artifacts.
fn validate_artifacts(artifacts: &CompilationArtifacts) -> Result<(), ReaderError> {
    let mut element_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let records = artifacts
        .logical_document()
        .record_types()
        .iter()
        .map(|record| (record.name(), record))
        .collect::<BTreeMap<_, _>>();
    validate_record_schemas(artifacts, &records)?;
    for record in artifacts.logical_document().record_types() {
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
        if !validate_value(declaration.resolved_type(), declaration.value(), &records) {
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
    Ok(())
}

/// Validates field provenance ownership, paths, uniqueness, and origin kinds.
fn validate_field_provenance(
    artifacts: &CompilationArtifacts,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> Result<(), ReaderError> {
    let declarations = artifacts
        .logical_document()
        .declarations()
        .iter()
        .map(|declaration| (declaration.element_id(), declaration))
        .collect::<BTreeMap<_, _>>();
    let mut observed = BTreeMap::new();
    for provenance in artifacts.field_provenance() {
        let Some(declaration) = declarations.get(&provenance.element_id()) else {
            return Err(ReaderError::InvalidFieldProvenance);
        };
        if provenance.field_path().is_empty()
            || !matches!(
                provenance.origin(),
                ValueOrigin::ExplicitRecordField | ValueOrigin::UserRecordDefault
            )
            || observed
                .insert(
                    (provenance.element_id(), provenance.field_path().to_vec()),
                    provenance.origin(),
                )
                .is_some()
            || !field_path_exists(
                declaration.resolved_type(),
                declaration.value(),
                provenance.field_path(),
                records,
            )
        {
            return Err(ReaderError::InvalidFieldProvenance);
        }
    }
    for declaration in declarations.values() {
        if !field_provenance_is_complete(
            declaration.element_id(),
            declaration.resolved_type(),
            declaration.value(),
            &mut Vec::new(),
            records,
            &observed,
        ) {
            return Err(ReaderError::InvalidFieldProvenance);
        }
    }
    Ok(())
}

/// Returns whether every final record field has exact explicit/default evidence.
fn field_provenance_is_complete(
    element_id: ElementId,
    expected: &ResolvedType,
    value: &LogicalValue,
    path: &mut Vec<String>,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
    observed: &BTreeMap<(ElementId, Vec<String>), ValueOrigin>,
) -> bool {
    let expected = match expected {
        ResolvedType::Nullable(inner) => inner.as_ref(),
        expected => expected,
    };
    if let (ResolvedType::List(inner), LogicalValue::List(items)) = (expected, value) {
        for (index, item) in items.iter().enumerate() {
            path.push(index.to_string());
            if !field_provenance_is_complete(element_id, inner, item, path, records, observed) {
                return false;
            }
            path.pop();
        }
        return true;
    }
    let (ResolvedType::Record(identity), LogicalValue::Record(value)) = (expected, value) else {
        return true;
    };
    let Some(schema) = records.get(identity.name()) else {
        return false;
    };
    for (schema_field, value_field) in schema.fields().iter().zip(value.fields()) {
        path.push(schema_field.name().to_owned());
        let key = (element_id, path.clone());
        let Some(origin) = observed.get(&key) else {
            return false;
        };
        if *origin == ValueOrigin::ExplicitRecordField
            && !field_provenance_is_complete(
                element_id,
                schema_field.resolved_type(),
                value_field.value(),
                path,
                records,
                observed,
            )
        {
            return false;
        }
        path.pop();
    }
    true
}

/// Returns whether a path identifies a final field in a typed record value.
fn field_path_exists(
    expected: &ResolvedType,
    value: &LogicalValue,
    path: &[String],
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> bool {
    let expected = match expected {
        ResolvedType::Nullable(inner) => inner.as_ref(),
        expected => expected,
    };
    let Some((head, tail)) = path.split_first() else {
        return false;
    };
    if let (ResolvedType::List(inner), LogicalValue::List(items)) = (expected, value) {
        let Ok(index) = head.parse::<usize>() else {
            return false;
        };
        let Some(item) = items.get(index) else {
            return false;
        };
        return field_path_exists(inner, item, tail, records);
    }
    let (ResolvedType::Record(identity), LogicalValue::Record(value)) = (expected, value) else {
        return false;
    };
    let Some(schema) = records.get(identity.name()) else {
        return false;
    };
    let Some(index) = schema
        .fields()
        .iter()
        .position(|field| field.name() == head)
    else {
        return false;
    };
    if tail.is_empty() {
        return true;
    }
    field_path_exists(
        schema.fields()[index].resolved_type(),
        value.fields()[index].value(),
        tail,
        records,
    )
}

/// Validates one recursively typed value against public record schemas.
fn validate_value(
    expected: &ResolvedType,
    value: &LogicalValue,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> bool {
    match (expected, value) {
        (ResolvedType::Nullable(_), LogicalValue::Null) => true,
        (ResolvedType::Nullable(inner), value) => validate_value(inner, value, records),
        (ResolvedType::Record(identity), LogicalValue::Record(value)) => {
            if identity != value.nominal_type() {
                return false;
            }
            let Some(schema) = records.get(identity.name()) else {
                return false;
            };
            if identity != schema.nominal_identity() {
                return false;
            }
            schema.fields().len() == value.fields().len()
                && schema
                    .fields()
                    .iter()
                    .zip(value.fields())
                    .all(|(schema_field, value_field)| {
                        schema_field.name() == value_field.name()
                            && validate_value(
                                schema_field.resolved_type(),
                                value_field.value(),
                                records,
                            )
                    })
        }
        (ResolvedType::List(inner), LogicalValue::List(items)) => items
            .iter()
            .all(|item| validate_value(inner, item, records)),
        _ => expected.accepts_value(value),
    }
}

/// Validates nominal ownership, canonical fields, targets, and acyclic embedding.
fn validate_record_schemas(
    artifacts: &CompilationArtifacts,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> Result<(), ReaderError> {
    for record in records.values() {
        if record.nominal_identity().module() != artifacts.logical_document().module() {
            return Err(ReaderError::InvalidRecordSchema);
        }
        let mut previous = None;
        for field in record.fields() {
            if previous.is_some_and(|name| name >= field.name())
                || !resolved_record_targets_exist(field.resolved_type(), records)
                || field
                    .default_value()
                    .is_some_and(|value| !validate_value(field.resolved_type(), value, records))
            {
                return Err(ReaderError::InvalidRecordSchema);
            }
            previous = Some(field.name());
        }
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for name in records.keys() {
        if record_schema_has_cycle(name, records, &mut visiting, &mut visited) {
            return Err(ReaderError::InvalidRecordSchema);
        }
    }
    Ok(())
}

/// Returns whether every nominal target in one resolved type has an exact schema.
fn resolved_record_targets_exist(
    resolved_type: &ResolvedType,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> bool {
    match resolved_type {
        ResolvedType::Record(identity) => records
            .get(identity.name())
            .is_some_and(|record| record.nominal_identity() == identity),
        ResolvedType::Nullable(inner) | ResolvedType::List(inner) => {
            resolved_record_targets_exist(inner, records)
        }
        ResolvedType::Num | ResolvedType::String | ResolvedType::Bool => true,
    }
}

/// Detects an embedded cycle from one public nominal record schema.
fn record_schema_has_cycle(
    name: &str,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(name) {
        return false;
    }
    visiting.insert(name.to_owned());
    for field in records[name].fields() {
        let Some(target) = resolved_record_target(field.resolved_type()) else {
            continue;
        };
        if visiting.contains(target) || record_schema_has_cycle(target, records, visiting, visited)
        {
            return true;
        }
    }
    visiting.remove(name);
    visited.insert(name.to_owned());
    false
}

/// Returns the nominal target embedded by an active public resolved type.
fn resolved_record_target(resolved_type: &ResolvedType) -> Option<&str> {
    match resolved_type {
        ResolvedType::Record(identity) => Some(identity.name()),
        ResolvedType::Nullable(inner) | ResolvedType::List(inner) => resolved_record_target(inner),
        ResolvedType::Num | ResolvedType::String | ResolvedType::Bool => None,
    }
}
