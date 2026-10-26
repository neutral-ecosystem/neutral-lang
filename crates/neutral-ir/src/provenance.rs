// SPDX-License-Identifier: Apache-2.0

//! Evidence describing where materialized values and their edges came from.
//!
//! Origin and normalization describe a value's source history. Field paths,
//! immutable reuse edges and typed reference edges retain distinct evidence;
//! copying a value is not the same operation as referring to a binding.
//! These records do not change logical meaning and require independent reader checks.
//!
//! Root records explain normalization; field records explain supplied/defaulted
//! values; reuse and reference records retain occurrence paths and distinct target
//! roles. Reuse copies meaning into a consuming location, whereas a reference
//! preserves an identity edge without embedding the target's value.
//!
//! Constructors record supplied claims without resolving paths, checking target
//! membership or proving correspondence with source bytes. Consumers must not
//! treat these records as authorization or invent evidence when it is absent.
//! The graph-local paths/IDs here belong to the materialized document model;
//! complete expression projects retain their own stable-owner companions.

use crate::ElementId;

/// Why a final logical value exists in the active document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOrigin {
    /// Value was written explicitly in captured source.
    ExplicitSource,
    /// A binding's final logical value was reused from another immutable binding.
    OrdinaryReuse,
    /// A typed identity edge was resolved from explicit `ref(name)` source.
    IdentityReference,
    /// A contextual record field was written explicitly.
    ExplicitRecordField,
    /// An omitted contextual field was materialized from a user-record default.
    UserRecordDefault,
    /// An omitted vocabulary-owned field was materialized from its captured contract.
    VocabularyDefault,
}

impl ValueOrigin {
    /// Returns the stable vocabulary spelling used by generic consumers.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitSource => "explicit-source",
            Self::OrdinaryReuse => "ordinary-reuse",
            Self::IdentityReference => "identity-reference",
            Self::ExplicitRecordField => "explicit-record-field",
            Self::UserRecordDefault => "user-record-default",
            Self::VocabularyDefault => "vocabulary-default",
        }
    }
}

/// Logical normalization applied while lowering one value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Normalization {
    /// Exact source number was normalized without host floating point.
    ExactNumberCanonicalization,
    /// String escapes were decoded to their exact Unicode scalar sequence.
    StringEscapeDecoding,
    /// A Boolean token was lowered without value transformation.
    BooleanIdentity,
    /// An explicit null token was lowered without structural omission.
    NullIdentity,
    /// Explicit record fields were matched to one nominal contextual schema.
    RecordContextualization,
    /// Ordered list items were checked against one contextual element type.
    ListContextualization,
    /// An ordinary immutable binding name was replaced by its final logical value.
    ImmutableValueReuse,
    /// A `ref(name)` target was resolved to a typed document-local identity edge.
    IdentityReferenceResolution,
}

impl Normalization {
    /// Returns the stable external vocabulary spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExactNumberCanonicalization => "exact-number",
            Self::StringEscapeDecoding => "string-escape",
            Self::BooleanIdentity => "boolean-identity",
            Self::NullIdentity => "null-identity",
            Self::RecordContextualization => "record-context",
            Self::ListContextualization => "list-context",
            Self::ImmutableValueReuse => "immutable-reuse",
            Self::IdentityReferenceResolution => "identity-reference",
        }
    }
}

/// Provenance record for one minimal binding value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvenanceRecord {
    /// Graph-local element whose value is explained.
    element_id: ElementId,
    /// Why the value exists.
    origin: ValueOrigin,
    /// Logical normalization applied to the source value.
    normalization: Normalization,
}

impl ProvenanceRecord {
    /// Records the declared origin and normalization for one materialized binding.
    /// Does not verify that the selected categories match the binding's actual value.
    #[must_use]
    pub const fn new(
        element_id: ElementId,
        origin: ValueOrigin,
        normalization: Normalization,
    ) -> Self {
        Self {
            element_id,
            origin,
            normalization,
        }
    }

    /// Returns the owning graph-local element.
    #[must_use]
    pub const fn element_id(self) -> ElementId {
        self.element_id
    }

    /// Returns why the value exists.
    #[must_use]
    pub const fn origin(self) -> ValueOrigin {
        self.origin
    }

    /// Returns the logical normalization applied to the value.
    #[must_use]
    pub const fn normalization(self) -> Normalization {
        self.normalization
    }
}

/// Provenance for one final field inside a binding's contextual record value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldProvenanceRecord {
    /// Graph-local binding element that owns the record value.
    element_id: ElementId,
    /// Canonical field path from the binding value root.
    field_path: Vec<String>,
    /// Whether the field was explicit or supplied by a user default.
    origin: ValueOrigin,
}

/// Provenance for one ordinary immutable-value reuse edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReuseProvenanceRecord {
    /// Graph-local binding element that owns the reused value occurrence.
    element_id: ElementId,
    /// Canonical record-field/list-index path from the binding value root.
    value_path: Vec<String>,
    /// Graph-local binding element whose final logical value was reused.
    source_element_id: ElementId,
}

impl ReuseProvenanceRecord {
    /// Creates one validated ordinary immutable-value reuse edge.
    /// The path identifies an occurrence in the consumer, not a traversal request
    /// into the source binding; neither endpoint is resolved by this constructor.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        value_path: Vec<String>,
        source_element_id: ElementId,
    ) -> Self {
        Self {
            element_id,
            value_path,
            source_element_id,
        }
    }

    /// Returns the binding that owns the reused value occurrence.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the canonical path from the owning binding value root.
    #[must_use]
    pub fn value_path(&self) -> &[String] {
        &self.value_path
    }

    /// Returns the binding whose final logical value was reused.
    #[must_use]
    pub const fn source_element_id(&self) -> ElementId {
        self.source_element_id
    }
}

/// Provenance for one typed identity-reference edge occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceProvenanceRecord {
    /// Graph-local binding element that owns the reference occurrence.
    element_id: ElementId,
    /// Canonical record-field/list-index path from the binding value root.
    value_path: Vec<String>,
    /// Authoritative graph-local target binding identifier.
    target_element_id: ElementId,
}

impl ReferenceProvenanceRecord {
    /// Creates one validated typed identity-reference provenance edge.
    #[must_use]
    pub fn new(
        element_id: ElementId,
        value_path: Vec<String>,
        target_element_id: ElementId,
    ) -> Self {
        Self {
            element_id,
            value_path,
            target_element_id,
        }
    }

    /// Returns the binding that owns the reference occurrence.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the canonical path from the owning binding value root.
    #[must_use]
    pub fn value_path(&self) -> &[String] {
        &self.value_path
    }

    /// Returns the authoritative document-local target binding identifier.
    #[must_use]
    pub const fn target_element_id(&self) -> ElementId {
        self.target_element_id
    }
}

impl FieldProvenanceRecord {
    /// Creates one validated record-field provenance entry.
    #[must_use]
    pub fn new(element_id: ElementId, field_path: Vec<String>, origin: ValueOrigin) -> Self {
        Self {
            element_id,
            field_path,
            origin,
        }
    }

    /// Returns the owning binding element.
    #[must_use]
    pub const fn element_id(&self) -> ElementId {
        self.element_id
    }

    /// Returns the canonical field path from the binding root.
    #[must_use]
    pub fn field_path(&self) -> &[String] {
        &self.field_path
    }

    /// Returns why this final field value exists.
    #[must_use]
    pub const fn origin(&self) -> ValueOrigin {
        self.origin
    }
}
