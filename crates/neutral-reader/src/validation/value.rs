// SPDX-License-Identifier: Apache-2.0

//! Validate value/type compatibility and document-local record schemas.
//!
//! Check nominal targets, field types, closed defaults and embedding cycles
//! before accepting record instances. Recursive value checks use the retained
//! schema rather than source syntax. Vocabulary contracts and provenance edges
//! have separate validators so type acceptance does not imply evidence validity.

use super::{
    BTreeMap, BTreeSet, CompilationArtifacts, LogicalValue, ReaderError, RecordTypeDefinition,
    ResolvedType,
};

/// Validates one recursively typed value against public record schemas.
pub(super) fn validate_value(
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
pub(super) fn validate_record_schemas(
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
                || field.default_value().is_some_and(|value| {
                    !is_closed_default(value)
                        || !validate_value(field.resolved_type(), value, records)
                })
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

/// Returns whether one schema default is recursively closed and reference-free.
pub(super) fn is_closed_default(value: &LogicalValue) -> bool {
    match value {
        LogicalValue::Record(record) => record
            .fields()
            .iter()
            .all(|field| is_closed_default(field.value())),
        LogicalValue::VocabularyRecord(record) => record
            .fields()
            .iter()
            .all(|field| is_closed_default(field.value())),
        LogicalValue::List(items) => items.iter().all(is_closed_default),
        LogicalValue::Reference(_) => false,
        LogicalValue::Number(_)
        | LogicalValue::String(_)
        | LogicalValue::Boolean(_)
        | LogicalValue::Null => true,
    }
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
        ResolvedType::Nullable(inner) | ResolvedType::List(inner) | ResolvedType::Ref(inner) => {
            resolved_record_targets_exist(inner, records)
        }
        ResolvedType::VocabularyRecord(_)
        | ResolvedType::Num
        | ResolvedType::String
        | ResolvedType::Bool => true,
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
        ResolvedType::VocabularyRecord(_) => None,
        ResolvedType::Nullable(inner) | ResolvedType::List(inner) => resolved_record_target(inner),
        ResolvedType::Ref(_) | ResolvedType::Num | ResolvedType::String | ResolvedType::Bool => {
            None
        }
    }
}
