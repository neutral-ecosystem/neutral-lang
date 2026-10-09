// SPDX-License-Identifier: Apache-2.0

//! Atomic, resolved binding validation; no source parser, wire decoder or acquisition.

use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::TryClone;

use super::{
    Budget, CompositionError as E, CompositionLimits, ValidatedCompositionScope, check_count,
    closure, scope, supplied, values,
};
use neutral_core::allocation::Shared as Arc;
use neutral_core::ordered::OrderedMap as BTreeMap;
use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{
        BindingValue as V, CompositionBinding, CompositionBindingReference, ValueOrigin,
        ValuePathSegment as P,
    },
    language::{is_protected_name, is_snake_name},
    project_interface::ProjectPublicType as T,
};

/// Immutable materialized binding and its separate, occurrence-sensitive safe facts.
#[derive(Clone, Eq, PartialEq)]
pub struct ValidatedCompositionBinding {
    /// Complete canonical meaning, never mutably exposed.
    binding: CompositionBinding,
    /// Supplied/null/default/absence classification, not fabricated source coordinates.
    origins: Vec<ValueOrigin>,
    /// Actual identity-only reference occurrences, in canonical value traversal order.
    references: Vec<CompositionBindingReference>,
}

impl std::fmt::Debug for ValidatedCompositionBinding {
    /// Logs visibility and counts without private owners, values or reference target names.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValidatedCompositionBinding")
            .field("public", &self.binding.public)
            .field("origins", &self.origins.len())
            .field("references", &self.references.len())
            .finish_non_exhaustive()
    }
}

impl ValidatedCompositionBinding {
    /// Returns immutable complete meaning for trusted producers; public readers filter visibility.
    #[must_use]
    pub fn binding(&self) -> &CompositionBinding {
        &self.binding
    }

    /// Returns canonical supplied/null/default/absence facts without claiming source attribution.
    #[must_use]
    pub fn origins(&self) -> &[ValueOrigin] {
        &self.origins
    }

    /// Returns checked target identities without evaluating or expanding the target bindings.
    #[must_use]
    pub fn references(&self) -> &[CompositionBindingReference] {
        &self.references
    }
}

/// Complete resolved bindings over one validated source/vocabulary contract scope.
///
/// This boundary establishes value, reference and public type closure, not source
/// grammar, imports, provenance, project companions, encoding or project identity.
#[derive(Clone, Eq, PartialEq)]
pub struct ValidatedCompositionBindings {
    /// Shared, immutable nominal contracts used by every binding in the request.
    scope: Arc<ValidatedCompositionScope>,
    /// Complete private/public bindings, ordered by exact module-symbol identity.
    bindings: Vec<ValidatedCompositionBinding>,
}

impl std::fmt::Debug for ValidatedCompositionBindings {
    /// Logs counts only; callers cannot recover private names or supplied data from debug.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValidatedCompositionBindings")
            .field("bindings", &self.bindings.len())
            .finish_non_exhaustive()
    }
}

impl ValidatedCompositionBindings {
    /// Returns immutable nominal contracts without copying defaults or catalogue bodies.
    #[must_use]
    pub fn scope(&self) -> &Arc<ValidatedCompositionScope> {
        &self.scope
    }

    /// Returns complete validated bindings for trusted producers; public readers filter visibility.
    #[must_use]
    pub fn bindings(&self) -> &[ValidatedCompositionBinding] {
        &self.bindings
    }
}

/// Identity-only target validation against the whole request, including later/cyclic bindings.
struct References<'a> {
    /// Every exact binding owner and its already checked signature/visibility.
    index: &'a BTreeMap<&'a ModuleSymbolIdentity, &'a CompositionBinding>,
    /// Consumer context for public and cross-module visibility checks.
    consumer: &'a CompositionBinding,
}

impl References<'_> {
    /// Rejects dangling, differently typed or inaccessible targets without inspecting target values.
    fn check(
        &self,
        target: &ModuleSymbolIdentity,
        expected: &T,
        budget: &mut Budget<'_>,
    ) -> Result<(), E> {
        budget.key(
            target.module().module_name().len() + target.declaration_name().len(),
            self.index.len(),
        )?;
        let target = self.index.get(target).ok_or(E::InvalidValue)?;
        // Repeated references must pay for nominal-key/type comparison too;
        // a long type name cannot amplify work behind a short binding name.
        scope::preflight_type(expected, budget)?;
        if &target.ty != expected {
            return Err(E::InvalidValue);
        }
        if !target.public
            && (self.consumer.public || target.owner.module() != self.consumer.owner.module())
        {
            return Err(E::PrivateType);
        }
        Ok(())
    }
}

/// Validates and materializes every resolved binding atomically under cumulative request budgets.
///
/// Public roots have public nominal closure and public binding reference targets.
/// Lists, nullable wrappers and selected variant payloads use the default validator;
/// references are invariant identity edges and may form cycles without evaluation.
/// Defaults remain closed by construction and optional absence is not null.
/// Input order cannot affect typing, reference availability or final ordering.
/// A caller must separately establish source imports and complete project companions.
///
/// # Errors
/// Rejects malformed/duplicate owners, inaccessible/dangling/mismatched types or
/// references, invalid defaults/constraints, limits, allocation or cancellation;
/// never publishes the passing prefix of an invalid request.
pub fn validate_composition_bindings(
    scope: Arc<ValidatedCompositionScope>,
    bindings: &[CompositionBinding],
    limits: CompositionLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedCompositionBindings, E> {
    limits.validate()?;
    let mut budget = Budget::new(limits, cancellation);
    budget.origin_root_depth = 0;
    budget.step(1)?;
    check_count(bindings.len(), limits.value_nodes)?;
    let catalogue = recheck_scope(&scope, &mut budget)?;
    let mut index = BTreeMap::new();
    for binding in bindings {
        let owner = &binding.owner;
        budget.key(
            owner.module().module_name().len() + owner.declaration_name().len(),
            bindings.len(),
        )?;
        if owner.module().language_behavior_version() != V1_SOURCE_PROFILE
            || !owner.module().module_name().split("::").all(is_snake_name)
            || !is_snake_name(owner.declaration_name())
            || is_protected_name(owner.declaration_name())
        {
            return Err(E::InvalidContract);
        }
        if owner.module().module_name().len() as u64 > limits.json.string_bytes()
            || owner.declaration_name().len() as u64 > limits.json.string_bytes()
        {
            return Err(E::Limit);
        }
        scope::preflight_type(&binding.ty, &mut budget)?;
        check_type_visibility(&binding.ty, binding, &catalogue, &mut budget)?;
        budget.step(index.len() as u64)?;
        if index
            .insert(owner, binding)
            .map_err(|_| E::Allocation)?
            .is_some()
        {
            return Err(E::InvalidContract);
        }
    }
    let mut result = Vec::new();
    result
        .try_retain(bindings.len())
        .map_err(|_| E::Allocation)?;
    for binding in index.values() {
        let references = References {
            index: &index,
            consumer: binding,
        };
        let value = values::materialize_with(
            &binding.value,
            &binding.ty,
            &catalogue,
            &mut budget,
            0,
            &|target, expected, budget| references.check(target, expected, budget),
        )
        .map_err(|error| {
            if error == E::InvalidDefault {
                E::InvalidValue
            } else {
                error
            }
        })?;
        let mut origins = Vec::new();
        supplied::classify(
            Some(&binding.value),
            Some(&value),
            false,
            &mut Vec::new(),
            &mut origins,
            &mut budget,
        )?;
        let mut references = Vec::new();
        for origin in &origins {
            if let Some(V::Reference(target)) = occurrence(&value, &origin.path, &mut budget)? {
                budget.step(
                    target.module().module_name().len() as u64
                        + target.declaration_name().len() as u64,
                )?;
                references.try_retain(1).map_err(|_| E::Allocation)?;
                references.push(CompositionBindingReference {
                    path: origin.path.try_clone().map_err(|_| E::Allocation)?,
                    target: target.try_clone().map_err(|_| E::Allocation)?,
                });
            }
        }
        result.push(ValidatedCompositionBinding {
            binding: CompositionBinding {
                owner: binding.owner.try_clone().map_err(|_| E::Allocation)?,
                public: binding.public,
                ty: binding.ty.try_clone().map_err(|_| E::Allocation)?,
                value,
            },
            origins,
            references,
        });
    }
    budget.step(1)?;
    Ok(ValidatedCompositionBindings {
        scope,
        bindings: result,
    })
}

/// Applies stricter semantic policy to dormant contracts before materializing any supplied binding.
///
/// Captured-byte/digest policy belongs to capture, not this resolved model;
/// original bundle bytes are deliberately not reconstructed from contracts.
fn recheck_scope<'a>(
    scope: &'a ValidatedCompositionScope,
    budget: &mut Budget<'_>,
) -> Result<values::Catalogue<'a>, E> {
    let limits = budget.limits;
    check_count(scope.catalogue().bundles().len(), limits.bundles)?;
    for bundle in scope.catalogue().bundles() {
        check_count(bundle.definitions.len(), limits.json.types())?;
        check_count(bundle.dependencies.len(), limits.dependencies_per_bundle)?;
        super::charge(
            &mut budget.edges,
            bundle.dependencies.len() as u64,
            limits.dependency_edges,
        )?;
        for definition in &bundle.definitions {
            scope::preflight(definition, false, budget)?;
        }
    }
    let mut roots = Vec::new();
    roots
        .try_retain(scope.catalogue().bundles().len())
        .map_err(|_| E::Allocation)?;
    roots.extend(
        scope
            .catalogue()
            .bundles()
            .iter()
            .map(|bundle| (bundle.identity.identity(), bundle.identity.version())),
    );
    closure::validate(scope.catalogue().bundles(), &roots, budget)?;
    let mut module_types = BTreeMap::new();
    for source in scope.sources() {
        budget.step(module_types.len() as u64)?;
        super::charge(
            module_types
                .entry(source.owner.module())
                .map_err(|_| E::Allocation)?
                .or_insert(0),
            1,
            limits.json.types(),
        )?;
        scope::preflight(&source.definition, true, budget)?;
    }
    let catalogue = scope::lookup(scope.catalogue(), scope.sources(), budget)?;
    // Unused defaults remain subject to stricter bounds too; checking only
    // supplied bindings would allow dormant invalid data through this boundary.
    for definition in scope
        .sources()
        .iter()
        .map(|source| &source.definition)
        .chain(
            scope
                .catalogue()
                .bundles()
                .iter()
                .flat_map(|bundle| &bundle.definitions),
        )
    {
        if let neutral_ir::composition::CompositionBody::Record(fields) = &definition.body {
            for field in fields {
                let restrictions = values::validate_restrictions(field, budget)?;
                if let Some(default) = &field.default {
                    let value = values::materialize(default, &field.ty, &catalogue, budget, 0)?;
                    values::check_restrictions(&restrictions, &value, budget)?;
                }
            }
        }
    }
    Ok(catalogue)
}

/// Checks every type wrapper before public/external access to an exact nominal owner.
fn check_type_visibility(
    ty: &T,
    binding: &CompositionBinding,
    catalogue: &values::Catalogue<'_>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut current = ty;
    loop {
        budget.step(1)?;
        match current {
            T::List(inner) | T::Nullable(inner) | T::Ref(inner) => current = inner,
            T::Nominal(owner) => {
                if !catalogue.resolve(current, budget)?.public
                    && (binding.public || owner.module() != binding.owner.module())
                {
                    return Err(E::PrivateType);
                }
                return Ok(());
            }
            T::VocabularyNominal { .. } => {
                if !catalogue.resolve(current, budget)?.public {
                    return Err(E::PrivateType);
                }
                return Ok(());
            }
            _ => return Ok(()),
        }
    }
}

/// Resolves one already classified value occurrence without expanding any reference target.
fn occurrence<'a>(
    mut value: &'a V,
    path: &[P],
    budget: &mut Budget<'_>,
) -> Result<Option<&'a V>, E> {
    for segment in path {
        budget.step(1)?;
        value = match (segment, value) {
            (P::Field(name), V::Record(fields)) => {
                budget.key(name.len(), fields.len())?;
                let index = fields
                    .binary_search_by(|(field, _)| field.cmp(name))
                    .map_err(|_| E::InvalidValue)?;
                let Some(value) = &fields[index].1 else {
                    return Ok(None);
                };
                value
            }
            (P::Element(index), V::List(items)) => items
                .get(usize::try_from(*index).map_err(|_| E::Limit)?)
                .ok_or(E::InvalidValue)?,
            (P::Payload, V::Variant { payload, .. }) => payload,
            _ => return Err(E::InvalidValue),
        };
    }
    Ok(Some(value))
}
