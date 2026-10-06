// SPDX-License-Identifier: Apache-2.0

//! Canonical dependency, visibility, and embedded-type graph validation.

use super::{Budget, CompositionError as E, values::definition_types};
use neutral_ir::{composition::CompositionBundle, project_interface::ProjectPublicType as T};
use std::collections::{BTreeMap, BTreeSet};

/// Canonical vocabulary/revision/type identity, never a source alias.
type TypeKey<'a> = (&'a str, &'a str, &'a str);

/// Checks exact supplied closure and every type edge, including private/unselected definitions.
pub(super) fn validate(
    bundles: &[CompositionBundle],
    roots: &[(&str, &str)],
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let owners = bundles
        .iter()
        .map(|bundle| (bundle.identity.identity(), bundle))
        .collect::<BTreeMap<_, _>>();
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
            types.insert(
                (
                    bundle.identity.identity(),
                    bundle.identity.version(),
                    definition.name.as_str(),
                ),
                definition,
            );
        }
    }
    let mut embedded = BTreeMap::<TypeKey<'_>, BTreeSet<TypeKey<'_>>>::new();
    for bundle in bundles {
        let dependencies = bundle
            .dependencies
            .iter()
            .map(|d| (d.identity.as_str(), d.version.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut used = BTreeSet::new();
        for definition in &bundle.definitions {
            let key = (
                bundle.identity.identity(),
                bundle.identity.version(),
                definition.name.as_str(),
            );
            let targets = embedded.entry(key).or_default();
            for ty in definition_types(definition) {
                let mut stack = vec![(ty, false)];
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
                                used.insert(identity.as_str());
                                if !target_definition.public {
                                    return Err(E::PrivateType);
                                }
                            }
                            if definition.public && !target_definition.public {
                                return Err(E::PrivateType);
                            }
                            if !reference {
                                targets.insert(target);
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
    let mut stack = roots.to_vec();
    while let Some((identity, version)) = stack.pop() {
        budget.step(1)?;
        if root_versions
            .insert(identity, version)
            .is_some_and(|previous| previous != version)
        {
            return Err(E::DuplicateBundle);
        }
        let bundle = owners.get(identity).ok_or(E::MissingDependency)?;
        if bundle.identity.version() != version {
            return Err(E::MissingDependency);
        }
        if required.insert(identity) {
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
        let mut stack = vec![(*root, false)];
        while let Some((key, leaving)) = stack.pop() {
            budget.step(1)?;
            if leaving {
                visiting.remove(&key);
                done.insert(key);
                continue;
            }
            if done.contains(&key) {
                continue;
            }
            if !visiting.insert(key) {
                return Err(E::EmbeddedCycle);
            }
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
    let mut stack = vec![(root, false)];
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
            heights.insert(name, height);
            visiting.remove(name);
            continue;
        }
        if heights.contains_key(name) {
            continue;
        }
        if !visiting.insert(name) {
            return Err(E::DependencyCycle);
        }
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
