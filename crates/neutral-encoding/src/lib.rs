// SPDX-License-Identifier: Apache-2.0

//! External encoding and hostile decoding of validated Neutral documents.
//!
//! This codec implements Neutral IR Framed CBOR 0.1. The encoder accepts only
//! trusted reader views; the bounds-first decoder returns one only after full
//! validation. Deterministic emitted field order is an implementation property:
//! encoded bytes are not logical identity, and producer/build facts remain
//! confined to the envelope.

mod cbor;
/// Explicit successor composition transport, distinct from frozen project /1.
pub mod composition;
pub mod constants;
mod decode;
mod decoder;
pub mod diagnostics;
/// Complete project transport and independent bounded decoding.
pub mod project;

mod encoder;
use encoder::capabilities::{TraversalBudget, capability, derive_capabilities};
pub use encoder::encode;
use std::ops::Range;

pub use decode::decode;

/// Complete bounded result class for hostile external artifact decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeErrorClass {
    /// An artifact, section, string, container, depth, or traversal limit failed.
    EncodedSizeLimit,
    /// Fixed framing or directory structure was malformed.
    MalformedFrame,
    /// A fixed framing, section, or contract version is unsupported.
    UnsupportedVersion,
    /// Capability declarations are unsupported or inconsistent with content.
    UnsupportedCapability,
    /// Exact section integrity verification failed.
    IntegrityMismatch,
    /// Restricted-CBOR lexical/container validation failed.
    MalformedCbor,
    /// A closed encoded schema was malformed.
    InvalidEncodedSchema,
    /// Logical IR validation failed.
    InvalidLogicalIr,
    /// Source-map validation failed.
    InvalidSourceMap,
    /// Provenance validation failed.
    InvalidProvenance,
    /// Derivation or cross-section validation failed.
    InvalidDerivation,
    /// Cooperative caller cancellation was observed.
    Cancelled,
    /// An implementation invariant failed independently of input validity.
    InternalDefect,
}

impl DecodeErrorClass {
    /// Returns the stable bounded diagnostic code for this result class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EncodedSizeLimit => diagnostics::ENCODED_SIZE_LIMIT,
            Self::MalformedFrame => diagnostics::MALFORMED_FRAME,
            Self::UnsupportedVersion => diagnostics::UNSUPPORTED_VERSION,
            Self::UnsupportedCapability => diagnostics::UNSUPPORTED_CAPABILITY,
            Self::IntegrityMismatch => diagnostics::INTEGRITY_MISMATCH,
            Self::MalformedCbor => diagnostics::MALFORMED_CBOR,
            Self::InvalidEncodedSchema => diagnostics::INVALID_ENCODED_SCHEMA,
            Self::InvalidLogicalIr => diagnostics::INVALID_LOGICAL_IR,
            Self::InvalidSourceMap => diagnostics::INVALID_SOURCE_MAP,
            Self::InvalidProvenance => diagnostics::INVALID_PROVENANCE,
            Self::InvalidDerivation => diagnostics::INVALID_DERIVATION,
            Self::Cancelled => diagnostics::CANCELLED,
            Self::InternalDefect => diagnostics::INTERNAL_DEFECT,
        }
    }
}

/// Bounded safe decoder failure with an optional encoded-byte offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeError {
    /// Stable failure class.
    class: DecodeErrorClass,
    /// Absolute byte offset when one is available.
    offset: Option<u64>,
}

impl DecodeError {
    /// Constructs one bounded internal decoder error.
    const fn new(class: DecodeErrorClass, offset: Option<u64>) -> Self {
        Self { class, offset }
    }

    /// Returns the stable result class.
    #[must_use]
    pub const fn class(self) -> DecodeErrorClass {
        self.class
    }

    /// Returns the stable diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.class.code()
    }

    /// Returns the absolute encoded-byte offset when available.
    #[must_use]
    pub const fn offset(self) -> Option<u64> {
        self.offset
    }
}

/// Host-configurable decoder ceilings, always clamped to frozen hard limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeLimits {
    /// Complete artifact bytes.
    artifact_bytes: usize,
    /// Bytes in any one section.
    section_bytes: usize,
    /// Nested CBOR/logical containers.
    nesting_depth: usize,
    /// Items in one container.
    container_items: usize,
    /// Bytes in one text string.
    text_bytes: usize,
    /// Bytes in one byte string.
    byte_string_bytes: usize,
    /// Total traversed nodes.
    traversal_nodes: usize,
}

impl DecodeLimits {
    /// Returns the immutable frozen hard ceilings.
    #[must_use]
    pub const fn hard() -> Self {
        Self {
            artifact_bytes: constants::MAXIMUM_ARTIFACT_BYTES,
            section_bytes: constants::MAXIMUM_SECTION_BYTES,
            nesting_depth: constants::MAXIMUM_NESTING_DEPTH,
            container_items: constants::MAXIMUM_CONTAINER_ITEMS,
            text_bytes: constants::MAXIMUM_TEXT_BYTES,
            byte_string_bytes: constants::MAXIMUM_BYTE_STRING_BYTES,
            traversal_nodes: constants::MAXIMUM_TRAVERSAL_NODES,
        }
    }

    /// Applies a lower host artifact-byte ceiling; zero rejects every artifact.
    #[must_use]
    pub const fn with_artifact_bytes(mut self, value: usize) -> Self {
        self.artifact_bytes = min_usize(self.artifact_bytes, value);
        self
    }

    /// Applies a lower host section-byte ceiling.
    #[must_use]
    pub const fn with_section_bytes(mut self, value: usize) -> Self {
        self.section_bytes = min_usize(self.section_bytes, value);
        self
    }

    /// Applies a lower host nesting-depth ceiling.
    #[must_use]
    pub const fn with_nesting_depth(mut self, value: usize) -> Self {
        self.nesting_depth = min_usize(self.nesting_depth, value);
        self
    }

    /// Applies a lower host per-container item ceiling.
    #[must_use]
    pub const fn with_container_items(mut self, value: usize) -> Self {
        self.container_items = min_usize(self.container_items, value);
        self
    }

    /// Applies a lower host text-string byte ceiling.
    #[must_use]
    pub const fn with_text_bytes(mut self, value: usize) -> Self {
        self.text_bytes = min_usize(self.text_bytes, value);
        self
    }

    /// Applies a lower host byte-string ceiling.
    #[must_use]
    pub const fn with_byte_string_bytes(mut self, value: usize) -> Self {
        self.byte_string_bytes = min_usize(self.byte_string_bytes, value);
        self
    }

    /// Applies a lower host total traversal ceiling.
    #[must_use]
    pub const fn with_traversal_nodes(mut self, value: usize) -> Self {
        self.traversal_nodes = min_usize(self.traversal_nodes, value);
        self
    }

    /// Returns the effective artifact-byte ceiling.
    pub(crate) const fn maximum_artifact_bytes(self) -> usize {
        self.artifact_bytes
    }
    /// Returns the effective per-section byte ceiling.
    pub(crate) const fn maximum_section_bytes(self) -> usize {
        self.section_bytes
    }
    /// Returns the effective nesting-depth ceiling.
    pub(crate) const fn maximum_nesting_depth(self) -> usize {
        self.nesting_depth
    }
    /// Returns the effective per-container item ceiling.
    pub(crate) const fn maximum_container_items(self) -> usize {
        self.container_items
    }
    /// Returns the effective text-string byte ceiling.
    pub(crate) const fn maximum_text_bytes(self) -> usize {
        self.text_bytes
    }
    /// Returns the effective byte-string ceiling.
    pub(crate) const fn maximum_byte_string_bytes(self) -> usize {
        self.byte_string_bytes
    }
    /// Returns the effective traversal ceiling.
    pub(crate) const fn maximum_traversal_nodes(self) -> usize {
        self.traversal_nodes
    }
}

/// Returns the smaller of two `usize` values in constant contexts.
const fn min_usize(left: usize, right: usize) -> usize {
    if left < right { left } else { right }
}

/// Envelope-only facts describing the implementation that produced an artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerInfo {
    /// Producer name.
    name: String,
    /// Producer version.
    version: String,
    /// Optional producer build identity.
    build: Option<String>,
}

impl ProducerInfo {
    /// Creates producer facts without a build identity.
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            build: None,
        }
    }

    /// Attaches an envelope-only build identity.
    #[must_use]
    pub fn with_build(mut self, build: impl Into<String>) -> Self {
        self.build = Some(build.into());
        self
    }

    /// Returns the producer name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the producer version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the optional producer build identity.
    #[must_use]
    pub fn build(&self) -> Option<&str> {
        self.build.as_deref()
    }
}

/// One fixed external section kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SectionKind {
    /// Encoding and producer envelope.
    Envelope = 1,
    /// Authoritative logical payload.
    LogicalPayload = 2,
    /// Original-byte source map.
    SourceMap = 3,
    /// Complete provenance companions.
    Provenance = 4,
    /// Complete derivation companions.
    Derivation = 5,
}

impl SectionKind {
    /// Returns the zero-based section position in the fixed directory.
    const fn index(self) -> usize {
        self as usize - 1
    }
}

/// Complete immutable encoded Neutral artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedArtifact {
    /// Complete framed bytes.
    bytes: Vec<u8>,
    /// Exact byte range for each fixed section kind.
    sections: [Range<usize>; constants::SECTION_COUNT],
}

impl EncodedArtifact {
    /// Returns the complete framed artifact bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the exact restricted-CBOR bytes for one section.
    #[must_use]
    pub fn section_bytes(&self, kind: SectionKind) -> &[u8] {
        &self.bytes[self.sections[kind.index()].clone()]
    }

    /// Consumes the wrapper and returns the complete frame.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A bounded failure while projecting validated memory into external bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingError {
    /// An immutable external encoding ceiling was exceeded.
    EncodedSizeLimit,
    /// Cooperative cancellation prevented complete project encoding.
    Cancelled,
    /// Validated input contradicted an encoder-only structural invariant.
    InternalDefect,
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
