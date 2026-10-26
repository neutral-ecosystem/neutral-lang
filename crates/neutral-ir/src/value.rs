// SPDX-License-Identifier: Apache-2.0

//! Typed immutable meaning after source syntax and aliases have been resolved.
//!
//! This module owns scalar/container values, nominal records, typed references,
//! declarations and captured vocabulary field contracts. It stores meaning, not
//! source tokens, host resources or executable behavior. Constructors allow
//! assembly; full schema, reference and companion consistency belongs to the reader.
//!
//! These shared materialized types are distinct from the richer composition and
//! residual-expression models. Records carry nominal owners, lists retain order,
//! and references retain both a local target label and a durable symbol identity.
//! Schema fields/defaults and final values are separate: a nullable field is not
//! automatically optional, and a declared default is not evaluated by construction.
//!
//! Constructors preserve supplied field order and declared fingerprints. They do
//! not sort, materialize defaults or verify reference targets. Display is a safe
//! reader-facing projection of stored values, not a canonical transport or a
//! source round-trip format. Recursive helpers require already-bounded input.

use crate::{
    DeclarationFingerprint, ElementId, ExactNumber, ModuleSymbolIdentity, NominalTypeIdentity,
    VocabularyIdentity, VocabularyTypeIdentity,
};
use std::fmt;

/// Resolved minimal Neutral type identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResolvedType {
    /// Exact Neutral numeric type.
    Num,
    /// Unicode scalar-sequence string type.
    String,
    /// Exact Boolean type.
    Bool,
    /// Exact module-owned nominal record type.
    Record(NominalTypeIdentity),
    /// Exact nominal record type owned by a captured vocabulary.
    VocabularyRecord(VocabularyTypeIdentity),
    /// Ordered homogeneous list with one invariant element type.
    List(Box<ResolvedType>),
    /// Typed document-local identity reference with one invariant target type.
    Ref(Box<ResolvedType>),
    /// One outer nullable layer around an otherwise resolved type.
    Nullable(Box<ResolvedType>),
}

impl ResolvedType {
    /// Adds one outer nullable layer around the supplied type.
    /// Does not reject an already-nullable argument or perform semantic validation.
    #[must_use]
    pub fn nullable(inner: Self) -> Self {
        Self::Nullable(Box::new(inner))
    }

    /// Wraps one resolved type as an invariant ordered list element type.
    #[must_use]
    pub fn list(inner: Self) -> Self {
        Self::List(Box::new(inner))
    }

    /// Wraps one exact resolved target type as a document-local identity reference.
    #[must_use]
    pub fn reference(inner: Self) -> Self {
        Self::Ref(Box::new(inner))
    }

    /// Returns whether this type has an outer nullable layer.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        matches!(self, Self::Nullable(_))
    }

    /// Returns the inner resolved type when this type is nullable.
    #[must_use]
    pub fn nullable_inner(&self) -> Option<&Self> {
        match self {
            Self::Nullable(inner) => Some(inner),
            _ => None,
        }
    }

    /// Returns whether a logical value satisfies exact identity plus outer nullability.
    ///
    /// This is a contextual compatibility check, not complete value validation:
    /// records compare nominal owners rather than their fields and references
    /// compare declared target types without looking up the target declaration.
    /// Lists recurse over their items; callers must preflight depth/work first.
    #[must_use]
    pub fn accepts_value(&self, value: &LogicalValue) -> bool {
        match (self, value) {
            (Self::Num, LogicalValue::Number(_))
            | (Self::String, LogicalValue::String(_))
            | (Self::Bool, LogicalValue::Boolean(_))
            | (Self::Nullable(_), LogicalValue::Null) => true,
            (Self::Record(expected), LogicalValue::Record(value)) => {
                expected == value.nominal_type()
            }
            (Self::VocabularyRecord(expected), LogicalValue::VocabularyRecord(value)) => {
                expected == value.nominal_type()
            }
            (Self::List(expected), LogicalValue::List(items)) => {
                // An empty list satisfies any contextual element type; context supplies its type.
                items.iter().all(|item| expected.accepts_value(item))
            }
            (Self::Ref(expected), LogicalValue::Reference(reference)) => {
                expected.as_ref() == reference.target_type()
            }
            (Self::Nullable(inner), value) => inner.accepts_value(value),
            _ => false,
        }
    }
}

impl fmt::Display for ResolvedType {
    /// Formats the stable source-facing type name.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num => formatter.write_str("num"),
            Self::String => formatter.write_str("string"),
            Self::Bool => formatter.write_str("bool"),
            Self::Record(identity) => formatter.write_str(identity.name()),
            Self::VocabularyRecord(identity) => identity.fmt(formatter),
            Self::List(inner) => write!(formatter, "List<{inner}>"),
            Self::Ref(inner) => write!(formatter, "Ref<{inner}>"),
            Self::Nullable(inner) => write!(formatter, "{inner}?"),
        }
    }
}

/// Immutable logical value in the active public IR slice.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LogicalValue {
    /// Exact normalized Neutral number.
    Number(ExactNumber),
    /// Exact decoded Unicode scalar sequence.
    String(String),
    /// Exact Boolean value.
    Boolean(bool),
    /// Explicit null value, valid only with a nullable resolved type.
    Null,
    /// Contextually typed nominal record value with canonical field order.
    Record(RecordValue),
    /// Contextually typed vocabulary-owned record value.
    VocabularyRecord(VocabularyRecordValue),
    /// Ordered homogeneous logical values.
    List(Vec<LogicalValue>),
    /// Typed document-local identity edge to one binding declaration.
    Reference(IdentityReference),
}

impl fmt::Display for LogicalValue {
    /// Formats a deterministic reader-facing logical value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => number.fmt(formatter),
            Self::String(value) => format_safe_string(value, formatter),
            Self::Boolean(value) => value.fmt(formatter),
            Self::Null => formatter.write_str("null"),
            Self::Record(value) => value.fmt(formatter),
            Self::VocabularyRecord(value) => value.fmt(formatter),
            Self::List(items) => {
                formatter.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    item.fmt(formatter)?;
                }
                formatter.write_str("]")
            }
            Self::Reference(reference) => {
                write!(formatter, "ref(#{})", reference.target_element_id().get())
            }
        }
    }
}

/// One final vocabulary-owned contextual record value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyRecordValue {
    /// Exact qualified nominal type supplied by context.
    nominal_type: VocabularyTypeIdentity,
    /// Final fields in canonical name order.
    fields: Vec<RecordValueField>,
}

impl VocabularyRecordValue {
    /// Creates one validated vocabulary-owned record value.
    #[must_use]
    pub fn new(nominal_type: VocabularyTypeIdentity, fields: Vec<RecordValueField>) -> Self {
        Self {
            nominal_type,
            fields,
        }
    }
    /// Returns the exact qualified nominal type.
    #[must_use]
    pub const fn nominal_type(&self) -> &VocabularyTypeIdentity {
        &self.nominal_type
    }
    /// Returns final fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[RecordValueField] {
        &self.fields
    }
}

impl fmt::Display for VocabularyRecordValue {
    /// Formats a deterministic generic reader-facing value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("{")?;
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{}: {}", field.name(), field.value())?;
        }
        formatter.write_str("}")
    }
}

/// One typed document-local identity edge retained in logical IR.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentityReference {
    /// Authoritative document-local target binding identifier.
    pub(super) element_id: ElementId,
    /// Durable target identity used by fingerprints instead of the local ID.
    symbol_identity: ModuleSymbolIdentity,
    /// Redundant exact target-type constraint validated against the declaration.
    resolved_type: ResolvedType,
}

impl IdentityReference {
    /// Creates one typed identity edge after target validation.
    /// The local ID, durable symbol and target type are stored independently;
    /// a reader must verify that all three describe the same declaration.
    #[must_use]
    pub fn new(
        target_element_id: ElementId,
        target_symbol_identity: ModuleSymbolIdentity,
        target_type: ResolvedType,
    ) -> Self {
        Self {
            element_id: target_element_id,
            symbol_identity: target_symbol_identity,
            resolved_type: target_type,
        }
    }

    /// Returns the authoritative document-local target identifier.
    #[must_use]
    pub const fn target_element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the target's module-symbol continuity identity.
    #[must_use]
    pub const fn target_symbol_identity(&self) -> &ModuleSymbolIdentity {
        &self.symbol_identity
    }

    /// Returns the redundant exact target-type constraint.
    #[must_use]
    pub const fn target_type(&self) -> &ResolvedType {
        &self.resolved_type
    }
}

/// One final field in a contextual record value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordValueField {
    /// Validated field name.
    name: String,
    /// Final recursively lowered logical value.
    value: LogicalValue,
}

impl RecordValueField {
    /// Creates one validated final record field.
    #[must_use]
    pub fn new(name: impl Into<String>, value: LogicalValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }

    /// Returns the validated field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the final field value.
    #[must_use]
    pub const fn value(&self) -> &LogicalValue {
        &self.value
    }
}

/// One contextually typed nominal record value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordValue {
    /// Exact nominal type supplied by context.
    nominal_type: NominalTypeIdentity,
    /// Final fields sorted by field name.
    fields: Vec<RecordValueField>,
}

impl RecordValue {
    /// Creates one validated contextual record value.
    #[must_use]
    pub fn new(nominal_type: NominalTypeIdentity, fields: Vec<RecordValueField>) -> Self {
        Self {
            nominal_type,
            fields,
        }
    }

    /// Returns the exact contextual nominal type.
    #[must_use]
    pub const fn nominal_type(&self) -> &NominalTypeIdentity {
        &self.nominal_type
    }

    /// Returns final fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[RecordValueField] {
        &self.fields
    }
}

impl fmt::Display for RecordValue {
    /// Formats a deterministic reader-facing record value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("{")?;
        for (index, field) in self.fields.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{}: {}", field.name(), field.value())?;
        }
        formatter.write_str("}")
    }
}

/// Formats a logical string with all hostile control text escaped.
/// This is diagnostic text escaping, not shell, HTML or general JSON escaping.
fn format_safe_string(value: &str, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("\"")?;
    for character in value.chars() {
        match character {
            '"' => formatter.write_str("\\\"")?,
            '\\' => formatter.write_str("\\\\")?,
            '\n' => formatter.write_str("\\n")?,
            '\r' => formatter.write_str("\\r")?,
            '\t' => formatter.write_str("\\t")?,
            '\0' => formatter.write_str("\\0")?,
            character if character.is_control() => {
                write!(formatter, "\\u{{{:x}}}", u32::from(character))?;
            }
            character => write!(formatter, "{character}")?,
        }
    }
    formatter.write_str("\"")
}

/// One typed field contract in a nominal record declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFieldSchema {
    /// Validated field name.
    name: String,
    /// Fully resolved field type.
    resolved_type: ResolvedType,
    /// Final closed default value, or `None` when the field is required.
    default_value: Option<LogicalValue>,
}

impl RecordFieldSchema {
    /// Creates one validated required record field.
    #[must_use]
    pub fn new(name: impl Into<String>, resolved_type: ResolvedType) -> Self {
        Self {
            name: name.into(),
            resolved_type,
            default_value: None,
        }
    }

    /// Attaches one validated closed logical default to this field contract.
    /// Does not check type compatibility or apply the default to any value.
    #[must_use]
    pub fn with_default(mut self, default_value: LogicalValue) -> Self {
        self.default_value = Some(default_value);
        self
    }

    /// Returns the validated field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the fully resolved required field type.
    #[must_use]
    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }

    /// Returns the final closed default, or `None` when this field is required.
    #[must_use]
    pub const fn default_value(&self) -> Option<&LogicalValue> {
        self.default_value.as_ref()
    }

    /// Returns whether this field must be supplied explicitly.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.default_value.is_none()
    }
}

/// One exported nominal record type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordTypeDefinition {
    /// Graph-local declaration identifier.
    element_id: ElementId,
    /// Durable module-symbol continuity identity.
    symbol_identity: ModuleSymbolIdentity,
    /// Logical schema fingerprint.
    fingerprint: DeclarationFingerprint,
    /// Exact nominal record identity.
    nominal_identity: NominalTypeIdentity,
    /// Required fields in canonical name order.
    fields: Vec<RecordFieldSchema>,
}

impl RecordTypeDefinition {
    /// Creates one fully validated nominal record declaration.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        symbol_identity: ModuleSymbolIdentity,
        fingerprint: DeclarationFingerprint,
        nominal_identity: NominalTypeIdentity,
        fields: Vec<RecordFieldSchema>,
    ) -> Self {
        Self {
            element_id,
            symbol_identity,
            fingerprint,
            nominal_identity,
            fields,
        }
    }

    /// Returns the graph-local declaration identifier.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the module-symbol continuity identity.
    #[must_use]
    pub const fn symbol_identity(&self) -> &ModuleSymbolIdentity {
        &self.symbol_identity
    }

    /// Returns the logical schema fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> DeclarationFingerprint {
        self.fingerprint
    }

    /// Returns the exact nominal identity.
    #[must_use]
    pub const fn nominal_identity(&self) -> &NominalTypeIdentity {
        &self.nominal_identity
    }

    /// Returns the validated record name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.nominal_identity.name()
    }

    /// Returns required fields in canonical name order.
    #[must_use]
    pub fn fields(&self) -> &[RecordFieldSchema] {
        &self.fields
    }
}

/// One exported immutable logical declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    /// Graph-local identifier.
    pub(super) element_id: ElementId,
    /// Durable module-symbol continuity identity.
    symbol_identity: ModuleSymbolIdentity,
    /// Logical definition fingerprint.
    pub(super) fingerprint: DeclarationFingerprint,
    /// Validated source declaration name.
    name: String,
    /// Resolved declared type.
    pub(super) resolved_type: ResolvedType,
    /// Final immutable logical value.
    pub(super) value: LogicalValue,
}

impl Declaration {
    /// Creates one fully validated logical binding declaration.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        symbol_identity: ModuleSymbolIdentity,
        fingerprint: DeclarationFingerprint,
        name: impl Into<String>,
        resolved_type: ResolvedType,
        value: LogicalValue,
    ) -> Self {
        Self {
            element_id,
            symbol_identity,
            fingerprint,
            name: name.into(),
            resolved_type,
            value,
        }
    }

    /// Returns the graph-local element identifier.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the module-symbol continuity identity.
    #[must_use]
    pub const fn symbol_identity(&self) -> &ModuleSymbolIdentity {
        &self.symbol_identity
    }

    /// Returns the logical definition fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> DeclarationFingerprint {
        self.fingerprint
    }

    /// Returns the validated source name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the resolved type.
    #[must_use]
    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }

    /// Returns the final immutable logical value.
    #[must_use]
    pub const fn value(&self) -> &LogicalValue {
        &self.value
    }
}

/// One immutable field in a captured vocabulary type contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyFieldContract {
    /// Canonical field name.
    name: String,
    /// Fully resolved field type.
    resolved_type: ResolvedType,
    /// Final closed vocabulary default, when present.
    default_value: Option<LogicalValue>,
}

impl VocabularyFieldContract {
    /// Creates one validated vocabulary field contract.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        resolved_type: ResolvedType,
        default_value: Option<LogicalValue>,
    ) -> Self {
        Self {
            name: name.into(),
            resolved_type,
            default_value,
        }
    }
    /// Returns the canonical field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the fully resolved field type.
    #[must_use]
    pub const fn resolved_type(&self) -> &ResolvedType {
        &self.resolved_type
    }
    /// Returns the final closed default, when present.
    #[must_use]
    pub const fn default_value(&self) -> Option<&LogicalValue> {
        self.default_value.as_ref()
    }
}

/// One immutable nominal type in a captured vocabulary contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyTypeContract {
    /// Exact qualified type identity.
    identity: VocabularyTypeIdentity,
    /// Canonical field contracts.
    fields: Vec<VocabularyFieldContract>,
}

impl VocabularyTypeContract {
    /// Creates one validated vocabulary type contract.
    #[must_use]
    pub fn new(identity: VocabularyTypeIdentity, fields: Vec<VocabularyFieldContract>) -> Self {
        Self { identity, fields }
    }
    /// Returns the exact qualified type identity.
    #[must_use]
    pub const fn identity(&self) -> &VocabularyTypeIdentity {
        &self.identity
    }
    /// Returns canonical field contracts.
    #[must_use]
    pub fn fields(&self) -> &[VocabularyFieldContract] {
        &self.fields
    }
}

/// Complete exact captured vocabulary contract retained in logical IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VocabularyContract {
    /// Exact captured and logical identity facts.
    pub(super) identity: VocabularyIdentity,
    /// Canonical nominal type contracts.
    pub(super) types: Vec<VocabularyTypeContract>,
}

impl VocabularyContract {
    /// Creates one validated exact vocabulary contract.
    #[must_use]
    pub fn new(identity: VocabularyIdentity, types: Vec<VocabularyTypeContract>) -> Self {
        Self { identity, types }
    }
    /// Returns exact captured and logical identity facts.
    #[must_use]
    pub const fn identity(&self) -> &VocabularyIdentity {
        &self.identity
    }
    /// Returns canonical nominal type contracts.
    #[must_use]
    pub fn types(&self) -> &[VocabularyTypeContract] {
        &self.types
    }
    /// Finds a type without external acquisition.
    #[must_use]
    pub fn type_by_name(&self, name: &str) -> Option<&VocabularyTypeContract> {
        self.types
            .iter()
            .find(|definition| definition.identity().name() == name)
    }

    /// Compares normalized logical vocabulary meaning without captured-byte facts.
    /// Ignores encoding version and captured digest, but retains semantic revision,
    /// schema, required features and complete type contracts. Ordinary equality
    /// remains stricter; choose it when exact retained lock facts must also agree.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        self.identity.identity == other.identity.identity
            && self.identity.version == other.identity.version
            && self.identity.schema_version == other.identity.schema_version
            && self.identity.required_features == other.identity.required_features
            && self.types == other.types
    }
}
