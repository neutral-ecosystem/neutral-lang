// SPDX-License-Identifier: Apache-2.0

//! Public-interface facts, separate from the future complete project IR.

use crate::ModuleSymbolIdentity;
use neutral_core::{CoreError, SemanticDigest, nht_frame, profile::V1_SOURCE_PROFILE};

/// Domain separating public signature fingerprints from all other identities.
pub const PROJECT_INTERFACE_FINGERPRINT_DOMAIN: &str = "neutral/project-interface/v1";
/// Maximum nested type layers accepted by the public-interface contract.
pub const MAX_PROJECT_INTERFACE_TYPE_DEPTH: usize = 64;

/// A canonical public type signature with alias-independent nominal identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectPublicType {
    /// Exact number type.
    Num,
    /// Unicode string type.
    String,
    /// Boolean type.
    Bool,
    /// Inert URL text, distinct from a string or path.
    Url,
    /// Inert path text, distinct from a string or URL.
    Path,
    /// Public nominal vocabulary type under its exact canonical lock.
    VocabularyNominal {
        /// Canonical vocabulary identity, not a source alias.
        identity: String,
        /// Exact semantic release from the captured lock.
        version: String,
        /// Public nominal type name.
        name: String,
    },
    /// Public nominal record owned by one logical module.
    Nominal(ModuleSymbolIdentity),
    /// Invariant ordered list type.
    List(Box<Self>),
    /// Invariant identity-reference target type.
    Ref(Box<Self>),
    /// One explicit outer nullable layer.
    Nullable(Box<Self>),
}

impl ProjectPublicType {
    /// Frames one bounded canonical type for the interface transcript.
    fn transcript(&self, depth: usize) -> Result<Vec<u8>, CoreError> {
        if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
            return Err(CoreError::TranscriptLengthExceeded);
        }
        match self {
            Self::Num => nht_frame("num", &[]),
            Self::String => nht_frame("string", &[]),
            Self::Bool => nht_frame("bool", &[]),
            Self::Url => nht_frame("url", &[]),
            Self::Path => nht_frame("path", &[]),
            Self::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                let mut payload = nht_frame("identity", identity.as_bytes())?;
                payload.extend(nht_frame("version", version.as_bytes())?);
                payload.extend(nht_frame("name", name.as_bytes())?);
                nht_frame("vocabulary-nominal", &payload)
            }
            Self::Nominal(identity) => nht_frame("nominal", &identity_transcript(identity)?),
            Self::List(inner) => nht_frame("list", &inner.transcript(depth + 1)?),
            Self::Ref(inner) => nht_frame("ref", &inner.transcript(depth + 1)?),
            Self::Nullable(inner) => nht_frame("nullable", &inner.transcript(depth + 1)?),
        }
    }
}

/// A source-authored inert location scalar retained as exact decoded text.
///
/// This value carries no resolver, filesystem handle, URL client, permission,
/// or normalization policy. Its variants are intentionally not interchangeable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectLocationValue {
    /// Exact decoded text of a `url` value.
    Url(String),
    /// Exact decoded text of a `path` value.
    Path(String),
}

impl ProjectLocationValue {
    /// Returns the exact decoded text without interpretation.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Url(text) | Self::Path(text) => text,
        }
    }
}

/// One public record field's name and canonical type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPublicField {
    /// Exact canonical field name.
    name: String,
    /// Alias-independent field type.
    ty: ProjectPublicType,
}

impl ProjectPublicField {
    /// Constructs a field for a canonical public record signature.
    #[must_use]
    pub fn new(name: impl Into<String>, ty: ProjectPublicType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }

    /// Returns the canonical field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the canonical field type.
    #[must_use]
    pub const fn ty(&self) -> &ProjectPublicType {
        &self.ty
    }

    /// Frames one field without depending on source order or trivia.
    fn transcript(&self) -> Result<Vec<u8>, CoreError> {
        let mut payload = nht_frame("name", self.name.as_bytes())?;
        payload.extend(nht_frame("type", &self.ty.transcript(0)?)?);
        nht_frame("field", &payload)
    }
}

/// One public declaration signature, excluding private implementation values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectPublicSignature {
    /// An immutable binding's explicit type.
    Binding(ProjectPublicType),
    /// A nominal record's fields in canonical name order.
    Record(Vec<ProjectPublicField>),
}

impl ProjectPublicSignature {
    /// Frames one public declaration signature.
    fn transcript(&self) -> Result<Vec<u8>, CoreError> {
        match self {
            Self::Binding(ty) => nht_frame("binding", &ty.transcript(0)?),
            Self::Record(fields) => {
                let mut payload = Vec::new();
                for field in fields {
                    payload.extend(field.transcript()?);
                }
                nht_frame("record", &payload)
            }
        }
    }
}

/// One exported root with no private source or provenance data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPublicExport {
    /// Stable full module-symbol identity.
    identity: ModuleSymbolIdentity,
    /// Public type or record signature.
    signature: ProjectPublicSignature,
}

impl ProjectPublicExport {
    /// Constructs one public export entry.
    #[must_use]
    pub const fn new(identity: ModuleSymbolIdentity, signature: ProjectPublicSignature) -> Self {
        Self {
            identity,
            signature,
        }
    }

    /// Returns the full alias-independent symbol identity.
    #[must_use]
    pub const fn identity(&self) -> &ModuleSymbolIdentity {
        &self.identity
    }

    /// Returns its public signature without an implementation value.
    #[must_use]
    pub const fn signature(&self) -> &ProjectPublicSignature {
        &self.signature
    }

    /// Frames one canonical exported declaration.
    fn transcript(&self) -> Result<Vec<u8>, CoreError> {
        let mut payload = identity_transcript(&self.identity)?;
        payload.extend(nht_frame("signature", &self.signature.transcript()?)?);
        nht_frame("export", &payload)
    }
}

/// Public dependency kind retained after private provenance redaction.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectPublicEdgeKind {
    /// Public nominal type dependency.
    Type,
    /// Public nominal target under a reference type.
    ReferenceType,
    /// Immutable value reuse between public roots.
    Value,
    /// Identity-only reference between public roots.
    Reference,
}

impl ProjectPublicEdgeKind {
    /// Returns the stable transcript spelling of one typed edge.
    const fn spelling(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::ReferenceType => "reference-type",
            Self::Value => "value",
            Self::Reference => "reference",
        }
    }
}

/// One public-to-public edge without source ID, span, or private target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPublicEdge {
    /// Public consumer root.
    from: ModuleSymbolIdentity,
    /// Public dependency target.
    to: ModuleSymbolIdentity,
    /// Typed public dependency category.
    kind: ProjectPublicEdgeKind,
}

/// One locked vocabulary's public, alias-free type catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPublicVocabulary {
    /// Canonical identity from the exact captured lock.
    identity: String,
    /// Exact semantic release from that lock.
    version: String,
    /// Source-authorable type names in canonical order; private names are absent.
    public_types: Vec<String>,
}

impl ProjectPublicVocabulary {
    /// Constructs one public-only locked vocabulary fact.
    #[must_use]
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        public_types: Vec<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            version: version.into(),
            public_types,
        }
    }

    /// Returns the canonical identity, never a local source alias.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the exact locked semantic release.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns only public source-authorable type names.
    #[must_use]
    pub fn public_types(&self) -> &[String] {
        &self.public_types
    }

    /// Frames the complete public fact without alias or authoring metadata.
    fn transcript(&self) -> Result<Vec<u8>, CoreError> {
        let mut payload = nht_frame("identity", self.identity.as_bytes())?;
        payload.extend(nht_frame("version", self.version.as_bytes())?);
        for name in &self.public_types {
            payload.extend(nht_frame("public-type", name.as_bytes())?);
        }
        nht_frame("vocabulary", &payload)
    }
}

impl ProjectPublicEdge {
    /// Constructs one redacted public dependency edge.
    #[must_use]
    pub const fn new(
        from: ModuleSymbolIdentity,
        to: ModuleSymbolIdentity,
        kind: ProjectPublicEdgeKind,
    ) -> Self {
        Self { from, to, kind }
    }

    /// Returns the public consumer identity.
    #[must_use]
    pub const fn from(&self) -> &ModuleSymbolIdentity {
        &self.from
    }

    /// Returns the public target identity.
    #[must_use]
    pub const fn to(&self) -> &ModuleSymbolIdentity {
        &self.to
    }

    /// Returns the public edge category.
    #[must_use]
    pub const fn kind(&self) -> ProjectPublicEdgeKind {
        self.kind
    }

    /// Frames one alias-independent edge without source provenance.
    fn transcript(&self) -> Result<Vec<u8>, CoreError> {
        let mut payload = nht_frame("from", &identity_transcript(&self.from)?)?;
        payload.extend(nht_frame("to", &identity_transcript(&self.to)?)?);
        payload.extend(nht_frame("kind", self.kind.spelling().as_bytes())?);
        nht_frame("edge", &payload)
    }
}

/// Public interface snapshot awaiting independent reader validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectInterface {
    /// Locked public vocabulary facts in canonical identity order.
    vocabularies: Vec<ProjectPublicVocabulary>,
    /// Public exports in canonical module-symbol order.
    exports: Vec<ProjectPublicExport>,
    /// Public-to-public dependency edges in canonical order.
    edges: Vec<ProjectPublicEdge>,
    /// Declared public signature fingerprint.
    fingerprint: SemanticDigest,
}

impl ProjectInterface {
    /// Constructs a snapshot with a separately supplied declared fingerprint.
    #[must_use]
    pub const fn from_parts(
        exports: Vec<ProjectPublicExport>,
        edges: Vec<ProjectPublicEdge>,
        fingerprint: SemanticDigest,
    ) -> Self {
        Self {
            vocabularies: Vec::new(),
            exports,
            edges,
            fingerprint,
        }
    }

    /// Constructs a complete snapshot with a separately supplied fingerprint.
    #[must_use]
    pub const fn from_parts_with_vocabularies(
        vocabularies: Vec<ProjectPublicVocabulary>,
        exports: Vec<ProjectPublicExport>,
        edges: Vec<ProjectPublicEdge>,
        fingerprint: SemanticDigest,
    ) -> Self {
        Self {
            vocabularies,
            exports,
            edges,
            fingerprint,
        }
    }

    /// Constructs a snapshot with its canonical public-interface fingerprint.
    ///
    /// # Errors
    ///
    /// Returns transcript framing failures for unrepresentable payloads.
    pub fn with_computed_fingerprint(
        exports: Vec<ProjectPublicExport>,
        edges: Vec<ProjectPublicEdge>,
    ) -> Result<Self, CoreError> {
        Self::with_vocabularies(Vec::new(), exports, edges)
    }

    /// Constructs a complete public snapshot with canonical locked vocabularies.
    ///
    /// # Errors
    ///
    /// Returns transcript framing failures for unrepresentable payloads.
    pub fn with_vocabularies(
        vocabularies: Vec<ProjectPublicVocabulary>,
        exports: Vec<ProjectPublicExport>,
        edges: Vec<ProjectPublicEdge>,
    ) -> Result<Self, CoreError> {
        let fingerprint = Self::fingerprint_for(&vocabularies, &exports, &edges)?;
        Ok(Self {
            vocabularies,
            exports,
            edges,
            fingerprint,
        })
    }

    /// Returns canonical locked vocabulary facts without aliases or metadata.
    #[must_use]
    pub fn vocabularies(&self) -> &[ProjectPublicVocabulary] {
        &self.vocabularies
    }

    /// Returns public exports in canonical order.
    #[must_use]
    pub fn exports(&self) -> &[ProjectPublicExport] {
        &self.exports
    }

    /// Returns only public-to-public edges, with private provenance omitted.
    #[must_use]
    pub fn edges(&self) -> &[ProjectPublicEdge] {
        &self.edges
    }

    /// Returns the declared public signature fingerprint.
    #[must_use]
    pub const fn fingerprint(&self) -> SemanticDigest {
        self.fingerprint
    }

    /// Recomputes the fingerprint from the complete public signature surface.
    ///
    /// # Errors
    ///
    /// Returns transcript framing failures for unrepresentable payloads.
    pub fn recompute_fingerprint(&self) -> Result<SemanticDigest, CoreError> {
        Self::fingerprint_for(&self.vocabularies, &self.exports, &self.edges)
    }

    /// Hashes exports and public edges with one domain-separated transcript.
    fn fingerprint_for(
        vocabularies: &[ProjectPublicVocabulary],
        exports: &[ProjectPublicExport],
        edges: &[ProjectPublicEdge],
    ) -> Result<SemanticDigest, CoreError> {
        let mut payload = nht_frame("profile", V1_SOURCE_PROFILE.as_bytes())?;
        for vocabulary in vocabularies {
            payload.extend(vocabulary.transcript()?);
        }
        for export in exports {
            payload.extend(export.transcript()?);
        }
        for edge in edges {
            payload.extend(edge.transcript()?);
        }
        SemanticDigest::from_nht(PROJECT_INTERFACE_FINGERPRINT_DOMAIN, &payload)
    }
}

/// Frames one full module-symbol identity with the exact profile.
fn identity_transcript(identity: &ModuleSymbolIdentity) -> Result<Vec<u8>, CoreError> {
    let mut payload = nht_frame(
        "profile",
        identity.module().language_behavior_version().as_bytes(),
    )?;
    payload.extend(nht_frame(
        "module",
        identity.module().module_name().as_bytes(),
    )?);
    payload.extend(nht_frame(
        "declaration",
        identity.declaration_name().as_bytes(),
    )?);
    Ok(payload)
}
