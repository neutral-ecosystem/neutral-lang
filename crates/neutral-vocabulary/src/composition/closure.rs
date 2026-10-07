// SPDX-License-Identifier: Apache-2.0

//! Canonical dependency, visibility, and embedded-type graph validation.

use super::{Budget, CompositionError as E, values::definition_types};
use neutral_core::ordered::{OrderedMap as BTreeMap, OrderedSet as BTreeSet};
use neutral_ir::{composition::CompositionBundle, project_interface::ProjectPublicType as T};

/// Canonical vocabulary/revision/type identity, never a source alias.
type TypeKey<'a> = (&'a str, &'a str, &'a str);

/// Checks exact supplied closure and every type edge, including private/unselected definitions.
pub(super) fn validate(
    bundles: &[CompositionBundle],
    roots: &[(&str, &str)],
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut owners = BTreeMap::new();
    for bundle in bundles {
        budget.step(owners.len() as u64 + 1)?;
        owners
            .insert(bundle.identity.identity(), bundle)
            .map_err(|_| E::Allocation)?;
    }
    // Check complete dependency availability/revisions/cycles before resolving
    // type targets, so a missing locked owner is not misclassified as a type typo.
    let mut heights = BTreeMap::new();
    for bundle in bundles {
        dependency_height(bundle.identity.identity(), &owners, &mut heights, budget)?;
    }
    let mut types = BTreeMap::new();
    for bundle in bundles {
        for definition in &bundle.definitions {
            budget.step(1)?;
            budget.step(types.len() as u64)?;
            types
                .insert(
                    (
                        bundle.identity.identity(),
                        bundle.identity.version(),
                        definition.name.as_str(),
                    ),
                    definition,
                )
                .map_err(|_| E::Allocation)?;
        }
    }
    let mut embedded = BTreeMap::<TypeKey<'_>, BTreeSet<TypeKey<'_>>>::new();
    for bundle in bundles {
        let mut dependencies = BTreeMap::new();
        for d in &bundle.dependencies {
            budget.step(dependencies.len() as u64 + 1)?;
            dependencies
                .insert(d.identity.as_str(), d.version.as_str())
                .map_err(|_| E::Allocation)?;
        }
        let mut used = BTreeSet::new();
        for definition in &bundle.definitions {
            let key = (
                bundle.identity.identity(),
                bundle.identity.version(),
                definition.name.as_str(),
            );
            budget.step(embedded.len() as u64)?;
            let targets = embedded.entry(key).map_err(|_| E::Allocation)?.or_default();
            for ty in definition_types(definition) {
                let mut stack = initial_stack((ty, false))?;
                while let Some((ty, reference)) = stack.pop() {
                    budget.step(1)?;
                    match ty {
                        T::List(inner) | T::Nullable(inner) => stack.push((inner, reference)),
                        T::Ref(inner) => stack.push((inner, true)),
                        T::VocabularyNominal {
                            identity,
                            version,
                            name,
                        } => {
                            let target = (identity.as_str(), version.as_str(), name.as_str());
                            let target_definition = types.get(&target).ok_or(E::UnknownType)?;
                            if identity != bundle.identity.identity() {
                                if dependencies.get(identity.as_str()) != Some(&version.as_str()) {
                                    return Err(E::InvalidDependency);
                                }
                                budget.step(used.len() as u64)?;
                                used.insert(identity.as_str()).map_err(|_| E::Allocation)?;
                                if !target_definition.public {
                                    return Err(E::PrivateType);
                                }
                            }
                            if definition.public && !target_definition.public {
                                return Err(E::PrivateType);
                            }
                            if !reference {
                                budget.step(targets.len() as u64)?;
                                targets.insert(target).map_err(|_| E::Allocation)?;
                            }
                        }
                        T::Nominal(_) => return Err(E::UnknownType),
                        _ => {}
                    }
                }
            }
        }
        if dependencies.keys().any(|name| !used.contains(name)) {
            return Err(E::InvalidDependency);
        }
    }
    reject_embedded_cycles(&embedded, budget)?;
    // Traverse *all* supplied bundle dependency paths, not just the first path
    // reaching a shared leaf. The longest-path memo enforces diamond depth too.
    validate_root_cover(&owners, roots, budget)
}

/// Verifies supplied bundles equal the union of direct source requirements and their dependencies.
fn validate_root_cover(
    owners: &BTreeMap<&str, &CompositionBundle>,
    roots: &[(&str, &str)],
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut required = BTreeSet::new();
    let mut root_versions = BTreeMap::new();
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(roots.len())
        .map_err(|_| E::Allocation)?;
    stack.extend_from_slice(roots);
    while let Some((identity, version)) = stack.pop() {
        budget.step(1)?;
        budget.step(root_versions.len() as u64)?;
        if root_versions
            .insert(identity, version)
            .map_err(|_| E::Allocation)?
            .is_some_and(|previous| previous != version)
        {
            return Err(E::DuplicateBundle);
        }
        let bundle = owners.get(identity).ok_or(E::MissingDependency)?;
        if bundle.identity.version() != version {
            return Err(E::MissingDependency);
        }
        budget.step(required.len() as u64)?;
        if required.insert(identity).map_err(|_| E::Allocation)? {
            stack
                .try_reserve(bundle.dependencies.len())
                .map_err(|_| E::Allocation)?;
            stack.extend(
                bundle
                    .dependencies
                    .iter()
                    .map(|d| (d.identity.as_str(), d.version.as_str())),
            );
        }
    }
    if required.len() != owners.len() {
        return Err(E::ExtraBundle);
    }
    Ok(())
}

/// Rejects embedded recursion with iterative enter/leave states rather than call-stack traversal.
fn reject_embedded_cycles<'a>(
    graph: &BTreeMap<TypeKey<'a>, BTreeSet<TypeKey<'a>>>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut done = BTreeSet::new();
    for root in graph.keys() {
        let mut visiting = BTreeSet::new();
        let mut stack = initial_stack((*root, false))?;
        while let Some((key, leaving)) = stack.pop() {
            budget.step(1)?;
            if leaving {
                budget.step(visiting.len() as u64 + done.len() as u64)?;
                visiting.remove(&key);
                done.insert(key).map_err(|_| E::Allocation)?;
                continue;
            }
            if done.contains(&key) {
                continue;
            }
            budget.step(visiting.len() as u64)?;
            if !visiting.insert(key).map_err(|_| E::Allocation)? {
                return Err(E::EmbeddedCycle);
            }
            stack
                .try_reserve(1 + graph.get(&key).map_or(0, BTreeSet::len))
                .map_err(|_| E::Allocation)?;
            stack.push((key, true));
            if let Some(targets) = graph.get(&key) {
                stack.extend(targets.iter().map(|target| (*target, false)));
            }
        }
    }
    Ok(())
}

/// Computes longest dependency paths with iterative memoization and exact revision checking.
fn dependency_height<'a>(
    root: &'a str,
    owners: &BTreeMap<&'a str, &'a CompositionBundle>,
    heights: &mut BTreeMap<&'a str, u64>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut visiting = BTreeSet::new();
    let mut stack = initial_stack((root, false))?;
    while let Some((name, leaving)) = stack.pop() {
        budget.step(1)?;
        let bundle = owners.get(name).ok_or(E::MissingDependency)?;
        if leaving {
            let mut height = 1;
            for dependency in &bundle.dependencies {
                let child = heights
                    .get(dependency.identity.as_str())
                    .ok_or(E::DependencyCycle)?;
                height = height.max(child.checked_add(1).ok_or(E::Limit)?);
            }
            if height > budget.limits.dependency_depth.min(super::MAX_DEPTH) {
                return Err(E::Limit);
            }
            budget.step(heights.len() as u64 + visiting.len() as u64)?;
            heights.insert(name, height).map_err(|_| E::Allocation)?;
            visiting.remove(name);
            continue;
        }
        if heights.contains_key(name) {
            continue;
        }
        budget.step(visiting.len() as u64)?;
        if !visiting.insert(name).map_err(|_| E::Allocation)? {
            return Err(E::DependencyCycle);
        }
        stack
            .try_reserve(1 + bundle.dependencies.len())
            .map_err(|_| E::Allocation)?;
        stack.push((name, true));
        for dependency in &bundle.dependencies {
            let child = owners
                .get(dependency.identity.as_str())
                .ok_or(E::MissingDependency)?;
            if child.identity.version() != dependency.version {
                return Err(E::MissingDependency);
            }
            stack.push((child.identity.identity(), false));
        }
    }
    Ok(())
}

/// Starts an iterative traversal with one fallible allocation rather than an infallible vector literal.
fn initial_stack<T>(value: T) -> Result<Vec<T>, E> {
    let mut stack = Vec::new();
    stack.try_reserve_exact(1).map_err(|_| E::Allocation)?;
    stack.push(value);
    Ok(stack)
}
