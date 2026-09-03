// SPDX-License-Identifier: Apache-2.0

//! Closed vocabulary contracts for Neutral.
//!
//! This crate validates already-captured, untrusted bundle bytes into immutable
//! logical contracts. It performs no filesystem, registry, environment,
//! network, dynamic-code, or other host acquisition.

mod json;
mod schema;

use json::JsonValue;
use neutral_core::{StructuralLimits, VocabularyContentDigest};
use neutral_ir::ExactNumber;
use std::collections::{BTreeMap, BTreeSet};

/// Frozen captured vocabulary bundle encoding version.
pub const VOCABULARY_ENCODING_VERSION: &str = schema::ENCODING_VERSION;
/// Frozen logical vocabulary schema version.
pub const VOCABULARY_SCHEMA_VERSION: &str = schema::SCHEMA_VERSION;

/// Explicit resource limits for strict vocabulary decoding and validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VocabularyLimits {
    /// Maximum exact captured bundle bytes.
    bundle_bytes: u64,
    /// Maximum decoded bytes in one JSON string.
    string_bytes: u64,
    /// Maximum members in one JSON object.
    object_members: u64,
    /// Maximum items in one JSON array.
    array_items: u64,
    /// Maximum nested JSON/type/default depth.
    nesting_depth: u64,
    /// Maximum total JSON and semantic nodes.
    total_nodes: u64,
    /// Maximum nominal vocabulary types.
    types: u64,
    /// Maximum fields in one type or record default.
    fields: u64,
    /// Maximum required structural features.
    features: u64,
    /// Maximum exact-number significant digits.
    numeric_digits: u64,
    /// Maximum exact-number absolute scale.
    numeric_scale: u64,
}

impl VocabularyLimits {
    /// Creates vocabulary limits from the shared structural policy.
    ///
    /// The captured byte bound also bounds JSON members, arrays, types, and
    /// features until a host supplies narrower values through the builder API.
    #[must_use]
    pub const fn from_structural(limits: StructuralLimits) -> Self {
        Self {
            bundle_bytes: limits.source_bytes(),
            string_bytes: limits.string_bytes(),
            object_members: limits.record_fields(),
            array_items: limits.list_items(),
            nesting_depth: limits.nesting_depth(),
            total_nodes: limits.traversal_nodes(),
            types: limits.declarations(),
            fields: limits.record_fields(),
            features: limits.list_items(),
            numeric_digits: limits.numeric_digits(),
            numeric_scale: limits.numeric_scale(),
        }
    }

    /// Overrides the exact captured bundle-byte limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_bundle_bytes(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.bundle_bytes = value;
        Ok(self)
    }

    /// Overrides the per-object member limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_object_members(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.object_members = value;
        Ok(self)
    }

    /// Overrides the per-array item limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_array_items(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.array_items = value;
        Ok(self)
    }

    /// Overrides the nominal-type limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_types(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.types = value;
        Ok(self)
    }

    /// Overrides the structural-feature limit.
    ///
    /// # Errors
    ///
    /// Returns [`VocabularyError::InvalidLimits`] for zero.
    pub const fn with_features(mut self, value: u64) -> Result<Self, VocabularyError> {
        if value == 0 {
            return Err(VocabularyError::InvalidLimits);
        }
        self.features = value;
        Ok(self)
    }

    /// Returns the exact captured bundle-byte limit.
    #[must_use]
    pub const fn bundle_bytes(self) -> u64 {
        self.bundle_bytes
    }

    /// Returns the maximum decoded JSON string bytes.
    #[must_use]
    pub const fn string_bytes(self) -> u64 {
        self.string_bytes
    }

    /// Returns the maximum object members.
    #[must_use]
    pub const fn object_members(self) -> u64 {
        self.object_members
    }

    /// Returns the maximum array items.
    #[must_use]
    pub const fn array_items(self) -> u64 {
        self.array_items
    }

    /// Returns the maximum nesting depth.
    #[must_use]
    pub const fn nesting_depth(self) -> u64 {
        self.nesting_depth
    }

    /// Returns the maximum total node count.
    #[must_use]
    pub const fn total_nodes(self) -> u64 {
        self.total_nodes
    }

    /// Returns the maximum nominal type count.
    #[must_use]
    pub const fn types(self) -> u64 {
        self.types
    }

    /// Returns the maximum fields per type or record default.
    #[must_use]
    pub const fn fields(self) -> u64 {
        self.fields
    }

    /// Returns the maximum structural feature count.
    #[must_use]
    pub const fn features(self) -> u64 {
        self.features
    }
}

/// Exact host-supplied lock facts for one captured vocabulary bundle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyLock {
    /// Expected logical vocabulary identity.
    identity: String,
    /// Expected exact vocabulary release.
    version: String,
    /// Expected bundle encoding version.
    encoding_version: String,
    /// Expected logical schema version.
    schema_version: String,
    /// Expected exact captured-byte digest.
    content_digest: VocabularyContentDigest,
    /// Exact normalized structural feature set.
    required_features: Vec<String>,
}

impl VocabularyLock {
    /// Creates validated immutable lock facts without acquiring bundle content.
    ///
    /// # Errors
    ///
    /// Returns a bounded lock error for invalid identities, versions, feature
    /// IDs, or duplicate features.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        encoding_version: impl Into<String>,
        schema_version: impl Into<String>,
        content_digest: VocabularyContentDigest,
        mut required_features: Vec<String>,
    ) -> Result<Self, VocabularyError> {
        let identity = identity.into();
        let version = version.into();
        let encoding_version = encoding_version.into();
        let schema_version = schema_version.into();
        if !schema::is_upper_name(&identity) || schema::is_protected_name(&identity) {
            return Err(VocabularyError::InvalidIdentity);
        }
        if !schema::is_exact_release_version(&version) {
            return Err(VocabularyError::InvalidVersion);
        }
        if required_features
            .iter()
            .any(|feature| !schema::is_feature_id(feature))
        {
            return Err(VocabularyError::InvalidFeatureId);
        }
        required_features.sort();
        if required_features.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(VocabularyError::DuplicateFeature);
        }
        Ok(Self {
            identity,
            version,
            encoding_version,
            schema_version,
            content_digest,
            required_features,
        })
    }

    /// Returns the expected exact content digest.
    #[must_use]
    pub const fn content_digest(&self) -> VocabularyContentDigest {
        self.content_digest
    }

    /// Returns the exact locked logical identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the exact locked release version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the exact locked encoding version.
    #[must_use]
    pub fn encoding_version(&self) -> &str {
        &self.encoding_version
    }

    /// Returns the exact locked logical schema version.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Returns the exact normalized locked feature set.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        &self.required_features
    }
}

/// One normalized closed vocabulary type expression.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VocabularyType {
    /// Exact Neutral number.
    Num,
    /// Unicode string.
    String,
    /// Boolean.
    Bool,
    /// Nullable inner type.
    Nullable(Box<VocabularyType>),
    /// Invariant ordered list element type.
    List(Box<VocabularyType>),
    /// Identity-only reference to a nominal vocabulary type.
    Ref(String),
    /// Embedded contextual nominal vocabulary record.
    Record(String),
}

/// One normalized closed default value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VocabularyValue {
    /// Exact normalized number.
    Number(ExactNumber),
    /// Exact decoded Unicode string.
    String(String),
    /// Boolean.
    Boolean(bool),
    /// Explicit Neutral null.
    Null,
    /// Ordered homogeneous list.
    List(Vec<VocabularyValue>),
    /// Contextual nominal record with canonical fields.
    Record {
        /// Exact target nominal type name.
        type_name: String,
        /// Materialized fields in canonical name order.
        fields: Vec<VocabularyRecordValueField>,
    },
}

/// One materialized field inside a vocabulary record default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyRecordValueField {
    /// Canonical field name.
    name: String,
    /// Final closed field value.
    value: VocabularyValue,
}

impl VocabularyRecordValueField {
    /// Returns the canonical field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the final closed field value.
    #[must_use]
    pub const fn value(&self) -> &VocabularyValue {
        &self.value
    }
}

/// One immutable validated vocabulary field contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyField {
    /// Canonical field name.
    name: String,
    /// Fully resolved closed field type.
    field_type: VocabularyType,
    /// Final closed default, or no default.
    default_value: Option<VocabularyValue>,
}

impl VocabularyField {
    /// Returns the canonical field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the fully resolved field type.
    #[must_use]
    pub const fn field_type(&self) -> &VocabularyType {
        &self.field_type
    }

    /// Returns the final closed default when present.
    #[must_use]
    pub const fn default_value(&self) -> Option<&VocabularyValue> {
        self.default_value.as_ref()
    }
}

/// One immutable validated nominal vocabulary type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyTypeDefinition {
    /// Canonical nominal type name.
    name: String,
    /// Fields in canonical name order.
    fields: Vec<VocabularyField>,
}

impl VocabularyTypeDefinition {
    /// Returns the canonical nominal type name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[VocabularyField] {
        &self.fields
    }
}

/// Immutable normalized logical vocabulary payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalVocabulary {
    /// Exact logical vocabulary identity.
    identity: String,
    /// Exact release version.
    version: String,
    /// Logical schema version.
    schema_version: String,
    /// Normalized structural feature set.
    required_features: Vec<String>,
    /// Nominal types in canonical name order.
    types: Vec<VocabularyTypeDefinition>,
}

impl LogicalVocabulary {
    /// Returns the exact logical identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the exact release version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the logical schema version.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Returns structural feature IDs in canonical order.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        &self.required_features
    }

    /// Returns nominal type definitions in canonical order.
    #[must_use]
    pub fn types(&self) -> &[VocabularyTypeDefinition] {
        &self.types
    }

    /// Finds one nominal type without external lookup.
    #[must_use]
    pub fn type_by_name(&self, name: &str) -> Option<&VocabularyTypeDefinition> {
        self.types.iter().find(|definition| definition.name == name)
    }
}

/// Validated bundle retaining exact captured facts beside its logical payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedVocabularyBundle {
    /// Exact bundle encoding version.
    encoding_version: String,
    /// Exact captured-byte identity.
    content_digest: VocabularyContentDigest,
    /// Normalized logical vocabulary contract.
    logical: LogicalVocabulary,
}

impl ValidatedVocabularyBundle {
    /// Returns the exact bundle encoding version.
    #[must_use]
    pub fn encoding_version(&self) -> &str {
        &self.encoding_version
    }

    /// Returns the exact captured bundle content digest.
    #[must_use]
    pub const fn content_digest(&self) -> VocabularyContentDigest {
        self.content_digest
    }

    /// Returns the immutable normalized logical contract.
    #[must_use]
    pub const fn logical(&self) -> &LogicalVocabulary {
        &self.logical
    }

    /// Compares logical contracts while excluding exact byte formatting identity.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.logical == other.logical
    }
}

/// Bounded strict bundle validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VocabularyError {
    /// A limit builder received zero.
    InvalidLimits,
    /// Captured bytes exceeded the configured bound.
    BundleByteLimitExceeded,
    /// Exact captured bytes did not match the lock digest.
    DigestMismatch,
    /// A UTF-8 BOM appeared before JSON.
    BomForbidden,
    /// Captured bytes were not strict UTF-8.
    InvalidUtf8,
    /// JSON syntax was malformed, truncated, or extended.
    MalformedJson,
    /// A duplicate object member appeared before map collapse.
    DuplicateJsonMember,
    /// JSON allocation/depth/node limits were exceeded.
    JsonLimitExceeded,
    /// A raw JSON numeric token appeared.
    RawJsonNumberForbidden,
    /// The top-level JSON value was not an object.
    InvalidEnvelope,
    /// A closed-schema object contained an unknown member.
    UnknownMember,
    /// An object omitted a required member.
    MissingMember,
    /// A member used the wrong JSON shape.
    InvalidMemberType,
    /// A member attempted to introduce executable behavior.
    ExecutableShapeForbidden,
    /// The format discriminator was unsupported.
    UnsupportedFormat,
    /// The encoding version was unsupported or mismatched.
    UnsupportedEncodingVersion,
    /// The logical schema version was unsupported or mismatched.
    UnsupportedSchemaVersion,
    /// Bundle identity, release, or feature facts disagreed with the lock.
    LockMismatch,
    /// A logical vocabulary identity was invalid.
    InvalidIdentity,
    /// A vocabulary release identifier was invalid.
    InvalidVersion,
    /// A structural feature ID was malformed.
    InvalidFeatureId,
    /// A required feature appeared more than once.
    DuplicateFeature,
    /// A required feature was not permitted by exact lock facts.
    UnknownRequiredFeature,
    /// A nominal type appeared more than once.
    DuplicateType,
    /// A nominal type name was invalid or protected.
    InvalidTypeName,
    /// A field appeared more than once.
    DuplicateField,
    /// A field name was invalid or protected.
    InvalidFieldName,
    /// A tagged type was unknown or malformed.
    InvalidType,
    /// A nominal type target did not exist.
    UnknownTypeTarget,
    /// Embedded nominal records formed a cycle outside `ref`.
    InvalidTypeRecursion,
    /// A closed default was malformed, incompatible, incomplete, or contained `ref`.
    InvalidDefault,
}

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
    let JsonValue::Object(object) = root else {
        return Err(VocabularyError::InvalidEnvelope);
    };
    validate_members(
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
    require_member_count(object, 7)?;
    let format = required_string(object, "format")?;
    let encoding_version = required_string(object, "encoding_version")?;
    let schema_version = required_string(object, "schema_version")?;
    let identity = required_string(object, "identity")?;
    let version = required_string(object, "version")?;
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
    let required_features =
        decode_features(required_member(object, "required_features")?, lock, limits)?;
    let pending = decode_pending_types(required_member(object, "types")?, limits)?;
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
    let JsonValue::Array(values) = value else {
        return Err(VocabularyError::InvalidMemberType);
    };
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
    let JsonValue::Array(values) = value else {
        return Err(VocabularyError::InvalidMemberType);
    };
    if u64::try_from(values.len()).unwrap_or(u64::MAX) > limits.types() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut names = BTreeSet::new();
    for value in values {
        let object = expected_object(value)?;
        validate_members(object, &["name", "fields"])?;
        require_member_count(object, 2)?;
        let name = required_string(object, "name")?;
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
    let object = expected_object(value)?;
    let name = required_string(object, "name")?.to_owned();
    let JsonValue::Array(fields) = required_member(object, "fields")? else {
        return Err(VocabularyError::InvalidMemberType);
    };
    if u64::try_from(fields.len()).unwrap_or(u64::MAX) > limits.fields() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut names = BTreeSet::new();
    let mut decoded = Vec::new();
    for field in fields {
        let field = expected_object(field)?;
        validate_members(field, &["name", "type", "default"])?;
        require_member_count(field, 3)?;
        let field_name = required_string(field, "name")?;
        if !schema::is_snake_name(field_name) || schema::is_protected_name(field_name) {
            return Err(VocabularyError::InvalidFieldName);
        }
        if !names.insert(field_name) {
            return Err(VocabularyError::DuplicateField);
        }
        decoded.push(PendingField {
            name: field_name.to_owned(),
            field_type: decode_type(required_member(field, "type")?, type_names, 1, limits)?,
            default: required_member(field, "default")?,
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
    let object = expected_object(value)?;
    let kind = required_string(object, "kind")?;
    match kind {
        "num" => {
            validate_exact_members(object, &["kind"])?;
            Ok(VocabularyType::Num)
        }
        "string" => {
            validate_exact_members(object, &["kind"])?;
            Ok(VocabularyType::String)
        }
        "bool" => {
            validate_exact_members(object, &["kind"])?;
            Ok(VocabularyType::Bool)
        }
        "nullable" => {
            validate_exact_members(object, &["kind", "inner"])?;
            decode_type(
                required_member(object, "inner")?,
                type_names,
                depth + 1,
                limits,
            )
            .map(|inner| VocabularyType::Nullable(Box::new(inner)))
        }
        "list" => {
            validate_exact_members(object, &["kind", "element"])?;
            decode_type(
                required_member(object, "element")?,
                type_names,
                depth + 1,
                limits,
            )
            .map(|element| VocabularyType::List(Box::new(element)))
        }
        "ref" | "record" => {
            validate_exact_members(object, &["kind", "target"])?;
            let target = required_string(object, "target")?;
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
    let object = expected_object(value).map_err(|_| VocabularyError::InvalidDefault)?;
    let kind = required_string(object, "kind").map_err(|_| VocabularyError::InvalidDefault)?;
    if kind == "null" {
        validate_exact_members(object, &["kind"]).map_err(|_| VocabularyError::InvalidDefault)?;
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
            validate_exact_members(object, &["kind", "value"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            let source =
                required_string(object, "value").map_err(|_| VocabularyError::InvalidDefault)?;
            ExactNumber::from_source(
                source,
                budget.limits.numeric_digits,
                budget.limits.numeric_scale,
            )
            .map(VocabularyValue::Number)
            .map_err(|_| VocabularyError::InvalidDefault)
        }
        ("string", VocabularyType::String) => {
            validate_exact_members(object, &["kind", "value"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            required_string(object, "value")
                .map(|value| VocabularyValue::String(value.to_owned()))
                .map_err(|_| VocabularyError::InvalidDefault)
        }
        ("bool", VocabularyType::Bool) => {
            validate_exact_members(object, &["kind", "value"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            required_bool(object, "value")
                .map(VocabularyValue::Boolean)
                .map_err(|_| VocabularyError::InvalidDefault)
        }
        ("list", VocabularyType::List(element)) => {
            validate_exact_members(object, &["kind", "items"])
                .map_err(|_| VocabularyError::InvalidDefault)?;
            let JsonValue::Array(items) =
                required_member(object, "items").map_err(|_| VocabularyError::InvalidDefault)?
            else {
                return Err(VocabularyError::InvalidDefault);
            };
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
    validate_exact_members(object, &["kind", "fields"])
        .map_err(|_| VocabularyError::InvalidDefault)?;
    let JsonValue::Array(fields) =
        required_member(object, "fields").map_err(|_| VocabularyError::InvalidDefault)?
    else {
        return Err(VocabularyError::InvalidDefault);
    };
    if u64::try_from(fields.len()).unwrap_or(u64::MAX) > budget.limits.fields() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let definition = definitions
        .iter()
        .find(|definition| definition.name == target)
        .ok_or(VocabularyError::UnknownTypeTarget)?;
    let mut supplied = BTreeMap::new();
    for field in fields {
        let field = expected_object(field).map_err(|_| VocabularyError::InvalidDefault)?;
        validate_exact_members(field, &["name", "value"])
            .map_err(|_| VocabularyError::InvalidDefault)?;
        let name = required_string(field, "name").map_err(|_| VocabularyError::InvalidDefault)?;
        let value = required_member(field, "value").map_err(|_| VocabularyError::InvalidDefault)?;
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

/// Returns one JSON object or a closed-schema type error.
fn expected_object(value: &JsonValue) -> Result<&[(String, JsonValue)], VocabularyError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

/// Rejects unknown or executable members in one closed-schema object.
fn validate_members(
    object: &[(String, JsonValue)],
    allowed: &[&str],
) -> Result<(), VocabularyError> {
    for (name, _) in object {
        if !allowed.contains(&name.as_str()) {
            return if schema::is_executable_member(name) {
                Err(VocabularyError::ExecutableShapeForbidden)
            } else {
                Err(VocabularyError::UnknownMember)
            };
        }
    }
    Ok(())
}

/// Requires an object to contain exactly the complete allowed member set.
fn validate_exact_members(
    object: &[(String, JsonValue)],
    allowed: &[&str],
) -> Result<(), VocabularyError> {
    validate_members(object, allowed)?;
    require_member_count(object, allowed.len())
}

/// Requires one exact closed-schema object member count.
fn require_member_count(
    object: &[(String, JsonValue)],
    expected: usize,
) -> Result<(), VocabularyError> {
    if object.len() == expected {
        Ok(())
    } else {
        Err(VocabularyError::MissingMember)
    }
}

/// Returns one required object member without map allocation.
fn required_member<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a JsonValue, VocabularyError> {
    object
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value)
        .ok_or(VocabularyError::MissingMember)
}

/// Returns one required string member.
fn required_string<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a str, VocabularyError> {
    match required_member(object, name)? {
        JsonValue::String(value) => Ok(value),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

/// Returns one required Boolean member.
fn required_bool(object: &[(String, JsonValue)], name: &str) -> Result<bool, VocabularyError> {
    match required_member(object, name)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

#[cfg(test)]
/// Executable conformance tests for the strict captured vocabulary boundary.
mod tests {
    use super::{
        ValidatedVocabularyBundle, VocabularyError, VocabularyLimits, VocabularyLock,
        VocabularyType, VocabularyValue, schema, validate_captured_bundle,
    };
    use neutral_core::{StructuralLimits, VocabularyContentDigest};

    /// Comprehensive accepted exact-byte bundle.
    const COMPREHENSIVE: &[u8] = include_bytes!(
        "../../../portable/spec/v0/fixtures/vocabulary/bundles/positive/comprehensive.json"
    );
    /// Logically equal reordered and reformatted bundle.
    const REORDERED: &[u8] = include_bytes!(
        "../../../portable/spec/v0/fixtures/vocabulary/bundles/positive/reordered.json"
    );
    /// Empty valid bundle used for generated hostile byte cases.
    const EMPTY_BUNDLE: &[u8] = b"{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"0.1\",\"schema_version\":\"0.1\",\"identity\":\"Fixture\",\"version\":\"0.1.0\",\"required_features\":[],\"types\":[]}";

    /// Returns generous deterministic limits for bundle conformance tests.
    fn limits() -> VocabularyLimits {
        let structural = StructuralLimits::new(100_000, 16)
            .expect("nonzero vocabulary test limits should be valid");
        VocabularyLimits::from_structural(structural)
    }

    /// Creates exact lock facts for one fixture's current captured bytes.
    fn lock(bytes: &[u8]) -> VocabularyLock {
        lock_with_features(bytes, Vec::new())
    }

    /// Creates exact lock facts with one caller-selected structural feature set.
    fn lock_with_features(bytes: &[u8], features: Vec<String>) -> VocabularyLock {
        VocabularyLock::new(
            "Fixture",
            "0.1.0",
            schema::ENCODING_VERSION,
            schema::SCHEMA_VERSION,
            VocabularyContentDigest::from_bytes(bytes),
            features,
        )
        .expect("test lock facts should be valid")
    }

    /// Validates one fixture using its matching exact lock facts.
    fn validate(bytes: &[u8]) -> Result<ValidatedVocabularyBundle, VocabularyError> {
        validate_captured_bundle(bytes, &lock(bytes), limits())
    }

    #[test]
    /// Verifies every allowed type/default form and reference-only recursion.
    fn conformance_vocabulary_comprehensive_bundle() {
        let bundle = validate(COMPREHENSIVE).expect("comprehensive bundle should validate");
        assert_eq!(bundle.logical().identity(), "Fixture");
        assert_eq!(bundle.logical().types().len(), 3);
        let metadata = bundle
            .logical()
            .type_by_name("Metadata")
            .expect("metadata type should exist");
        assert_eq!(metadata.fields().len(), 6);
        let count = metadata
            .fields()
            .iter()
            .find(|field| field.name() == "count")
            .expect("numeric default should exist");
        let Some(VocabularyValue::Number(number)) = count.default_value() else {
            panic!("count must retain an exact numeric default")
        };
        assert_eq!(number.coefficient(), "105");
        assert_eq!(number.scale(), -1);
        let node = bundle
            .logical()
            .type_by_name("Node")
            .expect("recursive identity type should exist");
        assert!(
            matches!(node.fields()[0].field_type(), VocabularyType::Ref(target) if target == "Node")
        );
    }

    #[test]
    /// Verifies byte formatting changes identity but not normalized logical meaning.
    fn property_vocabulary_order_and_format_are_logically_nonsemantic() {
        let first = validate(COMPREHENSIVE).expect("canonical bundle should validate");
        let second = validate(REORDERED).expect("reordered bundle should validate");
        assert_ne!(first.content_digest(), second.content_digest());
        assert_ne!(first, second);
        assert!(first.logically_equivalent(&second));
    }

    #[test]
    /// Verifies duplicate keys are rejected at envelope and nested object depths.
    fn security_vocabulary_duplicate_members_fail_before_map_collapse() {
        for bytes in [
            include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/duplicate-envelope-key.json").as_slice(),
            include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/duplicate-nested-key.json").as_slice(),
        ] {
            assert_eq!(validate(bytes), Err(VocabularyError::DuplicateJsonMember));
        }
    }

    #[test]
    /// Verifies unknown and executable-looking shapes fail the closed schema.
    fn security_vocabulary_closed_schema_rejects_unknown_and_executable_shapes() {
        let unknown = include_bytes!(
            "../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/unknown-envelope-member.json"
        );
        let executable = include_bytes!(
            "../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/executable-member.json"
        );
        let executable_kind = include_bytes!(
            "../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/unknown-type-kind.json"
        );
        assert_eq!(validate(unknown), Err(VocabularyError::UnknownMember));
        assert_eq!(
            validate(executable),
            Err(VocabularyError::ExecutableShapeForbidden)
        );
        assert_eq!(
            validate(executable_kind),
            Err(VocabularyError::ExecutableShapeForbidden)
        );
    }

    #[test]
    /// Verifies raw numbers, truncation, BOM, invalid UTF-8, and surrogates fail.
    fn security_vocabulary_strict_json_bytes_fail_closed() {
        let raw_number = include_bytes!(
            "../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/raw-json-number.json"
        );
        let truncated = include_bytes!(
            "../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/truncated.json"
        );
        assert_eq!(
            validate(raw_number),
            Err(VocabularyError::RawJsonNumberForbidden)
        );
        assert_eq!(validate(truncated), Err(VocabularyError::MalformedJson));

        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend_from_slice(EMPTY_BUNDLE);
        assert_eq!(
            validate_captured_bundle(&bom, &lock(&bom), limits()),
            Err(VocabularyError::BomForbidden)
        );
        let invalid_utf8 = [0xff];
        assert_eq!(
            validate_captured_bundle(&invalid_utf8, &lock(&invalid_utf8), limits()),
            Err(VocabularyError::InvalidUtf8)
        );
        let surrogate = br#"{"format":"neutral-vocabulary-bundle","encoding_version":"0.1","schema_version":"0.1","identity":"Fixture","version":"0.1.0","required_features":[],"types":[{"name":"Metadata","fields":[{"name":"label","type":{"kind":"string"},"default":{"kind":"string","value":"\uD800"}}]}]}"#;
        assert_eq!(
            validate_captured_bundle(surrogate, &lock(surrogate), limits()),
            Err(VocabularyError::MalformedJson)
        );
    }

    #[test]
    /// Verifies type targets, embedding recursion, reference defaults, and features.
    fn conformance_vocabulary_semantic_graph_failures() {
        let cases = [
            (include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/unknown-type-target.json").as_slice(), VocabularyError::UnknownTypeTarget),
            (include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/embedded-recursion.json").as_slice(), VocabularyError::InvalidTypeRecursion),
            (include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/reference-default.json").as_slice(), VocabularyError::InvalidDefault),
            (include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/incompatible-default.json").as_slice(), VocabularyError::InvalidDefault),
            (include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/incomplete-record-default.json").as_slice(), VocabularyError::InvalidDefault),
            (include_bytes!("../../../portable/spec/v0/fixtures/vocabulary/bundles/negative/unknown-feature.json").as_slice(), VocabularyError::UnknownRequiredFeature),
        ];
        for (bytes, expected) in cases {
            assert_eq!(validate(bytes), Err(expected));
        }
    }

    #[test]
    /// Verifies digest and byte limits run before parsing or proportional work.
    fn security_vocabulary_integrity_and_limits_precede_parsing() {
        let malformed = b"{";
        assert_eq!(
            validate_captured_bundle(malformed, &lock(EMPTY_BUNDLE), limits()),
            Err(VocabularyError::DigestMismatch)
        );
        let limited = limits()
            .with_bundle_bytes(8)
            .expect("nonzero byte limit should be valid");
        assert_eq!(
            validate_captured_bundle(EMPTY_BUNDLE, &lock(EMPTY_BUNDLE), limited),
            Err(VocabularyError::BundleByteLimitExceeded)
        );
        let limited = limits()
            .with_object_members(2)
            .expect("nonzero member limit should be valid");
        assert_eq!(
            validate_captured_bundle(EMPTY_BUNDLE, &lock(EMPTY_BUNDLE), limited),
            Err(VocabularyError::JsonLimitExceeded)
        );
    }

    #[test]
    /// Verifies an allowed exact structural feature set is normalized and retained.
    fn conformance_vocabulary_exact_feature_lock() {
        let bytes = b"{\"format\":\"neutral-vocabulary-bundle\",\"encoding_version\":\"0.1\",\"schema_version\":\"0.1\",\"identity\":\"Fixture\",\"version\":\"0.1.0\",\"required_features\":[\"neutral.feature/records@1\"],\"types\":[]}";
        let feature = "neutral.feature/records@1".to_owned();
        let lock = lock_with_features(bytes, vec![feature.clone()]);
        let bundle = validate_captured_bundle(bytes, &lock, limits())
            .expect("exact locked feature should validate");
        assert_eq!(bundle.logical().required_features(), [feature]);
    }
}
