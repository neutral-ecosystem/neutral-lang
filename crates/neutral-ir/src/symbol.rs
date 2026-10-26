// SPDX-License-Identifier: Apache-2.0

//! Namespaced identities used to connect typed declarations and references.
//!
//! Module, symbol and nominal type identities express logical ownership;
//! vocabulary identities retain the captured bundle contract and digest.
//! `ElementId` is deliberately different: it is an opaque document-local label,
//! not a content fingerprint or a stable identifier across compilations.
//!
//! A module-symbol tuple follows declaration continuity when its value changes;
//! a nominal identity additionally distinguishes types by their owning namespace.
//! Vocabulary lock facts include exact revisions and captured bytes, while a
//! vocabulary type name must be interpreted within the surrounding validated lock
//! context. Local aliases and machine paths are not substitutes for these owners.
//!
//! Constructors assemble the requested components without parsing names, checking
//! feature order, resolving declarations or recomputing digests. Producers and
//! independent readers must establish those invariants before using the identities
//! for lookup. Derived equality/order compare the stored components exactly.

use neutral_core::VocabularyContentDigest;
use std::fmt;

/// Structured logical identity of one Neutral module.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogicalModuleIdentity {
    /// Frozen language behavior version.
    pub(super) language_behavior_version: String,
    /// Validated logical module name.
    pub(super) module_name: String,
}

impl LogicalModuleIdentity {
    /// Creates a logical module identity from validated semantic values.
    /// Does not validate either string or consult the profile catalogue.
    #[must_use]
    pub fn new(
        language_behavior_version: impl Into<String>,
        module_name: impl Into<String>,
    ) -> Self {
        Self {
            language_behavior_version: language_behavior_version.into(),
            module_name: module_name.into(),
        }
    }

    /// Returns the frozen language behavior version.
    #[must_use]
    pub fn language_behavior_version(&self) -> &str {
        &self.language_behavior_version
    }

    /// Returns the validated logical module name.
    #[must_use]
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

/// Exact nominal identity of one user record type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NominalTypeIdentity {
    /// Logical module that owns the record declaration.
    module: LogicalModuleIdentity,
    /// Validated uppercase-leading record name.
    name: String,
}

impl NominalTypeIdentity {
    /// Creates one module-owned nominal record identity.
    ///
    /// This assembles a value from already validated names; it does not parse
    /// source or enforce the declaration grammar. Readers must check hostile
    /// external components before using the identity to resolve a type.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, name: impl Into<String>) -> Self {
        Self {
            module,
            name: name.into(),
        }
    }

    /// Returns the module that owns this nominal type.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns the validated record type name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Structured continuity identity for one declaration in a logical module.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleSymbolIdentity {
    /// Logical module that owns the declaration.
    pub(super) module: LogicalModuleIdentity,
    /// Validated declaration source name.
    pub(super) declaration_name: String,
}

impl ModuleSymbolIdentity {
    /// Creates a module-symbol identity from validated semantic values.
    ///
    /// The tuple denotes declaration continuity, not its current content. A
    /// declaration can keep this identity while its value or signature changes;
    /// content-sensitive decisions require the appropriate fingerprint/digest.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, declaration_name: impl Into<String>) -> Self {
        Self {
            module,
            declaration_name: declaration_name.into(),
        }
    }

    /// Returns the owning logical module identity.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns the declaration name used for continuity.
    #[must_use]
    pub fn declaration_name(&self) -> &str {
        &self.declaration_name
    }
}

/// An opaque graph-local element label without cross-document meaning.
///
/// This label is valid only inside its owning logical document. Persist
/// [`ModuleSymbolIdentity`] for continuity; never serialize or compare this
/// numeric spelling as a durable external identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementId(u64);

impl ElementId {
    /// Creates a validated-document-local element identifier.
    /// Accepts any integer; uniqueness and membership belong to the owning document.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the local integer label for indexing within one document only.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Exact captured and logical identity of one vocabulary contract.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyIdentity {
    /// Logical vocabulary name used by source qualification.
    pub(super) identity: String,
    /// Exact vocabulary release version.
    pub(super) version: String,
    /// Exact logical vocabulary schema version.
    pub(super) schema_version: String,
    /// Exact captured bundle encoding version.
    pub(super) encoding_version: String,
    /// Digest of the exact captured bundle bytes.
    pub(super) content_digest: VocabularyContentDigest,
    /// Required immutable structural feature IDs in canonical order.
    pub(super) required_features: Vec<String>,
}

impl VocabularyIdentity {
    /// Creates an exact validated vocabulary identity contract.
    /// Stores features in the supplied order and trusts the declared digest;
    /// canonical order, revision support and byte integrity require external checks.
    #[must_use]
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        schema_version: impl Into<String>,
        encoding_version: impl Into<String>,
        content_digest: VocabularyContentDigest,
        required_features: Vec<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            version: version.into(),
            schema_version: schema_version.into(),
            encoding_version: encoding_version.into(),
            content_digest,
            required_features,
        }
    }

    /// Returns the logical vocabulary name.
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
    /// Returns the captured encoding version.
    #[must_use]
    pub fn encoding_version(&self) -> &str {
        &self.encoding_version
    }
    /// Returns the exact captured-byte digest.
    #[must_use]
    pub const fn content_digest(&self) -> VocabularyContentDigest {
        self.content_digest
    }
    /// Returns canonical required structural feature IDs.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        &self.required_features
    }
}

/// Exact qualified identity of one vocabulary-owned nominal type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VocabularyTypeIdentity {
    /// Logical vocabulary namespace.
    vocabulary: String,
    /// Vocabulary-owned nominal type name.
    name: String,
}

impl VocabularyTypeIdentity {
    /// Creates a validated qualified vocabulary type identity.
    #[must_use]
    pub fn new(vocabulary: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            vocabulary: vocabulary.into(),
            name: name.into(),
        }
    }
    /// Returns the vocabulary namespace.
    #[must_use]
    pub fn vocabulary(&self) -> &str {
        &self.vocabulary
    }
    /// Returns the vocabulary-owned type name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for VocabularyTypeIdentity {
    /// Formats the stable qualified source type.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}::{}", self.vocabulary, self.name)
    }
}
