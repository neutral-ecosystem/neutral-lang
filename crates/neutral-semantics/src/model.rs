// SPDX-License-Identifier: Apache-2.0

//! Opaque semantic-model authority and derived obligations, not complete artifact acceptance.

use neutral_core::allocation::Shared;
use neutral_ir::{
    ModuleSymbolIdentity,
    expression::{
        profile::ExpressionLimits,
        project::{ExpressionProjectIr, ExpressionResourceFacts},
        templates::ExpansionLimits,
    },
    project::ProjectLimits,
    project_interface::ProjectPublicType,
};
use neutral_vocabulary::composition::{CompositionLimits, ValidatedCompositionScope};

/// Caller controls for semantic sections; full reader companions have their own validation.
#[derive(Clone, Copy, Debug)]
pub struct SemanticLimits {
    /// Complete project counts/text policy.
    pub project: ProjectLimits,
    /// Nominal/default/restriction policy.
    pub composition: CompositionLimits,
    /// Aggregate residual-tree policy.
    pub expression: ExpressionLimits,
    /// Exported template and finite-expansion policy.
    pub expansion: ExpansionLimits,
}

/// Stable safe expression failures; no untrusted text or private identity is retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionValidationError {
    /// Unknown/unavailable exact semantic tuple.
    Unsupported,
    /// Malformed declaration, owner, ordering or envelope.
    Malformed,
    /// Missing/private nominal owner or referenced binding.
    UnknownOrPrivate,
    /// Annotation, operand, result or lexical frame mismatch.
    TypeMismatch,
    /// Missing/duplicate/unknown exact match alternative.
    MatchAlternatives,
    /// Actual ordinary value cycle.
    ValueCycle,
    /// Invalid restriction or selector contract.
    Restriction,
    /// Recomputed retained facts differ from producer claims.
    Resources,
    /// Independent count/depth/work/storage/allocation failure.
    Limit,
    /// Cooperative cancellation before publication.
    Cancelled,
}
impl ExpressionValidationError {
    /// Maps semantic classifications to the frozen expression diagnostic registry.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "NEU-EXP-001",
            Self::Malformed | Self::Resources => "NEU-EXP-002",
            Self::UnknownOrPrivate => "NEU-EXP-003",
            Self::TypeMismatch => "NEU-EXP-004",
            Self::MatchAlternatives => "NEU-EXP-005",
            Self::ValueCycle => "NEU-EXP-007",
            Self::Restriction => "NEU-EXP-008",
            Self::Limit => "NEU-EXP-009",
            Self::Cancelled => "NEU-EXP-010",
        }
    }
}

/// Exact retained root within one owner, distinct from its tree's child-index path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionRoot {
    /// Ordinary binding residual.
    Binding,
    /// Exported template body.
    TemplateBody,
    /// Default of the zero-based declared template parameter.
    TemplateDefault(usize),
}

/// Canonical retained expression site, separate from source attribution.
#[derive(Debug, Eq, PartialEq)]
pub struct ExpressionSite {
    /// Exact declaration/template owner.
    pub owner: ModuleSymbolIdentity,
    /// Separates template defaults from their body without inventing wire child indices.
    pub root: ExpressionRoot,
    /// Canonical child-index path.
    pub node_path: Vec<u64>,
}
/// Availability checks that typing alone cannot prove for a future value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AvailabilityRequirement {
    /// Future optional field must be present.
    Presence,
    /// Future list must contain the selected index.
    Index,
    /// Future variant must select the requested alternative.
    Alternative,
}
/// One unresolved selector condition; interpretation uses its retained exact segment.
#[derive(Debug, Eq, PartialEq)]
pub struct AvailabilityObligation {
    /// Actual validated output-expression site.
    pub site: ExpressionSite,
    /// Zero-based segment in that output's retained selector.
    pub segment: usize,
    /// Required future value availability.
    pub requirement: AvailabilityRequirement,
}
/// A real nominal-field restriction that must hold when its expression materializes.
#[derive(Debug, Eq, PartialEq)]
pub struct RestrictionObligation {
    /// Canonical value-expression site, not a caller-supplied obligation.
    pub site: ExpressionSite,
    /// Exact source/vocabulary nominal contract containing the field.
    pub contract: ProjectPublicType,
    /// Canonical constrained field name.
    pub field: String,
}

/// Independently checked semantic sections. Source/companion/identity validity is not implied.
pub struct ValidatedExpressionModel {
    /// Immutable producer data; semantic bodies have been independently retyped.
    pub(crate) ir: Shared<ExpressionProjectIr>,
    /// Independently validated complete nominal contracts/defaults/restrictions.
    pub(crate) scope: Shared<ValidatedCompositionScope>,
    /// Intersected acceptance controls retained for future explicit evaluation.
    pub(crate) limits: SemanticLimits,
    /// Independently recomputed successor facts.
    pub(crate) facts: ExpressionResourceFacts,
    /// Derived future-selector checks; never a scheduling graph.
    pub(crate) availability: Vec<AvailabilityObligation>,
    /// Derived unresolved field restriction checks.
    pub(crate) restrictions: Vec<RestrictionObligation>,
}
impl ValidatedExpressionModel {
    /// Borrows complete semantic data; public readers must filter private content.
    #[must_use]
    pub fn ir(&self) -> &ExpressionProjectIr {
        &self.ir
    }
    /// Borrows complete validated contract scope; not a public-only view.
    #[must_use]
    pub fn scope(&self) -> &ValidatedCompositionScope {
        &self.scope
    }
    /// Returns intersected acceptance controls without relaxing producer policy.
    #[must_use]
    pub const fn limits(&self) -> SemanticLimits {
        self.limits
    }
    /// Returns independently recomputed retained expression facts.
    #[must_use]
    pub const fn facts(&self) -> ExpressionResourceFacts {
        self.facts
    }
    /// Returns all derived future selector availability checks.
    #[must_use]
    pub fn availability_obligations(&self) -> &[AvailabilityObligation] {
        &self.availability
    }
    /// Returns all retained nominal-field restriction obligations.
    #[must_use]
    pub fn restriction_obligations(&self) -> &[RestrictionObligation] {
        &self.restrictions
    }
}
impl std::fmt::Debug for ValidatedExpressionModel {
    /// Counts only; semantic acceptance cannot leak private owners through debug text.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedExpressionModel")
            .field("declarations", &self.ir.declarations.len())
            .field("availability", &self.availability.len())
            .field("restrictions", &self.restrictions.len())
            .finish_non_exhaustive()
    }
}
