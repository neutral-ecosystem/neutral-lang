// SPDX-License-Identifier: Apache-2.0

//! Shared source and resource companions used by current expression projects.
//!
//! Modules describe the complete logical import graph, including disconnected
//! private units. Source and vocabulary records describe exact captured bytes;
//! source maps and provenance connect declaration identities to original occurrences.
//! These companions retain evidence separately from normalized logical meaning.
//!
//! All public fields are raw claims until checked with the complete project.
//! Digests must match captured facts, spans must fit their owning source, and
//! resource counts must be independently recomputed. Logical source IDs are opaque
//! labels, not paths that instruct a reader to acquire missing content.
//! [`ProjectLimits`] carries independent policy dimensions rather than counters;
//! intersection cannot loosen either the producer or consumer's bound.

use crate::{
    LogicalModuleIdentity, ModuleSymbolIdentity, project_interface::ProjectPublicEdgeKind,
};
use neutral_core::{SourceContentDigest, SourceLocation, VocabularyContentDigest};

/// Hard recursion ceiling shared by producers and independent readers.
pub const PROJECT_MAX_DEPTH: usize = crate::project_interface::MAX_PROJECT_INTERFACE_TYPE_DEPTH;

/// One logical module, including disconnected and private-only units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectModule {
    /// Exact logical module identity.
    pub identity: LogicalModuleIdentity,
    /// Distinct imported logical module IDs in canonical order, without aliases.
    pub imports: Vec<String>,
}

/// Exact source accounting, separate from project meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSource {
    /// Owning logical module ID.
    pub module: String,
    /// Opaque logical source ID, never a path.
    pub source_id: String,
    /// Digest of original captured bytes.
    pub digest: SourceContentDigest,
    /// Original byte length used to check spans independently.
    pub byte_len: u64,
}

/// Declaration-level original-byte source-map companion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSourceMap {
    /// Exact declaration identity.
    pub declaration: ModuleSymbolIdentity,
    /// Complete declaration span in its captured source.
    pub location: SourceLocation,
}

/// An occurrence-sensitive semantic edge retained as private provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectProvenance {
    /// Declaration consuming the dependency.
    pub from: ModuleSymbolIdentity,
    /// Declaration providing the dependency.
    pub to: ModuleSymbolIdentity,
    /// Typed dependency category.
    pub kind: ProjectPublicEdgeKind,
    /// Original-byte dependency occurrence, independent of identity.
    pub location: SourceLocation,
}

/// Recomputable resource accounting for complete publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectResourceFacts {
    /// Complete supplied source units.
    pub source_units: u64,
    /// Sum of exact original source byte lengths.
    pub source_bytes: u64,
    /// Complete exact vocabulary lock count.
    pub vocabulary_units: u64,
    /// Sum of exact captured vocabulary bytes.
    pub vocabulary_bytes: u64,
    /// Complete root declaration count, including private roots.
    pub declarations: u64,
    /// Alias-independent logical import edge count.
    pub import_edges: u64,
    /// Materialized value nodes, including unused closed defaults.
    pub value_nodes: u64,
}

/// Explicit immutable project validation bounds, never ambient defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectLimits {
    /// Maximum source/module count.
    pub modules: u64,
    /// Maximum root declarations.
    pub declarations: u64,
    /// Maximum logical import edges.
    pub import_edges: u64,
    /// Maximum materialized nodes and provenance occurrences.
    pub nodes: u64,
    /// Maximum aggregate value/source bytes and individual schema-name bytes.
    pub text_bytes: u64,
    /// Complete encoded artifact bytes, independent of materialized value work.
    pub artifact_bytes: u64,
}

impl ProjectLimits {
    /// Intersects producer context with independent consumer bounds; neither can relax the other.
    /// Zero remains zero for the validation boundary to reject. This does not
    /// validate positivity, enforce hard ceilings or count current resource usage.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            modules: self.modules.min(other.modules),
            declarations: self.declarations.min(other.declarations),
            import_edges: self.import_edges.min(other.import_edges),
            nodes: self.nodes.min(other.nodes),
            text_bytes: self.text_bytes.min(other.text_bytes),
            artifact_bytes: self.artifact_bytes.min(other.artifact_bytes),
        }
    }
}

/// Exact locked vocabulary derivation companion, never authoring metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVocabularySource {
    /// Canonical vocabulary identity.
    pub identity: String,
    /// Exact semantic revision.
    pub version: String,
    /// Digest of exact captured bundle bytes.
    pub digest: VocabularyContentDigest,
    /// Captured bundle byte count.
    pub byte_len: u64,
}
