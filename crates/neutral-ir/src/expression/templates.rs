// SPDX-License-Identifier: Apache-2.0

//! Non-callable public template data and independent finite-expansion controls.

use super::{
    Expression,
    profile::{MAX_DEPTH, MAX_ITEMS, MAX_MODEL_BYTES, MAX_WORK},
};
use crate::{ModuleSymbolIdentity, project_interface::ProjectPublicType};

/// Whether an argument must be static or may retain deferred value data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterPhase {
    /// Required at compile-time expansion; cannot depend on consumer bindings.
    Static,
    /// May populate the body but cannot determine expansion topology.
    Deferred,
}
/// One parameter, ordered by declaration; names matter for named applications.
#[derive(Debug, Eq, PartialEq)]
pub struct TemplateParameter {
    /// Exact semantic named argument.
    pub name: String,
    /// Explicit phase constraint.
    pub phase: ParameterPhase,
    /// Exact type, not structural duck typing.
    pub ty: ProjectPublicType,
    /// Definition-scoped default, distinct from typed null.
    pub default: Option<Expression>,
}
/// Exported definition contract; applications disappear from ordinary semantic bindings.
#[derive(Debug, Eq, PartialEq)]
pub struct PublicTemplate {
    /// Exact stable public template owner.
    pub identity: ModuleSymbolIdentity,
    /// Ordered lexical parameter frame.
    pub parameters: Vec<TemplateParameter>,
    /// Exact result contract.
    pub result_type: ProjectPublicType,
    /// Definition-scoped typed body; not a runtime callable value.
    pub body: Expression,
}
/// Independent positive finite-expansion controls in frozen tuple order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpansionLimits {
    /// Template definitions.
    pub definitions: u64,
    /// Template applications.
    pub applications: u64,
    /// Parameters per application.
    pub parameters: u64,
    /// Axes per matrix.
    pub axes: u64,
    /// Items per axis.
    pub axis_items: u64,
    /// Full Cartesian tuples before filtering per matrix.
    pub product_tuples: u64,
    /// Aggregate tuples visited across the request.
    pub aggregate_tuples: u64,
    /// Aggregate generated bindings.
    pub expanded_bindings: u64,
    /// Aggregate generated nodes.
    pub expanded_nodes: u64,
    /// Expansion depth.
    pub depth: u64,
    /// Aggregate work, including excluded tuples.
    pub work: u64,
    /// Owned model storage, excluding allocator metadata.
    pub retained_bytes: u64,
    /// Independent output bytes.
    pub output_bytes: u64,
}
impl ExpansionLimits {
    /// Returns frozen finite-expansion ceilings, without advertising activation.
    ///
    /// These bound compile-time description generation, not runtime fan-out.
    /// Callers must intersect their policy and preflight cardinality before allocation.
    #[must_use]
    pub const fn hard() -> Self {
        Self {
            definitions: MAX_ITEMS,
            applications: MAX_ITEMS,
            parameters: 64,
            axes: 16,
            axis_items: MAX_ITEMS,
            product_tuples: MAX_ITEMS,
            aggregate_tuples: MAX_ITEMS,
            expanded_bindings: MAX_ITEMS,
            expanded_nodes: MAX_ITEMS,
            depth: MAX_DEPTH as u64,
            work: MAX_WORK,
            retained_bytes: MAX_MODEL_BYTES,
            output_bytes: MAX_MODEL_BYTES,
        }
    }
    /// Projects raw controls into the frozen thirteen-position tuple.
    #[must_use]
    pub const fn values(self) -> [u64; 13] {
        [
            self.definitions,
            self.applications,
            self.parameters,
            self.axes,
            self.axis_items,
            self.product_tuples,
            self.aggregate_tuples,
            self.expanded_bindings,
            self.expanded_nodes,
            self.depth,
            self.work,
            self.retained_bytes,
            self.output_bytes,
        ]
    }
    /// Intersects each dimension independently, never redistributing unused controls.
    ///
    /// Zero controls remain invalid; validation must reject them rather than treating
    /// them as defaults. Tuple positions are frozen and do not follow package versions.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            definitions: self.definitions.min(other.definitions),
            applications: self.applications.min(other.applications),
            parameters: self.parameters.min(other.parameters),
            axes: self.axes.min(other.axes),
            axis_items: self.axis_items.min(other.axis_items),
            product_tuples: self.product_tuples.min(other.product_tuples),
            aggregate_tuples: self.aggregate_tuples.min(other.aggregate_tuples),
            expanded_bindings: self.expanded_bindings.min(other.expanded_bindings),
            expanded_nodes: self.expanded_nodes.min(other.expanded_nodes),
            depth: self.depth.min(other.depth),
            work: self.work.min(other.work),
            retained_bytes: self.retained_bytes.min(other.retained_bytes),
            output_bytes: self.output_bytes.min(other.output_bytes),
        }
    }
}
impl Default for ExpansionLimits {
    /// Uses frozen hard ceilings, not a consumer-selected expansion budget.
    fn default() -> Self {
        Self::hard()
    }
}
