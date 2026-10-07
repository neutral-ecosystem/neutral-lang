// SPDX-License-Identifier: Apache-2.0

//! Explicit, bounded composition validation without activating old project schemas.

mod bindings;
mod closure;
mod copy;
mod decode;
mod model;
mod scope;
mod supplied;
mod values;

pub use bindings::{
    ValidatedCompositionBinding, ValidatedCompositionBindings, validate_composition_bindings,
};
pub use model::validate_composition_model;
pub use scope::{ValidatedCompositionScope, validate_composition_scope};
pub use supplied::{ValidatedCompositionValue, materialize_composition_value};
pub use values::check_composition_field_restrictions;

use crate::{VocabularyError, VocabularyLimits, VocabularyLock};
use neutral_core::{CancellationToken, VocabularyContentDigest};
use neutral_ir::composition::CompositionBundle;

/// New logical composition schema, independent of package versions.
pub const SCHEMA_VERSION: &str = neutral_ir::composition::profile::VOCABULARY_SCHEMA_VERSION;
/// Reused strict JSON encoding; its representation is unchanged.
pub const ENCODING_VERSION: &str = crate::PROJECT_VOCABULARY_ENCODING_VERSION;
/// Exact structural feature required by every composition-schema bundle.
pub const REQUIRED_FEATURE: &str = neutral_ir::composition::profile::VOCABULARY_COMPOSITION_FEATURE;
/// Maximum recursive semantic layers, intersected with caller limits.
pub const MAX_DEPTH: u64 = crate::MAX_VOCABULARY_NESTING_DEPTH;
/// Hard aggregate item/work ceiling, independent of semantic restrictions.
pub const MAX_WORK: u64 = neutral_ir::composition::profile::MAX_ITEMS;
/// Hard total captured-byte ceiling for one supplied closure.
pub const MAX_CAPTURED_BYTES: u64 = 67_108_864;
/// Largest legitimate closed-schema JSON object, bounding duplicate-key inspection work.
pub const MAX_OBJECT_MEMBERS: u64 = 8;

/// Caller policy for independent composition and dependency resource budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionLimits {
    /// Existing strict per-bundle JSON, scalar, type and field budgets.
    pub json: VocabularyLimits,
    /// Maximum supplied bundles.
    pub bundles: u64,
    /// Maximum aggregate captured bytes.
    pub captured_bytes: u64,
    /// Maximum direct dependencies per bundle.
    pub dependencies_per_bundle: u64,
    /// Maximum total dependency edges.
    pub dependency_edges: u64,
    /// Maximum number of bundles along one dependency path.
    pub dependency_depth: u64,
    /// Maximum aggregate nominal definitions.
    pub total_types: u64,
    /// Maximum aggregate record fields.
    pub total_fields: u64,
    /// Maximum alternatives in one variant.
    pub alternatives_per_type: u64,
    /// Maximum aggregate variant alternatives.
    pub total_alternatives: u64,
    /// Maximum finite choices in one field.
    pub choices_per_field: u64,
    /// Maximum aggregate finite choices.
    pub total_choices: u64,
    /// Maximum recursive type layers.
    pub type_depth: u64,
    /// Maximum closed default/value layers, including nominal expansion.
    pub value_depth: u64,
    /// Maximum cumulative materialized value visits, including absent field states.
    pub value_nodes: u64,
    /// Maximum semantic traversal, comparison, and default-expansion work.
    pub work: u64,
}

impl CompositionLimits {
    /// Projects the independently retained acceptance controls in frozen contract order.
    #[must_use]
    pub const fn policy(self) -> neutral_ir::composition::project::CompositionPolicy {
        neutral_ir::composition::project::CompositionPolicy::from_values([
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
        ])
    }

    /// Combines an explicit wire policy with independent JSON/scalar consumer controls.
    #[must_use]
    pub const fn from_policy(
        json: VocabularyLimits,
        policy: neutral_ir::composition::project::CompositionPolicy,
    ) -> Self {
        Self {
            json,
            bundles: policy.bundles,
            captured_bytes: policy.captured_bytes,
            dependencies_per_bundle: policy.dependencies_per_bundle,
            dependency_edges: policy.dependency_edges,
            dependency_depth: policy.dependency_depth,
            total_types: policy.total_types,
            total_fields: policy.total_fields,
            alternatives_per_type: policy.alternatives_per_type,
            total_alternatives: policy.total_alternatives,
            choices_per_field: policy.choices_per_field,
            total_choices: policy.total_choices,
            type_depth: policy.type_depth,
            value_depth: policy.value_depth,
            value_nodes: policy.value_nodes,
            work: policy.work,
        }
    }
    /// Derives finite defaults from existing structural policy; every budget remains independently tunable.
    #[must_use]
    pub const fn from_vocabulary(json: VocabularyLimits) -> Self {
        Self {
            json,
            bundles: json.types(),
            captured_bytes: json.bundle_bytes(),
            dependencies_per_bundle: json.array_items(),
            dependency_edges: json.total_nodes(),
            dependency_depth: json.nesting_depth(),
            total_types: json.types(),
            total_fields: json.total_nodes(),
            alternatives_per_type: json.fields(),
            total_alternatives: json.total_nodes(),
            choices_per_field: json.array_items(),
            total_choices: json.total_nodes(),
            type_depth: json.nesting_depth(),
            value_depth: json.nesting_depth(),
            value_nodes: json.total_nodes(),
            work: json.total_nodes(),
        }
    }

    /// Checks policy before processing input, rather than silently disabling a resource gate.
    ///
    /// # Errors
    /// Returns [`CompositionError::InvalidLimits`] if any composition budget is zero.
    pub fn validate(self) -> Result<(), CompositionError> {
        if [
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
        .contains(&0)
        {
            Err(CompositionError::InvalidLimits)
        } else {
            Ok(())
        }
    }
}

/// Borrowed already-captured bytes and exact lock; no path/URL/acquisition callback exists.
#[derive(Clone, Copy)]
pub struct CapturedCompositionBundle<'a> {
    /// Exact captured JSON bytes.
    pub bytes: &'a [u8],
    /// Host's exact semantic/content lock.
    pub lock: &'a VocabularyLock,
}

/// Bounded composition failure; errors never contain captured text or host paths.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionError {
    /// Existing strict JSON/envelope/scalar failure.
    Vocabulary(VocabularyError),
    /// A caller supplied a zero independent limit.
    InvalidLimits,
    /// An independent caller or hard resource bound was exceeded.
    Limit,
    /// Cooperative cancellation, before any partial catalogue publication.
    Cancelled,
    /// Duplicate canonical identity or conflicting revision.
    DuplicateBundle,
    /// Missing root/dependency bundle or revision mismatch.
    MissingDependency,
    /// Supplied bundle is outside the required transitive closure.
    ExtraBundle,
    /// Dependency is duplicate, self-referential, or unused by any type.
    InvalidDependency,
    /// Direct bundle dependency cycle.
    DependencyCycle,
    /// Public signature or external target exposes a private type.
    PrivateType,
    /// Missing nominal owner/type or unsupported reference target.
    UnknownType,
    /// Embedded nominal types form a cycle, including unselected alternatives.
    EmbeddedCycle,
    /// Invalid tag/name/type/field/presence shape.
    InvalidContract,
    /// Invalid/incompatible/contradictory declarative restrictions.
    InvalidRestrictions,
    /// Duplicate finite scalar choice after normalization.
    DuplicateChoice,
    /// Closed default is incomplete, incompatible, or violates restrictions.
    InvalidDefault,
    /// Supplied value violates its selected public type or declarative restrictions.
    InvalidValue,
    /// A bounded fallible result/origin reservation failed.
    Allocation,
}

impl From<VocabularyError> for CompositionError {
    /// Preserves existing vocabulary classifications without leaking untrusted values.
    fn from(value: VocabularyError) -> Self {
        Self::Vocabulary(value)
    }
}

impl CompositionError {
    /// Returns a stable composition-boundary diagnostic without untrusted text or host locations.
    ///
    /// The nested vocabulary classification remains available for structured
    /// inspection. Source compilation will attach reviewed safe locations; this
    /// captured-byte library must not invent a source span or acquisition path.
    #[must_use]
    pub const fn diagnostic_code(&self) -> &'static str {
        match self {
            Self::Vocabulary(_) => "NEU-COM-001",
            Self::InvalidLimits => "NEU-COM-002",
            Self::Limit => "NEU-COM-003",
            Self::Cancelled => "NEU-COM-004",
            Self::DuplicateBundle => "NEU-COM-005",
            Self::MissingDependency => "NEU-COM-006",
            Self::ExtraBundle => "NEU-COM-007",
            Self::InvalidDependency => "NEU-COM-008",
            Self::DependencyCycle => "NEU-COM-009",
            Self::PrivateType => "NEU-COM-010",
            Self::UnknownType => "NEU-COM-011",
            Self::EmbeddedCycle => "NEU-COM-012",
            Self::InvalidContract => "NEU-COM-013",
            Self::InvalidRestrictions => "NEU-COM-014",
            Self::DuplicateChoice => "NEU-COM-015",
            Self::InvalidDefault => "NEU-COM-016",
            Self::InvalidValue => "NEU-COM-017",
            Self::Allocation => "NEU-COM-018",
        }
    }
}

/// Complete validated immutable vocabulary catalogue, not a compiled project artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedComposition {
    /// Exact canonical bundle owners with their complete public/private contracts.
    bundles: Vec<CompositionBundle>,
}

impl ValidatedComposition {
    /// Returns all bundles in canonical identity order without mutable validated access.
    #[must_use]
    pub fn bundles(&self) -> &[CompositionBundle] {
        &self.bundles
    }
}

/// Validates exact transitive lock cover, closed schemas, typing, defaults and public closure.
///
/// Roots are canonical identities/revisions required by source, not post-compilation
/// view selections. Existing schema bundles are permitted only as dependency leaves.
/// No partially validated catalogue is returned when any bundle fails.
///
/// # Errors
/// Returns a classified schema/lock/closure/semantic/limit/cancellation failure.
pub fn validate_composition_closure(
    inputs: &[CapturedCompositionBundle<'_>],
    roots: &[(&str, &str)],
    limits: CompositionLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedComposition, CompositionError> {
    limits.validate()?;
    // A caller cannot remove the JSON stage's hard node/depth/object ceilings
    // by making its semantic budgets generous. Every legitimate object has at
    // most eight members; duplicate detection is therefore bounded per key.
    let mut limits = limits;
    limits.json.object_members = limits.json.object_members.min(MAX_OBJECT_MEMBERS);
    limits.json.total_nodes = limits.json.total_nodes.min(MAX_WORK);
    limits.json.array_items = limits.json.array_items.min(MAX_WORK);
    limits.json.nesting_depth = limits.json.nesting_depth.min(MAX_DEPTH);
    let mut budget = Budget::new(limits, cancellation);
    budget.step(1)?;
    check_count(inputs.len(), limits.bundles)?;
    check_count(roots.len(), limits.bundles)?;
    let mut bytes = 0_u64;
    let mut owners = neutral_core::ordered::OrderedMap::new();
    for input in inputs {
        budget.step(input.bytes.len() as u64 + 1)?;
        bytes = bytes
            .checked_add(input.bytes.len() as u64)
            .ok_or(CompositionError::Limit)?;
        if bytes > limits.captured_bytes.min(MAX_CAPTURED_BYTES) {
            return Err(CompositionError::Limit);
        }
        budget.step(owners.len() as u64)?;
        budget.allocation()?;
        if owners
            .insert(input.lock.identity(), input.lock.version())
            .map_err(|_| CompositionError::Allocation)?
            .is_some()
        {
            return Err(CompositionError::DuplicateBundle);
        }
        if !VocabularyContentDigest::from_bytes(input.bytes)
            .securely_matches(input.lock.content_digest())
        {
            return Err(VocabularyError::DigestMismatch.into());
        }
    }
    let mut bundles = budget.storage(inputs.len())?;
    for input in inputs {
        bundles.push(decode::bundle(*input, &mut budget)?);
    }
    bundles.sort_unstable_by(|left, right| left.identity.identity().cmp(right.identity.identity()));
    closure::validate(&bundles, roots, &mut budget)?;
    values::validate_defaults(&mut bundles, &mut budget)?;
    budget.step(1)?;
    Ok(ValidatedComposition { bundles })
}

/// Per-request semantic budget; no process-global state can leak across requests.
struct Budget<'a> {
    /// Request-local reservation checkpoint count, never process-global.
    allocations: usize,
    /// Deterministic allocation failure or cancellation at one private checkpoint.
    allocation_fault: Option<(usize, bool)>,
    /// Complete caller policy.
    limits: CompositionLimits,
    /// Cancellation signal shared by all phases.
    cancellation: &'a CancellationToken,
    /// Total charged semantic/comparison/default work.
    work: u64,
    /// Total record fields decoded.
    fields: u64,
    /// Total nominal types decoded.
    types: u64,
    /// Total alternatives decoded.
    alternatives: u64,
    /// Total choices decoded.
    choices: u64,
    /// Total dependency edges decoded.
    edges: u64,
    /// Cumulative materialization visits, independent of aggregate work.
    value_nodes: u64,
    /// Standalone origins start at one; explicitly selected project /2 starts at zero.
    origin_root_depth: u64,
}

impl<'a> Budget<'a> {
    /// Starts isolated counters for one captured closure.
    fn new(limits: CompositionLimits, cancellation: &'a CancellationToken) -> Self {
        Self {
            allocations: 0,
            allocation_fault: None,
            limits,
            cancellation,
            work: 0,
            fields: 0,
            types: 0,
            alternatives: 0,
            choices: 0,
            edges: 0,
            value_nodes: 0,
            origin_root_depth: 1,
        }
    }
    /// Charges work and observes cancellation before proportional semantic operations.
    fn step(&mut self, amount: u64) -> Result<(), CompositionError> {
        if self.cancellation.is_cancelled() {
            return Err(CompositionError::Cancelled);
        }
        charge(&mut self.work, amount, self.limits.work)
    }

    /// Checks cancellation/work before an allocation and supports isolated private fault tests.
    fn allocation(&mut self) -> Result<(), CompositionError> {
        self.step(1)?;
        {
            let current = self.allocations;
            self.allocations += 1;
            if let Some((index, cancel)) = self.allocation_fault
                && index == current
            {
                if cancel {
                    self.cancellation.cancel();
                    return Err(CompositionError::Cancelled);
                }
                return Err(CompositionError::Allocation);
            }
        }
        Ok(())
    }

    /// Reserves complete outer storage after the caller has checked its independent count.
    fn storage<T>(&mut self, count: usize) -> Result<Vec<T>, CompositionError> {
        self.allocation()?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(count)
            .map_err(|_| CompositionError::Allocation)?;
        self.step(0)?;
        Ok(result)
    }

    /// Copies bounded exact UTF-8 bytes without infallible string growth.
    fn owned(&mut self, value: &str) -> Result<String, CompositionError> {
        self.allocation()?;
        let result =
            neutral_core::allocation::text(value).map_err(|_| CompositionError::Allocation)?;
        self.step(0)?;
        Ok(result)
    }

    /// Reserves a recursive wrapper after the caller has checked depth and shape.
    fn boxed<T>(&mut self, value: T) -> Result<Box<T>, CompositionError> {
        self.allocation()?;
        let result =
            neutral_core::allocation::boxed(value).map_err(|_| CompositionError::Allocation)?;
        self.step(0)?;
        Ok(result)
    }

    /// Charges ordered insertion shifts before reserving and retaining unique membership.
    fn unique<K: Ord>(
        &mut self,
        set: &mut neutral_core::ordered::OrderedSet<K>,
        key: K,
    ) -> Result<bool, CompositionError> {
        self.step(set.len() as u64)?;
        self.allocation()?;
        set.insert(key).map_err(|_| CompositionError::Allocation)
    }
    /// Intersects recursion policy with the stack-safe hard ceiling.
    fn depth(&mut self, depth: u64, limit: u64) -> Result<(), CompositionError> {
        self.step(1)?;
        if depth > limit.min(MAX_DEPTH) {
            Err(CompositionError::Limit)
        } else {
            Ok(())
        }
    }

    /// Charges an independent output-value visit before reserving materialized storage.
    fn value_node(&mut self) -> Result<(), CompositionError> {
        self.step(1)?;
        charge(&mut self.value_nodes, 1, self.limits.value_nodes)
    }

    /// Reserves conservative string-key inspection work before ordered insertion and sorting.
    fn key(&mut self, bytes: usize, items: usize) -> Result<(), CompositionError> {
        let factor = u64::from(items.max(1).ilog2()) + 1;
        self.step(
            (bytes as u64)
                .checked_mul(factor)
                .and_then(|n| n.checked_mul(4))
                .ok_or(CompositionError::Limit)?,
        )
    }
}

/// Checks independent counts before reserving/decoding their collections.
fn check_count(count: usize, limit: u64) -> Result<(), CompositionError> {
    if count as u64 > limit.min(MAX_WORK) {
        Err(CompositionError::Limit)
    } else {
        Ok(())
    }
}

/// Increments a cumulative counter with checked arithmetic and a fixed hard ceiling.
fn charge(counter: &mut u64, amount: u64, limit: u64) -> Result<(), CompositionError> {
    *counter = counter.checked_add(amount).ok_or(CompositionError::Limit)?;
    if *counter > limit.min(MAX_WORK) {
        Err(CompositionError::Limit)
    } else {
        Ok(())
    }
}
