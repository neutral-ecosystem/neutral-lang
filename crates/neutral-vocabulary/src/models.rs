// SPDX-License-Identifier: Apache-2.0

//! Immutable normalized vocabulary types, defaults and validated bundle projections.

use neutral_core::VocabularyContentDigest;
use neutral_ir::ExactNumber;

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
    pub(crate) name: String,
    /// Final closed field value.
    pub(crate) value: VocabularyValue,
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
    pub(crate) name: String,
    /// Fully resolved closed field type.
    pub(crate) field_type: VocabularyType,
    /// Final closed default, or no default.
    pub(crate) default_value: Option<VocabularyValue>,
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
    pub(crate) name: String,
    /// Fields in canonical name order.
    pub(crate) fields: Vec<VocabularyField>,
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
    pub(crate) identity: String,
    /// Exact release version.
    pub(crate) version: String,
    /// Logical schema version.
    pub(crate) schema_version: String,
    /// Normalized structural feature set.
    pub(crate) required_features: Vec<String>,
    /// Nominal types in canonical name order.
    pub(crate) types: Vec<VocabularyTypeDefinition>,
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
    pub(crate) encoding_version: String,
    /// Exact captured-byte identity.
    pub(crate) content_digest: VocabularyContentDigest,
    /// Normalized logical vocabulary contract.
    pub(crate) logical: LogicalVocabulary,
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
