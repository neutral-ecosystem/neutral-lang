// SPDX-License-Identifier: Apache-2.0

//! Frozen expression selectors and independent controls; recognition is not activation.

/// Pure typed expression contract.
pub const EXPRESSION_PROFILE: &str = "neutral.expression/1";
/// Explicit pure evaluation contract.
pub const EVALUATOR_PROFILE: &str = "neutral.evaluator/1";
/// Compile-time finite expansion contract.
pub const EXPANSION_PROFILE: &str = "neutral.expansion/1";
/// Fixed decimal rounding semantics; never a host floating-point mode.
pub const DECIMAL_PROFILE: &str = "neutral.decimal/1";
/// Consumer/1's independent typed selector path ceiling.
pub const MAX_SELECTOR_SEGMENTS: usize = 64;
/// Frozen aggregate retained-item ceiling shared by expression and expansion controls.
pub const MAX_ITEMS: u64 = 1_000_000;
/// Frozen structural/evaluation/expansion depth ceiling; leaves start at one.
pub const MAX_DEPTH: usize = 64;
/// Frozen request work ceiling shared by expression and expansion controls.
pub const MAX_WORK: u64 = 100_000_000;
/// Frozen owned model/output byte ceiling, excluding allocator metadata.
pub const MAX_MODEL_BYTES: u64 = 67_108_864;

/// Exact expression capture envelope, never inferred from source spelling.
pub const CAPTURE_REQUEST_VERSION: &str = "neutral.capture/v3";
/// Complete expression artifact schema.
pub const PROJECT_IR_SCHEMA: &str = "neutral.project-ir/3";
/// Expression public projection schema.
pub const PROJECT_VIEW_SCHEMA: &str = "neutral.project-view/3";
/// Expression whole-request result schema.
pub const PROJECT_RESULT_SCHEMA: &str = "neutral.project-result/3";
/// Expression identity profile, distinct from historical transcripts.
pub const IDENTITY_PROFILE: &str = "neutral.project-identity/3";
/// Complete restricted-CBOR expression artifact encoding.
pub const ENCODING: &str = "NIR-PROJECT-CBOR/3";
/// Distinct magic; recognition alone never grants codec availability.
pub const MAGIC: [u8; 8] = *b"NEUP3\r\n\x1a";
/// Exact sorted successor feature set, including finite expansion.
pub const REQUIRED_FEATURES: &[&str] = &[
    "decimal-division-v1",
    "finite-expansion-v1",
    "pure-evaluator-v1",
    "pure-expressions-v1",
    crate::composition::profile::TAGGED_VARIANTS_FEATURE,
    crate::composition::profile::VOCABULARY_COMPOSITION_FEATURE,
];

/// Raw exact semantic tuple; package versions and optional-feature fallback cannot select it.
#[derive(Debug, Eq, PartialEq)]
pub struct SemanticProfiles {
    /// Typed expression contract.
    pub expression: String,
    /// Pure evaluator contract.
    pub evaluator: String,
    /// Finite compile-time expansion contract.
    pub expansion: String,
    /// Fixed decimal arithmetic context.
    pub decimal: String,
    /// Canonical complete required feature set.
    pub required_features: Vec<String>,
}
impl SemanticProfiles {
    /// Creates the exact frozen tuple fallibly, without claiming implementation availability.
    ///
    /// # Errors
    /// Returns allocation failure without a partial tuple.
    pub fn try_frozen() -> Result<Self, neutral_core::allocation::AllocationError> {
        use neutral_core::allocation::{RetainCapacity, text};
        let mut required_features = Vec::new();
        required_features.try_retain_exact(REQUIRED_FEATURES.len())?;
        for feature in REQUIRED_FEATURES {
            required_features.push(text(feature)?);
        }
        Ok(Self {
            expression: text(EXPRESSION_PROFILE)?,
            evaluator: text(EVALUATOR_PROFILE)?,
            expansion: text(EXPANSION_PROFILE)?,
            decimal: text(DECIMAL_PROFILE)?,
            required_features,
        })
    }
    /// Checks exact selectors and the complete, sorted required feature list.
    ///
    /// Missing, extra, reordered and unknown selectors reject without fallback.
    /// A match is recognition only, not evidence that source, wire or evaluator
    /// boundaries implement this tuple.
    #[must_use]
    pub fn matches_frozen(&self) -> bool {
        self.expression == EXPRESSION_PROFILE
            && self.evaluator == EVALUATOR_PROFILE
            && self.expansion == EXPANSION_PROFILE
            && self.decimal == DECIMAL_PROFILE
            && self.required_features == REQUIRED_FEATURES
    }
}

/// Positive controls in frozen expression-profile tuple order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpressionLimits {
    /// Retained expression occurrences, including skipped branches.
    pub nodes: u64,
    /// Source/tree and nested structural depth.
    pub syntax_depth: u64,
    /// Retained evaluation tree depth, leaf one.
    pub evaluation_depth: u64,
    /// Complete binding registry; enforced by project/evaluator integration.
    pub bindings: u64,
    /// Potential input/output occurrence edges.
    pub dependency_edges: u64,
    /// Traversal, comparison and copying work.
    pub work: u64,
    /// Owned structural data, excluding allocator metadata.
    pub retained_bytes: u64,
    /// Owned result bytes; never interchangeable with transport bytes.
    pub output_bytes: u64,
    /// Decimal coefficient digits.
    pub coefficient_digits: u64,
    /// Absolute decimal exponent.
    pub absolute_scale: u64,
    /// Complete diagnostic collection; enforced by project/evaluator integration.
    pub diagnostics: u64,
}
impl ExpressionLimits {
    /// Returns frozen acceptance ceilings, not a recommended per-request budget.
    ///
    /// Intersect with caller policy before traversing raw input. Returning these
    /// ceilings does not advertise implementation availability or validate positivity.
    #[must_use]
    pub const fn hard() -> Self {
        Self {
            nodes: MAX_ITEMS,
            syntax_depth: MAX_DEPTH as u64,
            evaluation_depth: MAX_DEPTH as u64,
            bindings: MAX_ITEMS,
            dependency_edges: MAX_ITEMS,
            work: MAX_WORK,
            retained_bytes: MAX_MODEL_BYTES,
            output_bytes: MAX_MODEL_BYTES,
            coefficient_digits: 4096,
            absolute_scale: 4096,
            diagnostics: MAX_ITEMS,
        }
    }
    /// Projects controls into the frozen eleven-position tuple.
    #[must_use]
    pub const fn values(self) -> [u64; 11] {
        [
            self.nodes,
            self.syntax_depth,
            self.evaluation_depth,
            self.bindings,
            self.dependency_edges,
            self.work,
            self.retained_bytes,
            self.output_bytes,
            self.coefficient_digits,
            self.absolute_scale,
            self.diagnostics,
        ]
    }
    /// Intersects each dimension separately without borrowing unused budgets.
    ///
    /// Zero remains zero so validation can reject an invalid policy; this method
    /// neither replaces omitted controls with defaults nor silently raises limits.
    ///
    /// ```
    /// use neutral_ir::expression::profile::ExpressionLimits;
    /// let caller = ExpressionLimits { nodes: 2, work: 30, ..ExpressionLimits::hard() };
    /// let producer = ExpressionLimits { nodes: 9, work: 10, ..ExpressionLimits::hard() };
    /// let effective = caller.intersect(producer);
    /// assert_eq!(effective.nodes, 2);
    /// assert_eq!(effective.work, 10);
    /// ```
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            nodes: self.nodes.min(other.nodes),
            syntax_depth: self.syntax_depth.min(other.syntax_depth),
            evaluation_depth: self.evaluation_depth.min(other.evaluation_depth),
            bindings: self.bindings.min(other.bindings),
            dependency_edges: self.dependency_edges.min(other.dependency_edges),
            work: self.work.min(other.work),
            retained_bytes: self.retained_bytes.min(other.retained_bytes),
            output_bytes: self.output_bytes.min(other.output_bytes),
            coefficient_digits: self.coefficient_digits.min(other.coefficient_digits),
            absolute_scale: self.absolute_scale.min(other.absolute_scale),
            diagnostics: self.diagnostics.min(other.diagnostics),
        }
    }
}
impl Default for ExpressionLimits {
    /// Uses frozen hard ceilings; consumers should supply a tighter request policy.
    fn default() -> Self {
        Self::hard()
    }
}
