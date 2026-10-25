// SPDX-License-Identifier: Apache-2.0

//! Bounded captured-bundle decoding, default validation and type-graph checks.

use crate::{
    JsonValue, LogicalVocabulary, ValidatedVocabularyBundle, VocabularyError, VocabularyField,
    VocabularyLimits, VocabularyLock, VocabularyRecordValueField, VocabularyType,
    VocabularyTypeDefinition, VocabularyValue, json, schema, validation,
};
use neutral_core::VocabularyContentDigest;
use neutral_ir::ExactNumber;
use std::collections::{BTreeMap, BTreeSet};
use validation::{array, boolean, exact_members, member, object as json_object, string};

/// Validates exact captured bytes into an immutable closed logical contract.
///
/// # Errors
///
/// Returns a bounded failure before exposing any partially validated contract.
pub fn validate_captured_bundle(
    bytes: &[u8],
    lock: &VocabularyLock,
    limits: VocabularyLimits,
) -> Result<ValidatedVocabularyBundle, VocabularyError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bundle_bytes() {
        return Err(VocabularyError::BundleByteLimitExceeded);
    }
    let content_digest = VocabularyContentDigest::from_bytes(bytes);
    if !content_digest.securely_matches(lock.content_digest) {
        return Err(VocabularyError::DigestMismatch);
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(VocabularyError::BomForbidden);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| VocabularyError::InvalidUtf8)?;
    let root = json::parse(text, limits)?;
    decode_bundle(&root, lock, limits, content_digest)
}

/// Decodes and validates the complete closed vocabulary envelope.
fn decode_bundle(
    root: &JsonValue,
    lock: &VocabularyLock,
    limits: VocabularyLimits,
    content_digest: VocabularyContentDigest,
) -> Result<ValidatedVocabularyBundle, VocabularyError> {
    let object = json_object(root).map_err(|_| VocabularyError::InvalidEnvelope)?;
    exact_members(
        object,
        &[
            "format",
            "encoding_version",
            "schema_version",
            "identity",
            "version",
            "required_features",
            "types",
        ],
    )?;
    let format = string(object, "format")?;
    let encoding_version = string(object, "encoding_version")?;
    let schema_version = string(object, "schema_version")?;
    let identity = string(object, "identity")?;
    let version = string(object, "version")?;
    if format != schema::BUNDLE_FORMAT {
        return Err(VocabularyError::UnsupportedFormat);
    }
    if encoding_version != schema::ENCODING_VERSION || encoding_version != lock.encoding_version {
        return Err(VocabularyError::UnsupportedEncodingVersion);
    }
    if schema_version != schema::SCHEMA_VERSION || schema_version != lock.schema_version {
        return Err(VocabularyError::UnsupportedSchemaVersion);
    }
    if !schema::is_upper_name(identity) || schema::is_protected_name(identity) {
        return Err(VocabularyError::InvalidIdentity);
    }
    if !schema::is_exact_release_version(version) {
        return Err(VocabularyError::InvalidVersion);
    }
    if identity != lock.identity || version != lock.version {
        return Err(VocabularyError::LockMismatch);
    }
    let required_features = decode_features(member(object, "required_features")?, lock, limits)?;
    let pending = decode_pending_types(member(object, "types")?, limits)?;
    validate_type_recursion(&pending, limits)?;
    let mut budget = ValidationBudget::new(limits);
    let mut types = pending
        .iter()
        .map(|definition| finalize_type(definition, &pending, &mut budget))
        .collect::<Result<Vec<_>, _>>()?;
    types.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(ValidatedVocabularyBundle {
        encoding_version: encoding_version.to_owned(),
        content_digest,
        logical: LogicalVocabulary {
            identity: identity.to_owned(),
            version: version.to_owned(),
            schema_version: schema_version.to_owned(),
            required_features,
            types,
        },
    })
}

/// Decodes and normalizes the exact required structural feature set.
fn decode_features(
    value: &JsonValue,
    lock: &VocabularyLock,
    limits: VocabularyLimits,
) -> Result<Vec<String>, VocabularyError> {
    let values = array(value)?;
    if u64::try_from(values.len()).unwrap_or(u64::MAX) > limits.features() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut features = values
        .iter()
        .map(|value| match value {
            JsonValue::String(value) if schema::is_feature_id(value) => Ok(value.clone()),
            JsonValue::String(_) => Err(VocabularyError::InvalidFeatureId),
            _ => Err(VocabularyError::InvalidMemberType),
        })
        .collect::<Result<Vec<_>, _>>()?;
    features.sort();
    if features.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(VocabularyError::DuplicateFeature);
    }
    if features
        .iter()
        .any(|feature| !lock.required_features.contains(feature))
    {
        return Err(VocabularyError::UnknownRequiredFeature);
    }
    if features != lock.required_features {
        return Err(VocabularyError::LockMismatch);
    }
    Ok(features)
}

/// One field awaiting contextual default validation.
struct PendingField<'a> {
    /// Validated field name.
    name: String,
    /// Resolved closed field type.
    field_type: VocabularyType,
    /// Untrusted default node retained by reference.
    default: &'a JsonValue,
}
/// One nominal type awaiting graph and default validation.
struct PendingType<'a> {
    /// Validated nominal type name.
    name: String,
    /// Fields awaiting contextual default validation.
    fields: Vec<PendingField<'a>>,
}

/// Collects all type names before resolving field target references.
fn decode_pending_types(
    value: &JsonValue,
    limits: VocabularyLimits,
) -> Result<Vec<PendingType<'_>>, VocabularyError> {
    let values = array(value)?;
    if u64::try_from(values.len()).unwrap_or(u64::MAX) > limits.types() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut names = BTreeSet::new();
    for value in values {
        let object = json_object(value)?;
        exact_members(object, &["name", "fields"])?;
        let name = string(object, "name")?;
        if !schema::is_upper_name(name) || schema::is_protected_name(name) {
            return Err(VocabularyError::InvalidTypeName);
        }
        if !names.insert(name) {
            return Err(VocabularyError::DuplicateType);
        }
    }
    values
        .iter()
        .map(|value| decode_pending_type(value, &names, limits))
        .collect()
}

/// Decodes one nominal type after complete type-name collection.
fn decode_pending_type<'a>(
    value: &'a JsonValue,
    type_names: &BTreeSet<&str>,
    limits: VocabularyLimits,
) -> Result<PendingType<'a>, VocabularyError> {
    let object = json_object(value)?;
    let name = string(object, "name")?.to_owned();
    let fields = array(member(object, "fields")?)?;
    if u64::try_from(fields.len()).unwrap_or(u64::MAX) > limits.fields() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut names = BTreeSet::new();
    let mut decoded = Vec::new();
    for field in fields {
        let field = json_object(field)?;
        exact_members(field, &["name", "type", "default"])?;
        let field_name = string(field, "name")?;
        if !schema::is_snake_name(field_name) || schema::is_protected_name(field_name) {
            return Err(VocabularyError::InvalidFieldName);
        }
        if !names.insert(field_name) {
            return Err(VocabularyError::DuplicateField);
        }
        decoded.push(PendingField {
            name: field_name.to_owned(),
            field_type: decode_type(member(field, "type")?, type_names, 1, limits)?,
            default: member(field, "default")?,
        });
    }
    decoded.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(PendingType {
        name,
        fields: decoded,
    })
}

/// Decodes one closed tagged type expression with exact key ownership.
fn decode_type(
    value: &JsonValue,
    type_names: &BTreeSet<&str>,
    depth: u64,
    limits: VocabularyLimits,
) -> Result<VocabularyType, VocabularyError> {
    if depth > limits.nesting_depth() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let object = json_object(value)?;
    let kind = string(object, "kind")?;
    match kind {
        "num" => {
            exact_members(object, &["kind"])?;
            Ok(VocabularyType::Num)
        }
        "string" => {
            exact_members(object, &["kind"])?;
            Ok(VocabularyType::String)
        }
        "bool" => {
            exact_members(object, &["kind"])?;
            Ok(VocabularyType::Bool)
        }
        "nullable" => {
            exact_members(object, &["kind", "inner"])?;
            decode_type(member(object, "inner")?, type_names, depth + 1, limits)
                .map(|inner| VocabularyType::Nullable(Box::new(inner)))
        }
        "list" => {
            exact_members(object, &["kind", "element"])?;
            decode_type(member(object, "element")?, type_names, depth + 1, limits)
                .map(|element| VocabularyType::List(Box::new(element)))
        }
        "ref" | "record" => {
            exact_members(object, &["kind", "target"])?;
            let target = string(object, "target")?;
            if !type_names.contains(target) {
                return Err(VocabularyError::UnknownTypeTarget);
            }
            if kind == "ref" {
                Ok(VocabularyType::Ref(target.to_owned()))
            } else {
                Ok(VocabularyType::Record(target.to_owned()))
            }
        }
        executable if schema::is_executable_member(executable) => {
            Err(VocabularyError::ExecutableShapeForbidden)
        }
        _ => Err(VocabularyError::InvalidType),
    }
}

/// Rejects embedded nominal cycles while treating identity references as leaves.
fn validate_type_recursion(
    definitions: &[PendingType<'_>],
    limits: VocabularyLimits,
) -> Result<(), VocabularyError> {
    let graph = definitions
        .iter()
        .map(|definition| {
            let mut targets = BTreeSet::new();
            for field in &definition.fields {
                collect_embedded_targets(&field.field_type, &mut targets);
            }
            (definition.name.as_str(), targets)
        })
        .collect::<BTreeMap<_, _>>();
    for root in graph.keys() {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut stack = vec![(*root, false)];
        let mut traversed = 0_u64;
        while let Some((name, leaving)) = stack.pop() {
            traversed = traversed.saturating_add(1);
            if traversed > limits.total_nodes() {
                return Err(VocabularyError::JsonLimitExceeded);
            }
            if leaving {
                visiting.remove(name);
                visited.insert(name);
                continue;
            }
            if visited.contains(name) {
                continue;
            }
            if !visiting.insert(name) {
                return Err(VocabularyError::InvalidTypeRecursion);
            }
            stack.push((name, true));
            for target in graph.get(name).into_iter().flatten() {
                stack.push((*target, false));
            }
        }
    }
    Ok(())
}

/// Collects record-embedding targets while stopping at identity references.
fn collect_embedded_targets<'a>(value: &'a VocabularyType, targets: &mut BTreeSet<&'a str>) {
    match value {
        VocabularyType::Nullable(inner) | VocabularyType::List(inner) => {
            collect_embedded_targets(inner, targets);
        }
        VocabularyType::Record(target) => {
            targets.insert(target);
        }
        VocabularyType::Num
        | VocabularyType::String
        | VocabularyType::Bool
        | VocabularyType::Ref(_) => {}
    }
}

/// Bounded semantic traversal counter shared by nested defaults.
struct ValidationBudget {
    /// Active resource limits.
    limits: VocabularyLimits,
    /// Total visited semantic nodes.
    nodes: u64,
}
impl ValidationBudget {
    /// Creates an empty semantic budget.
    fn new(limits: VocabularyLimits) -> Self {
        Self { limits, nodes: 0 }
    }
    /// Consumes one semantic node under the total traversal limit.
    fn consume(&mut self) -> Result<(), VocabularyError> {
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > self.limits.total_nodes() {
            Err(VocabularyError::JsonLimitExceeded)
        } else {
            Ok(())
        }
    }
}

/// Finalizes one type by contextually validating every field default.
fn finalize_type(
    definition: &PendingType<'_>,
    definitions: &[PendingType<'_>],
    budget: &mut ValidationBudget,
) -> Result<VocabularyTypeDefinition, VocabularyError> {
    let fields = definition
        .fields
        .iter()
        .map(|field| {
            let default_value = if matches!(field.default, JsonValue::Null) {
                None
            } else {
                Some(decode_default(
                    field.default,
                    &field.field_type,
                    definitions,
                    1,
                    budget,
                )?)
            };
            Ok(VocabularyField {
                name: field.name.clone(),
                field_type: field.field_type.clone(),
                default_value,
            })
        })
        .collect::<Result<Vec<_>, VocabularyError>>()?;
    Ok(VocabularyTypeDefinition {
        name: definition.name.clone(),
        fields,
    })
}

/// Decodes one closed tagged default against its exact contextual type.
fn decode_default(
    value: &JsonValue,
    expected: &VocabularyType,
    definitions: &[PendingType<'_>],
    depth: u64,
    budget: &mut ValidationBudget,
) -> Result<VocabularyValue, VocabularyError> {
    budget.consume()?;
    if depth > budget.limits.nesting_depth() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let object = json_object(value).map_err(|_| VocabularyError::InvalidDefault)?;
    let kind = string(object, "kind").map_err(|_| VocabularyError::InvalidDefault)?;
    if kind == "null" {
        exact_members(object, &["kind"]).map_err(|_| VocabularyError::InvalidDefault)?;
        return matches!(expected, VocabularyType::Nullable(_))
            .then_some(VocabularyValue::Null)
            .ok_or(VocabularyError::InvalidDefault);
    }
    let expected = match expected {
        VocabularyType::Nullable(inner) => inner.as_ref(),
        expected => expected,
    };
    match (kind, expected) {
        ("num", VocabularyType::Num) => {
            exact_members(object, &["kind", "value"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            let source = string(object, "value").map_err(|_| VocabularyError::InvalidDefault)?;
            ExactNumber::from_source(
                source,
                budget.limits.numeric_digits,
                budget.limits.numeric_scale,
            )
            .map(VocabularyValue::Number)
            .map_err(|_| VocabularyError::InvalidDefault)
        }
        ("string", VocabularyType::String) => {
            exact_members(object, &["kind", "value"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            string(object, "value")
                .map(|value| VocabularyValue::String(value.to_owned()))
                .map_err(|_| VocabularyError::InvalidDefault)
        }
        ("bool", VocabularyType::Bool) => {
            exact_members(object, &["kind", "value"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            boolean(object, "value")
                .map(VocabularyValue::Boolean)
                .map_err(|_| VocabularyError::InvalidDefault)
        }
        ("list", VocabularyType::List(element)) => {
            exact_members(object, &["kind", "items"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            let items =
                array(member(object, "items")?).map_err(|_| VocabularyError::InvalidDefault)?;
            if u64::try_from(items.len()).unwrap_or(u64::MAX) > budget.limits.array_items() {
                return Err(VocabularyError::JsonLimitExceeded);
            }
            items
                .iter()
                .map(|item| decode_default(item, element, definitions, depth + 1, budget))
                .collect::<Result<Vec<_>, _>>()
                .map(VocabularyValue::List)
        }
        ("record", VocabularyType::Record(target)) => {
            decode_record_default(object, target, definitions, depth, budget)
        }
        _ => Err(VocabularyError::InvalidDefault),
    }
}

/// Decodes and materializes one contextual nominal record default.
fn decode_record_default(
    object: &[(String, JsonValue)],
    target: &str,
    definitions: &[PendingType<'_>],
    depth: u64,
    budget: &mut ValidationBudget,
) -> Result<VocabularyValue, VocabularyError> {
    exact_members(object, &["kind", "fields"]).map_err(|_| VocabularyError::InvalidDefault)?;
    let fields = array(member(object, "fields")?).map_err(|_| VocabularyError::InvalidDefault)?;
    if u64::try_from(fields.len()).unwrap_or(u64::MAX) > budget.limits.fields() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let definition = definitions
        .iter()
        .find(|definition| definition.name == target)
        .ok_or(VocabularyError::UnknownTypeTarget)?;
    let mut supplied = BTreeMap::new();
    for field in fields {
        let field = json_object(field).map_err(|_| VocabularyError::InvalidDefault)?;
        exact_members(field, &["name", "value"]).map_err(|_| VocabularyError::InvalidDefault)?;
        let name = string(field, "name").map_err(|_| VocabularyError::InvalidDefault)?;
        let value = member(field, "value").map_err(|_| VocabularyError::InvalidDefault)?;
        if supplied.insert(name, value).is_some() {
            return Err(VocabularyError::InvalidDefault);
        }
    }
    if supplied
        .keys()
        .any(|name| !definition.fields.iter().any(|field| field.name == **name))
    {
        return Err(VocabularyError::InvalidDefault);
    }
    let mut final_fields = Vec::new();
    for field in &definition.fields {
        let source = match supplied.get(field.name.as_str()) {
            Some(value) => *value,
            None if !matches!(field.default, JsonValue::Null) => field.default,
            None => return Err(VocabularyError::InvalidDefault),
        };
        final_fields.push(VocabularyRecordValueField {
            name: field.name.clone(),
            value: decode_default(source, &field.field_type, definitions, depth + 1, budget)?,
        });
    }
    Ok(VocabularyValue::Record {
        type_name: target.to_owned(),
        fields: final_fields,
    })
}
