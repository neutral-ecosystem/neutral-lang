// SPDX-License-Identifier: Apache-2.0

//! Strict v1 project vocabulary validation, independent of v0 bundle decoding.

use crate::{
    JsonValue, VocabularyError, VocabularyLimits, VocabularyLock, json, schema, validation,
};
use neutral_core::VocabularyContentDigest;
use neutral_ir::language;
use std::collections::{BTreeMap, BTreeSet};

/// Exact v1 project bundle encoding version.
pub const PROJECT_VOCABULARY_ENCODING_VERSION: &str = schema::PROJECT_ENCODING_VERSION;
/// Exact v1 project logical schema version.
pub const PROJECT_VOCABULARY_SCHEMA_VERSION: &str = schema::PROJECT_SCHEMA_VERSION;

/// A closed source-authorable v1 vocabulary field type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectVocabularyType {
    /// Exact number.
    Num,
    /// Unicode string.
    String,
    /// Boolean.
    Bool,
    /// Inert URL text.
    Url,
    /// Inert path text.
    Path,
    /// Nominal type within the same locked vocabulary.
    Nominal(String),
}

/// One canonical field in a v1 nominal vocabulary type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabularyField {
    /// Canonical field name.
    name: String,
    /// Closed field type.
    ty: ProjectVocabularyType,
}

impl ProjectVocabularyField {
    /// Returns the canonical field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the closed field type.
    #[must_use]
    pub const fn ty(&self) -> &ProjectVocabularyType {
        &self.ty
    }
}

/// One validated nominal type with an explicit source-visibility bit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabularyTypeDefinition {
    /// Canonical nominal name.
    name: String,
    /// Explicit source visibility.
    public: bool,
    /// Fields in canonical name order.
    fields: Vec<ProjectVocabularyField>,
}

impl ProjectVocabularyTypeDefinition {
    /// Returns the canonical nominal name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether source may name this type.
    #[must_use]
    pub const fn is_public(&self) -> bool {
        self.public
    }

    /// Returns fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[ProjectVocabularyField] {
        &self.fields
    }
}

/// Complete validated v1 vocabulary, excluding aliases and authoring metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabulary {
    /// Canonical locked identity.
    identity: String,
    /// Exact locked release.
    version: String,
    /// Digest of exact captured bytes.
    content_digest: VocabularyContentDigest,
    /// Nominal types in canonical name order.
    types: Vec<ProjectVocabularyTypeDefinition>,
}

impl ProjectVocabulary {
    /// Returns the canonical identity from the exact lock.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the locked semantic release.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the digest of exact captured bytes, not a semantic alias.
    #[must_use]
    pub const fn content_digest(&self) -> VocabularyContentDigest {
        self.content_digest
    }

    /// Returns types in canonical name order.
    #[must_use]
    pub fn types(&self) -> &[ProjectVocabularyTypeDefinition] {
        &self.types
    }

    /// Returns an explicitly public source-authorable type, if present.
    #[must_use]
    pub fn public_type(&self, name: &str) -> Option<&ProjectVocabularyTypeDefinition> {
        self.types.iter().find(|ty| ty.name == name && ty.public)
    }
}

/// Validates one exact, bounded, data-only v1 bundle against its semantic lock.
///
/// # Errors
///
/// Returns a closed-schema or lock error without exposing a partial contract.
#[expect(
    clippy::too_many_lines,
    reason = "the closed schema is validated in one fail-closed pass"
)]
pub fn validate_project_bundle(
    bytes: &[u8],
    lock: &VocabularyLock,
    limits: VocabularyLimits,
) -> Result<ProjectVocabulary, VocabularyError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bundle_bytes() {
        return Err(VocabularyError::BundleByteLimitExceeded);
    }
    let digest = VocabularyContentDigest::from_bytes(bytes);
    if !digest.securely_matches(lock.content_digest()) {
        return Err(VocabularyError::DigestMismatch);
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(VocabularyError::BomForbidden);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| VocabularyError::InvalidUtf8)?;
    let parsed = json::parse(text, limits)?;
    let root = validation::object(&parsed)?;
    validation::exact_members(
        root,
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
    if validation::string(root, "format")? != schema::BUNDLE_FORMAT {
        return Err(VocabularyError::UnsupportedFormat);
    }
    if validation::string(root, "encoding_version")? != PROJECT_VOCABULARY_ENCODING_VERSION
        || lock.encoding_version() != PROJECT_VOCABULARY_ENCODING_VERSION
    {
        return Err(VocabularyError::UnsupportedEncodingVersion);
    }
    if validation::string(root, "schema_version")? != PROJECT_VOCABULARY_SCHEMA_VERSION
        || lock.schema_version() != PROJECT_VOCABULARY_SCHEMA_VERSION
    {
        return Err(VocabularyError::UnsupportedSchemaVersion);
    }
    if validation::string(root, "identity")? != lock.identity()
        || validation::string(root, "version")? != lock.version()
    {
        return Err(VocabularyError::LockMismatch);
    }
    let features = validation::array(validation::member(root, "required_features")?)?;
    if u64::try_from(features.len()).unwrap_or(u64::MAX) > limits.features() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut decoded_features = BTreeSet::new();
    for feature in features {
        let JsonValue::String(value) = feature else {
            return Err(VocabularyError::InvalidMemberType);
        };
        if !schema::is_feature_id(value) {
            return Err(VocabularyError::InvalidFeatureId);
        }
        if !decoded_features.insert(value.as_str()) {
            return Err(VocabularyError::DuplicateFeature);
        }
    }
    if decoded_features
        .into_iter()
        .ne(lock.required_features().iter().map(String::as_str))
    {
        return Err(VocabularyError::LockMismatch);
    }
    let definitions = validation::array(validation::member(root, "types")?)?;
    if u64::try_from(definitions.len()).unwrap_or(u64::MAX) > limits.types() {
        return Err(VocabularyError::JsonLimitExceeded);
    }
    let mut visibility = BTreeMap::new();
    for definition in definitions {
        let fields = validation::object(definition)?;
        validation::exact_members(fields, &["name", "public", "fields"])?;
        let name = validation::string(fields, "name")?;
        if !schema::is_upper_name(name) || schema::is_protected_name(name) {
            return Err(VocabularyError::InvalidTypeName);
        }
        if visibility
            .insert(name.to_owned(), validation::boolean(fields, "public")?)
            .is_some()
        {
            return Err(VocabularyError::DuplicateType);
        }
    }
    let mut types = Vec::with_capacity(definitions.len());
    for definition in definitions {
        let fields = validation::object(definition)?;
        let name = validation::string(fields, "name")?;
        let public = visibility[name];
        let raw_fields = validation::array(validation::member(fields, "fields")?)?;
        if u64::try_from(raw_fields.len()).unwrap_or(u64::MAX) > limits.fields() {
            return Err(VocabularyError::JsonLimitExceeded);
        }
        let mut seen = BTreeSet::new();
        let mut decoded = Vec::with_capacity(raw_fields.len());
        for raw in raw_fields {
            let field = validation::object(raw)?;
            validation::exact_members(field, &["name", "type"])?;
            let field_name = validation::string(field, "name")?;
            if !schema::is_snake_name(field_name) || schema::is_protected_name(field_name) {
                return Err(VocabularyError::InvalidFieldName);
            }
            if !seen.insert(field_name) {
                return Err(VocabularyError::DuplicateField);
            }
            let spelling = validation::string(field, "type")?;
            let ty = match spelling {
                "num" => ProjectVocabularyType::Num,
                "string" => ProjectVocabularyType::String,
                "bool" => ProjectVocabularyType::Bool,
                language::URL => ProjectVocabularyType::Url,
                language::PATH => ProjectVocabularyType::Path,
                other => {
                    let Some(target_public) = visibility.get(other) else {
                        return Err(VocabularyError::UnknownTypeTarget);
                    };
                    if public && !target_public {
                        return Err(VocabularyError::PrivateTypeExposed);
                    }
                    ProjectVocabularyType::Nominal(other.to_owned())
                }
            };
            decoded.push(ProjectVocabularyField {
                name: field_name.to_owned(),
                ty,
            });
        }
        decoded.sort_by(|left, right| left.name.cmp(&right.name));
        types.push(ProjectVocabularyTypeDefinition {
            name: name.to_owned(),
            public,
            fields: decoded,
        });
    }
    types.sort_by(|left, right| left.name.cmp(&right.name));
    validate_embedding_graph(&types, limits)?;
    Ok(ProjectVocabulary {
        identity: lock.identity().to_owned(),
        version: lock.version().to_owned(),
        content_digest: digest,
        types,
    })
}

/// Rejects embedded nominal cycles with a bounded iterative graph walk.
fn validate_embedding_graph(
    types: &[ProjectVocabularyTypeDefinition],
    limits: VocabularyLimits,
) -> Result<(), VocabularyError> {
    let graph = types
        .iter()
        .map(|ty| {
            let targets = ty
                .fields
                .iter()
                .filter_map(|field| match &field.ty {
                    ProjectVocabularyType::Nominal(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            (ty.name.as_str(), targets)
        })
        .collect::<BTreeMap<_, _>>();
    let mut visited = BTreeSet::new();
    let mut traversed = 0_u64;
    for root in graph.keys() {
        if visited.contains(root) {
            continue;
        }
        let mut visiting = BTreeSet::new();
        let mut stack = vec![(*root, false)];
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
