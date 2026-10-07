// SPDX-License-Identifier: Apache-2.0

//! Independent canonical resolved-catalogue validation for successor IR/wire consumers.

use super::{
    Budget, CompositionError as E, CompositionLimits, ENCODING_VERSION, REQUIRED_FEATURE,
    SCHEMA_VERSION, ValidatedComposition, check_count, closure, scope, values,
};
use neutral_core::CancellationToken;
use neutral_ir::{
    composition::{CompositionBody, CompositionBundle, FieldPresence, FieldRestrictions},
    language::{is_exact_release_version, is_upper_name},
    project_interface::ProjectPublicType as T,
};

/// Validates canonical complete contracts without captured JSON, source parsing or compiler linkage.
///
/// All metadata, dependencies, public/embedded closure and unused defaults are
/// checked independently. Unknown schemas and noncanonical materialization reject,
/// rather than silently sorting or rewriting an external producer's data. Digests
/// are evidence claims; complete project companion checks bind them separately.
///
/// # Errors
/// Rejects malformed/noncanonical contracts, limits, allocation or cancellation
/// before exposing any immutable catalogue or recursively cloning unbounded data.
pub fn validate_composition_model(
    bundles: &[CompositionBundle],
    limits: CompositionLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedComposition, E> {
    limits.validate()?;
    let mut budget = Budget::new(limits, cancellation);
    budget.step(1)?;
    check_count(bundles.len(), limits.bundles)?;
    let mut previous = None;
    for bundle in bundles {
        let identity = &bundle.identity;
        budget.key(
            identity.identity().len() + identity.version().len(),
            bundles.len(),
        )?;
        if previous.is_some_and(|owner: (&str, &str)| {
            owner >= (identity.identity(), identity.version()) || owner.0 == identity.identity()
        }) {
            return Err(E::InvalidContract);
        }
        previous = Some((identity.identity(), identity.version()));
        if !is_upper_name(identity.identity())
            || !is_exact_release_version(identity.version())
            || identity.encoding_version() != ENCODING_VERSION
        {
            return Err(E::InvalidContract);
        }
        check_count(identity.required_features().len(), limits.json.features())?;
        for text in [
            identity.identity(),
            identity.version(),
            identity.schema_version(),
            identity.encoding_version(),
        ]
        .into_iter()
        .chain(identity.required_features().iter().map(String::as_str))
        {
            budget.step(text.len() as u64)?;
            if text.len() as u64 > limits.json.string_bytes() {
                return Err(E::Limit);
            }
        }
        let legacy = identity.schema_version() == crate::PROJECT_VOCABULARY_SCHEMA_VERSION;
        if !(legacy && identity.required_features().is_empty()
            || identity.schema_version() == SCHEMA_VERSION
                && identity.required_features() == [REQUIRED_FEATURE])
        {
            return Err(E::InvalidContract);
        }
        dependencies(bundle, legacy, &mut budget)?;
        check_count(bundle.definitions.len(), limits.json.types())?;
        let mut name = None;
        for definition in &bundle.definitions {
            if name.is_some_and(|previous| previous >= definition.name.as_str()) {
                return Err(E::InvalidContract);
            }
            name = Some(definition.name.as_str());
            scope::preflight(definition, false, &mut budget)?;
            canonical_body(
                &definition.body,
                legacy,
                identity.identity(),
                identity.version(),
            )?;
        }
    }
    let mut roots = Vec::new();
    roots
        .try_reserve(bundles.len())
        .map_err(|_| E::Allocation)?;
    roots.extend(
        bundles
            .iter()
            .map(|bundle| (bundle.identity.identity(), bundle.identity.version())),
    );
    closure::validate(bundles, &roots, &mut budget)?;
    let mut normalized = Vec::new();
    normalized
        .try_reserve(bundles.len())
        .map_err(|_| E::Allocation)?;
    normalized.extend_from_slice(bundles);
    values::validate_defaults_policy(&mut normalized, &mut budget, true)?;
    if normalized != bundles {
        return Err(E::InvalidContract);
    }
    budget.step(1)?;
    Ok(ValidatedComposition {
        bundles: normalized,
    })
}

/// Validates canonical exact dependency metadata before any graph traversal.
fn dependencies(
    bundle: &CompositionBundle,
    legacy: bool,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    check_count(
        bundle.dependencies.len(),
        budget.limits.dependencies_per_bundle,
    )?;
    super::charge(
        &mut budget.edges,
        bundle.dependencies.len() as u64,
        budget.limits.dependency_edges,
    )?;
    if legacy && !bundle.dependencies.is_empty() {
        return Err(E::InvalidContract);
    }
    let mut previous = None;
    for dependency in &bundle.dependencies {
        budget.key(
            dependency.identity.len() + dependency.version.len(),
            bundle.dependencies.len(),
        )?;
        if dependency.identity.len() as u64 > budget.limits.json.string_bytes()
            || dependency.version.len() as u64 > budget.limits.json.string_bytes()
        {
            return Err(E::Limit);
        }
        if !is_upper_name(&dependency.identity)
            || !is_exact_release_version(&dependency.version)
            || dependency.identity == bundle.identity.identity()
            || previous.is_some_and(|owner| owner >= (&dependency.identity, &dependency.version))
        {
            return Err(E::InvalidDependency);
        }
        previous = Some((&dependency.identity, &dependency.version));
    }
    Ok(())
}

/// Checks canonical member order and the intentionally narrow explicit legacy-leaf adapter.
fn canonical_body(
    body: &CompositionBody,
    legacy: bool,
    identity: &str,
    version: &str,
) -> Result<(), E> {
    match body {
        CompositionBody::Record(fields) => {
            if fields.windows(2).any(|pair| pair[0].name >= pair[1].name) {
                return Err(E::InvalidContract);
            }
            for field in fields {
                if legacy
                    && (field.presence != FieldPresence::Required
                        || field.default.is_some()
                        || field.restrictions != FieldRestrictions::default()
                        || !matches!(&field.ty, T::Num | T::String | T::Bool | T::Url | T::Path)
                            && !matches!(&field.ty, T::VocabularyNominal { identity: i, version: v, .. } if i == identity && v == version))
                {
                    return Err(E::InvalidContract);
                }
            }
        }
        CompositionBody::Variant(alternatives) => {
            if legacy
                || alternatives
                    .windows(2)
                    .any(|pair| pair[0].tag >= pair[1].tag)
            {
                return Err(E::InvalidContract);
            }
        }
    }
    Ok(())
}
