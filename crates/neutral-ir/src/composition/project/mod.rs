// SPDX-License-Identifier: Apache-2.0

//! Shared retained-data policy and preflight helpers for current expression projects.
//!
//! [`CompositionSignature`] lets source nominal declarations and vocabulary
//! definitions share one body model. [`CompositionPolicy`] keeps independent
//! catalogue, dependency, type, default/value and work controls in frozen tuple
//! order. [`CompositionResourceFacts`] describes retained structure, not the
//! amount of compilation or consumer execution that actually took place.
//!
//! Origin companions connect a consuming value path to supplied/null/omitted/default
//! evidence. Attribution can refer to original source or a canonical vocabulary
//! field owner, or explicitly be absent/redacted. It never authenticates a producer
//! or instructs a reader to fetch a file/bundle. Raw tuples and evidence need full
//! independent validation; the reexported preflight functions only bound inspection.

use super::{CompositionBody, ValueOriginKind, ValuePathSegment};
use crate::ModuleSymbolIdentity;
use neutral_core::{ByteSpan, SourceLocation};
mod inspection;
pub use inspection::{
    CompositionBodyInspection, CompositionShapeError, inspect_composition_bodies,
    inspect_composition_catalogue, inspect_expression_project_resources,
    inspect_expression_project_resources_with_work,
};

/// Independent composition policy in frozen wire/derivation order, excluding JSON parser controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionPolicy {
    /// Bundle count.
    pub bundles: u64,
    /// Aggregate captured vocabulary bytes.
    pub captured_bytes: u64,
    /// Direct dependencies per bundle.
    pub dependencies_per_bundle: u64,
    /// Aggregate dependency edges.
    pub dependency_edges: u64,
    /// Longest bundle path, including root and leaf.
    pub dependency_depth: u64,
    /// Source and vocabulary nominal definitions.
    pub total_types: u64,
    /// Aggregate record fields.
    pub total_fields: u64,
    /// Alternatives in one variant.
    pub alternatives_per_type: u64,
    /// Aggregate alternatives.
    pub total_alternatives: u64,
    /// Choices in one field.
    pub choices_per_field: u64,
    /// Aggregate choices.
    pub total_choices: u64,
    /// Composed wrapper depth, root zero.
    pub type_depth: u64,
    /// Value occurrence depth, root zero.
    pub value_depth: u64,
    /// Cumulative materialization visits.
    pub value_nodes: u64,
    /// Semantic traversal and comparison work.
    pub work: u64,
}

impl CompositionPolicy {
    /// Projects explicit named controls into the immutable fifteen-position contract.
    #[must_use]
    pub const fn values(self) -> [u64; 15] {
        [
            self.bundles,
            self.captured_bytes,
            self.dependencies_per_bundle,
            self.dependency_edges,
            self.dependency_depth,
            self.total_types,
            self.total_fields,
            self.alternatives_per_type,
            self.total_alternatives,
            self.choices_per_field,
            self.total_choices,
            self.type_depth,
            self.value_depth,
            self.value_nodes,
            self.work,
        ]
    }

    /// Reads an already shape-checked contract tuple; positive/hard bounds remain a validator's job.
    #[must_use]
    pub const fn from_values(v: [u64; 15]) -> Self {
        Self {
            bundles: v[0],
            captured_bytes: v[1],
            dependencies_per_bundle: v[2],
            dependency_edges: v[3],
            dependency_depth: v[4],
            total_types: v[5],
            total_fields: v[6],
            alternatives_per_type: v[7],
            total_alternatives: v[8],
            choices_per_field: v[9],
            total_choices: v[10],
            type_depth: v[11],
            value_depth: v[12],
            value_nodes: v[13],
            work: v[14],
        }
    }

    /// Intersects producer and independent consumer policies without relaxing either.
    /// Keeps zero controls invalid and preserves tuple positions; it does not
    /// redistribute unused budgets or validate the resulting policy.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let left = self.values();
        let right = other.values();
        // Use the same frozen projection in both directions so every named control is intersected.
        Self::from_values(std::array::from_fn(|i| left[i].min(right[i])))
    }
}

/// Shared exact binding/nominal-definition signature for current expression declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionSignature {
    /// Immutable binding's invariant expected type.
    Binding(crate::project_interface::ProjectPublicType),
    /// Source record/variant using the exact shared vocabulary body model.
    Definition(CompositionBody),
}

/// Attribution evidence, not authentication or a host acquisition hint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionAttribution {
    /// Checked original source bytes, distinct from a host filesystem path.
    Source(SourceLocation),
    /// Exact vocabulary contract owner with an optional checked bundle-byte span.
    Vocabulary {
        /// Canonical bundle identity.
        identity: String,
        /// Exact semantic revision.
        version: String,
        /// Nominal contract name.
        type_name: String,
        /// Field whose closed default supplied this occurrence.
        field_name: String,
        /// Optional original-byte bundle occurrence; absence is explicit.
        span: Option<ByteSpan>,
    },
}

/// One non-semantic materialized occurrence companion, including optional absence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionOrigin {
    /// Consuming binding identity.
    pub binding: ModuleSymbolIdentity,
    /// Canonical field/list/selected-payload occurrence path.
    pub path: Vec<ValuePathSegment>,
    /// Supplied/null/omitted/default classification.
    pub kind: ValueOriginKind,
    /// Checked evidence or explicit unavailable/redacted attribution.
    pub attribution: Option<CompositionAttribution>,
}

/// Independently recomputable retained contract/value facts, never execution-work observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionResourceFacts {
    /// Exact canonical bundle count.
    pub bundles: u64,
    /// Direct dependency edges after diamond deduplication.
    pub dependency_edges: u64,
    /// All source/vocabulary definitions.
    pub types: u64,
    /// All record fields.
    pub fields: u64,
    /// All variant alternatives.
    pub alternatives: u64,
    /// All finite choices.
    pub choices: u64,
    /// All retained binding/default/choice value nodes, including optional absence.
    pub retained_value_nodes: u64,
}

impl CompositionResourceFacts {
    /// Projects retained facts in the immutable seven-position transport order.
    #[must_use]
    pub const fn values(self) -> [u64; 7] {
        [
            self.bundles,
            self.dependency_edges,
            self.types,
            self.fields,
            self.alternatives,
            self.choices,
            self.retained_value_nodes,
        ]
    }

    /// Reads an already shape-checked fact tuple; independent recomputation remains mandatory.
    #[must_use]
    pub const fn from_values(v: [u64; 7]) -> Self {
        Self {
            bundles: v[0],
            dependency_edges: v[1],
            types: v[2],
            fields: v[3],
            alternatives: v[4],
            choices: v[5],
            retained_value_nodes: v[6],
        }
    }
}
