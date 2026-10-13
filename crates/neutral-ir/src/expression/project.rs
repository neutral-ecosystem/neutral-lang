// SPDX-License-Identifier: Apache-2.0

//! Complete raw /3 producer data; neither a materialized /2 project nor reader authority.

use super::{
    ExpressionBody, ExpressionShapeError,
    attribution::{ExpansionAttribution, ExpressionAttribution},
    profile::{ExpressionLimits, SemanticProfiles},
    templates::{ExpansionLimits, PublicTemplate},
};
use crate::{
    ModuleSymbolIdentity,
    composition::{
        CompositionBundle,
        project::{
            CompositionOrigin, CompositionPolicy, CompositionResourceFacts, CompositionSignature,
        },
    },
    project::{
        ProjectLimits, ProjectModule, ProjectProvenance, ProjectResourceFacts, ProjectSource,
        ProjectSourceMap, ProjectVocabularySource,
    },
};
use neutral_core::SemanticDigest;

/// A complete declaration retains absence for definitions and a tagged body for bindings.
#[derive(Debug, Eq, PartialEq)]
pub struct ExpressionDeclaration {
    /// Exact stable source owner, never an alias or host path.
    pub identity: ModuleSymbolIdentity,
    /// Explicit public visibility.
    pub public: bool,
    /// Shared exact type or nominal record/variant contract.
    pub signature: CompositionSignature,
    /// Constant, external input or residual expression; absent only for definitions.
    pub body: Option<ExpressionBody>,
}
impl ExpressionDeclaration {
    /// Checks only the definition/binding body sum, not typing, visibility or project validity.
    ///
    /// # Errors
    /// Rejects fabricated definition bodies and missing binding bodies.
    pub fn validate_body_shape(&self) -> Result<(), ExpressionShapeError> {
        match (&self.signature, &self.body) {
            (CompositionSignature::Binding(_), Some(_))
            | (CompositionSignature::Definition(_), None) => Ok(()),
            _ => Err(ExpressionShapeError::Malformed),
        }
    }
}

/// Independently recomputable retained facts in frozen seven-position order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExpressionResourceFacts {
    /// Occurrences in residual and public template trees.
    pub residual_nodes: u64,
    /// Maximum retained expression depth, leaf one.
    pub residual_depth: u64,
    /// Declared external inputs, not supplied runtime values.
    pub input_declarations: u64,
    /// Retained potential output occurrences.
    pub output_leaves: u64,
    /// Potential input/output value-edge occurrences.
    pub potential_value_edges: u64,
    /// Exported non-callable template contracts.
    pub public_templates: u64,
    /// UTF-8 text in canonical wire expression trees, including kind/type/operator tags.
    pub retained_expression_text_bytes: u64,
}
impl ExpressionResourceFacts {
    /// Projects claimed counts without validating their truth.
    #[must_use]
    pub const fn values(self) -> [u64; 7] {
        [
            self.residual_nodes,
            self.residual_depth,
            self.input_declarations,
            self.output_leaves,
            self.potential_value_edges,
            self.public_templates,
            self.retained_expression_text_bytes,
        ]
    }
}

/// Complete raw data for the fixed 22-position /3 transport, with unchanged companion roles.
#[derive(Debug, Eq, PartialEq)]
pub struct ExpressionProjectIr {
    /// Exact complete schema selector.
    pub schema: String,
    /// Complete modules/imports, including disconnected private units.
    pub modules: Vec<ProjectModule>,
    /// Canonically ordered complete declarations, including external inputs.
    pub declarations: Vec<ExpressionDeclaration>,
    /// Complete source-independent vocabulary contracts and dependencies.
    pub vocabularies: Vec<CompositionBundle>,
    /// Claimed public interface /3 digest; independently recomputed by the reader.
    pub interface_digest: SemanticDigest,
    /// Original source accounting, never host acquisition instructions.
    pub sources: Vec<ProjectSource>,
    /// Checked original-byte declaration occurrences.
    pub source_maps: Vec<ProjectSourceMap>,
    /// Structural type/value/reference evidence; not guarded output dependency authority.
    pub provenance: Vec<ProjectProvenance>,
    /// Retained project policy, separate from logical meaning.
    pub limits: ProjectLimits,
    /// Claimed base retained project facts.
    pub resources: ProjectResourceFacts,
    /// Original captured vocabulary accounting.
    pub vocabulary_sources: Vec<ProjectVocabularySource>,
    /// Supplied/default/reuse occurrence evidence for materialized values.
    pub origins: Vec<CompositionOrigin>,
    /// Frozen composition policy.
    pub composition_limits: CompositionPolicy,
    /// Claimed retained composition facts.
    pub composition_resources: CompositionResourceFacts,
    /// Exact semantic profile tuple and features.
    pub semantic_profiles: SemanticProfiles,
    /// Canonical exported template contracts, never runtime callable values.
    pub public_templates: Vec<PublicTemplate>,
    /// Independent expression policy in frozen tuple order.
    pub expression_limits: ExpressionLimits,
    /// Independent expansion policy in frozen tuple order.
    pub expansion_limits: ExpansionLimits,
    /// Claimed recomputable successor facts, never execution counts.
    pub expression_resources: ExpressionResourceFacts,
    /// Checked occurrence paths for residual/public-template nodes.
    pub expression_attributions: Vec<ExpressionAttribution>,
    /// Original application/definition/argument/axis links, not fabricated generated source.
    pub expansion_attributions: Vec<ExpansionAttribution>,
}
