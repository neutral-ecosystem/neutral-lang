// SPDX-License-Identifier: Apache-2.0

//! Check that provenance paths and edges describe the actual logical values.
//!
//! Reference evidence must identify the typed target; reuse evidence must point
//! to a compatible source value; field evidence must cover materialized fields.
//! Path traversal and completeness checks reject missing, extra or contradictory
//! claims instead of inventing source history on behalf of a producer.

use super::{
    BTreeMap, BTreeSet, CompilationArtifacts, ElementId, LogicalValue, ReaderError,
    RecordTypeDefinition, ResolvedType, ValueOrigin, VocabularyContract,
};

/// Validates every typed identity edge and requires exact provenance coverage.
pub(super) fn validate_reference_provenance(
    artifacts: &CompilationArtifacts,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> Result<(), ReaderError> {
    let declarations = artifacts
        .logical_document()
        .declarations()
        .iter()
        .map(|declaration| (declaration.element_id(), declaration))
        .collect::<BTreeMap<_, _>>();
    let root_reuses = artifacts
        .reuse_provenance()
        .iter()
        .filter(|record| record.value_path().is_empty())
        .map(neutral_ir::ReuseProvenanceRecord::element_id)
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeMap::new();
    for provenance in artifacts.reference_provenance() {
        let Some(owner) = declarations.get(&provenance.element_id()) else {
            return Err(ReaderError::InvalidReferenceEdge);
        };
        let Some(target) = declarations.get(&provenance.target_element_id()) else {
            return Err(ReaderError::InvalidReferenceEdge);
        };
        let Some((expected, value)) = value_at_path(
            owner.resolved_type(),
            owner.value(),
            provenance.value_path(),
            records,
            artifacts.logical_document().vocabulary(),
        ) else {
            return Err(ReaderError::InvalidReferenceEdge);
        };
        let expected = match expected {
            ResolvedType::Nullable(inner) => inner.as_ref(),
            expected => expected,
        };
        let (ResolvedType::Ref(expected_target), LogicalValue::Reference(reference)) =
            (expected, value)
        else {
            return Err(ReaderError::InvalidReferenceEdge);
        };
        if reference.target_element_id() != provenance.target_element_id()
            || reference.target_symbol_identity() != target.symbol_identity()
            || reference.target_type() != target.resolved_type()
            || expected_target.as_ref() != target.resolved_type()
            || observed
                .insert(
                    (provenance.element_id(), provenance.value_path().to_vec()),
                    provenance.target_element_id(),
                )
                .is_some()
        {
            return Err(ReaderError::InvalidReferenceEdge);
        }
    }
    for declaration in declarations.values() {
        if !reference_provenance_is_complete(
            declaration.element_id(),
            declaration.value(),
            &mut Vec::new(),
            &observed,
        ) {
            return Err(ReaderError::InvalidReferenceEdge);
        }
    }
    for provenance in artifacts.provenance() {
        let has_root_reference = observed.contains_key(&(provenance.element_id(), Vec::new()));
        let expected_direct = has_root_reference && !root_reuses.contains(&provenance.element_id());
        if expected_direct
            != matches!(
                (provenance.origin(), provenance.normalization()),
                (
                    ValueOrigin::IdentityReference,
                    neutral_ir::Normalization::IdentityReferenceResolution
                )
            )
        {
            return Err(ReaderError::InvalidReferenceEdge);
        }
    }
    Ok(())
}

/// Returns whether every final logical identity edge has one provenance entry.
fn reference_provenance_is_complete(
    element_id: ElementId,
    value: &LogicalValue,
    path: &mut Vec<String>,
    observed: &BTreeMap<(ElementId, Vec<String>), ElementId>,
) -> bool {
    match value {
        LogicalValue::Reference(reference) => observed
            .get(&(element_id, path.clone()))
            .is_some_and(|target| *target == reference.target_element_id()),
        LogicalValue::Record(record) => record.fields().iter().all(|field| {
            path.push(field.name().to_owned());
            let complete =
                reference_provenance_is_complete(element_id, field.value(), path, observed);
            path.pop();
            complete
        }),
        LogicalValue::VocabularyRecord(record) => record.fields().iter().all(|field| {
            path.push(field.name().to_owned());
            let complete =
                reference_provenance_is_complete(element_id, field.value(), path, observed);
            path.pop();
            complete
        }),
        LogicalValue::List(items) => items.iter().enumerate().all(|(index, item)| {
            path.push(index.to_string());
            let complete = reference_provenance_is_complete(element_id, item, path, observed);
            path.pop();
            complete
        }),
        LogicalValue::Number(_)
        | LogicalValue::String(_)
        | LogicalValue::Boolean(_)
        | LogicalValue::Null => true,
    }
}

/// Validates immutable-value reuse ownership, paths, targets, types, and values.
pub(super) fn validate_reuse_provenance(
    artifacts: &CompilationArtifacts,
    records: &BTreeMap<&str, &RecordTypeDefinition>,
) -> Result<(), ReaderError> {
    let declarations = artifacts
        .logical_document()
        .declarations()
        .iter()
        .map(|declaration| (declaration.element_id(), declaration))
        .collect::<BTreeMap<_, _>>();
    let mut observed = BTreeSet::new();
    for provenance in artifacts.reuse_provenance() {
        let Some(owner) = declarations.get(&provenance.element_id()) else {
            return Err(ReaderError::InvalidReuseProvenance);
        };
        let Some(source) = declarations.get(&provenance.source_element_id()) else {
            return Err(ReaderError::InvalidReuseProvenance);
        };
        let Some((expected, value)) = value_at_path(
            owner.resolved_type(),
            owner.value(),
            provenance.value_path(),
            records,
            artifacts.logical_document().vocabulary(),
        ) else {
            return Err(ReaderError::InvalidReuseProvenance);
        };
        if !observed.insert((provenance.element_id(), provenance.value_path().to_vec()))
            || !reuse_type_is_compatible(expected, source.resolved_type())
            || value != source.value()
        {
            return Err(ReaderError::InvalidReuseProvenance);
        }
    }
    for provenance in artifacts.provenance() {
        let root_reuse = observed.contains(&(provenance.element_id(), Vec::new()));
        if matches!(
            (provenance.origin(), provenance.normalization(), root_reuse),
            (
                ValueOrigin::OrdinaryReuse,
                neutral_ir::Normalization::ImmutableValueReuse,
                true
            )
        ) {
            continue;
        }
        if provenance.origin() == ValueOrigin::OrdinaryReuse
            || provenance.normalization() == neutral_ir::Normalization::ImmutableValueReuse
            || root_reuse
        {
            return Err(ReaderError::InvalidReuseProvenance);
        }
    }
    Ok(())
}

/// Resolves one canonical value path to its contextual type and final value.
fn value_at_path<'a>(
    expected: &'a ResolvedType,
    value: &'a LogicalValue,
    path: &[String],
    records: &BTreeMap<&str, &'a RecordTypeDefinition>,
    vocabulary: Option<&'a VocabularyContract>,
) -> Option<(&'a ResolvedType, &'a LogicalValue)> {
    if path.is_empty() {
        return Some((expected, value));
    }
    let expected = match expected {
        ResolvedType::Nullable(inner) => inner.as_ref(),
        expected => expected,
    };
    let (head, tail) = path.split_first()?;
    if let (ResolvedType::List(inner), LogicalValue::List(items)) = (expected, value) {
        let index = head.parse::<usize>().ok()?;
        return value_at_path(inner, items.get(index)?, tail, records, vocabulary);
    }
    if let (
        ResolvedType::VocabularyRecord(identity),
        LogicalValue::VocabularyRecord(record_value),
    ) = (expected, value)
    {
        let schema = vocabulary?.type_by_name(identity.name())?;
        let index = schema
            .fields()
            .iter()
            .position(|field| field.name() == head)?;
        return value_at_path(
            schema.fields()[index].resolved_type(),
            record_value.fields()[index].value(),
            tail,
            records,
            vocabulary,
        );
    }
    let (ResolvedType::Record(identity), LogicalValue::Record(record_value)) = (expected, value)
    else {
        return None;
    };
    let schema = records.get(identity.name())?;
    let index = schema
        .fields()
        .iter()
        .position(|field| field.name() == head)?;
    value_at_path(
        schema.fields()[index].resolved_type(),
        record_value.fields()[index].value(),
        tail,
        records,
        vocabulary,
    )
}

/// Returns whether a source binding type may satisfy one reuse occurrence.
fn reuse_type_is_compatible(expected: &ResolvedType, source: &ResolvedType) -> bool {
    expected == source
        || matches!(expected, ResolvedType::Nullable(inner) if inner.as_ref() == source)
}

/// Validates field provenance ownership, paths, uniqueness, and origin kinds.
pub(super) fn validate_field_provenance(
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
                ValueOrigin::ExplicitRecordField
                    | ValueOrigin::UserRecordDefault
                    | ValueOrigin::VocabularyDefault
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
                artifacts.logical_document().vocabulary(),
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
            artifacts.logical_document().vocabulary(),
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
    vocabulary: Option<&VocabularyContract>,
    observed: &BTreeMap<(ElementId, Vec<String>), ValueOrigin>,
) -> bool {
    let expected = match expected {
        ResolvedType::Nullable(inner) => inner.as_ref(),
        expected => expected,
    };
    if let (ResolvedType::List(inner), LogicalValue::List(items)) = (expected, value) {
        for (index, item) in items.iter().enumerate() {
            path.push(index.to_string());
            if !field_provenance_is_complete(
                element_id, inner, item, path, records, vocabulary, observed,
            ) {
                return false;
            }
            path.pop();
        }
        return true;
    }
    if let (ResolvedType::VocabularyRecord(identity), LogicalValue::VocabularyRecord(value)) =
        (expected, value)
    {
        let Some(schema) = vocabulary.and_then(|contract| contract.type_by_name(identity.name()))
        else {
            return false;
        };
        for (schema_field, value_field) in schema.fields().iter().zip(value.fields()) {
            path.push(schema_field.name().to_owned());
            let Some(origin) = observed.get(&(element_id, path.clone())) else {
                return false;
            };
            if *origin == ValueOrigin::ExplicitRecordField
                && !field_provenance_is_complete(
                    element_id,
                    schema_field.resolved_type(),
                    value_field.value(),
                    path,
                    records,
                    vocabulary,
                    observed,
                )
            {
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
                vocabulary,
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
    vocabulary: Option<&VocabularyContract>,
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
        return field_path_exists(inner, item, tail, records, vocabulary);
    }
    if let (ResolvedType::VocabularyRecord(identity), LogicalValue::VocabularyRecord(value)) =
        (expected, value)
    {
        let Some(schema) = vocabulary.and_then(|contract| contract.type_by_name(identity.name()))
        else {
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
        return field_path_exists(
            schema.fields()[index].resolved_type(),
            value.fields()[index].value(),
            tail,
            records,
            vocabulary,
        );
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
        vocabulary,
    )
}
