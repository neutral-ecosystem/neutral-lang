// SPDX-License-Identifier: Apache-2.0

//! Contextual closed-value materialization and safe, non-semantic occurrence facts.

use super::{Budget, CompositionError as E, CompositionLimits, ValidatedComposition, values};
use neutral_core::CancellationToken;
use neutral_core::allocation::RetainCapacity;
use neutral_core::ordered::OrderedMap as BTreeMap;
use neutral_ir::{
    composition::{
        ClosedValue, CompositionValue as V, ValueOrigin, ValueOriginKind as K,
        ValuePathSegment as P,
    },
    project_interface::ProjectPublicType as T,
};

/// Complete immutable value plus origin classifications, never a compiled project artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedCompositionValue {
    /// Canonical materialized meaning, separate from occurrence evidence.
    pub(super) value: ClosedValue,
    /// Ordered safe paths; no invented source span or host information.
    pub(super) origins: Vec<ValueOrigin>,
}

impl ValidatedCompositionValue {
    /// Returns immutable materialized meaning; compare this rather than origins for value equality.
    #[must_use]
    pub fn value(&self) -> &ClosedValue {
        &self.value
    }

    /// Returns canonical root/field/list/payload occurrence facts without source attribution claims.
    #[must_use]
    pub fn origins(&self) -> &[ValueOrigin] {
        &self.origins
    }
}

/// Materializes a closed supplied value against one exact public vocabulary nominal contract.
///
/// Uses the same validator as bundle defaults, including every nested restriction,
/// selected variant payload and omission rule. References cannot be supplied as
/// closed values; nullable-reference null never creates a binding edge. This API
/// does not acquire input, parse source, or select a project/wire/identity profile.
///
/// # Errors
/// Returns a safe type/value/limit/cancellation/allocation classification and no partial result.
pub fn materialize_composition_value(
    catalogue: &ValidatedComposition,
    owner: (&str, &str, &str),
    supplied: &ClosedValue,
    limits: CompositionLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedCompositionValue, E> {
    limits.validate()?;
    let mut budget = Budget::new(limits, cancellation);
    budget.step(1)?;
    // Public callers cannot use this boundary to materialize a private root.
    // All its reachable public type closure was validated before publication.
    let mut lookup = values::Catalogue::new();
    for bundle in &catalogue.bundles {
        for definition in &bundle.definitions {
            budget.key(
                bundle.identity.identity().len()
                    + bundle.identity.version().len()
                    + definition.name.len(),
                lookup.len() + 1,
            )?;
            budget.step(lookup.len() as u64)?;
            lookup.insert(
                (
                    bundle.identity.identity(),
                    bundle.identity.version(),
                    definition.name.as_str(),
                ),
                definition,
            )?;
        }
    }
    budget.key(owner.0.len() + owner.1.len() + owner.2.len(), lookup.len())?;
    let root = lookup.get(&owner).ok_or(E::UnknownType)?;
    if !root.public {
        return Err(E::PrivateType);
    }
    let ty = T::VocabularyNominal {
        identity: super::copy::text(owner.0)?,
        version: super::copy::text(owner.1)?,
        name: super::copy::text(owner.2)?,
    };
    let value = values::materialize(supplied, &ty, &lookup, &mut budget, 1).map_err(|error| {
        if error == E::InvalidDefault {
            E::InvalidValue
        } else {
            error
        }
    })?;
    let mut origins = Vec::new();
    classify(
        Some(supplied),
        Some(&value),
        false,
        &mut Vec::new(),
        &mut origins,
        &mut budget,
    )?;
    budget.step(1)?;
    Ok(ValidatedCompositionValue { value, origins })
}

/// Classifies canonical final occurrences without equating optional absence with explicit null.
pub(super) fn classify<R>(
    raw: Option<&V<R>>,
    value: Option<&V<R>>,
    inherited_default: bool,
    path: &mut Vec<P>,
    origins: &mut Vec<ValueOrigin>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.depth(
        path.len() as u64 + budget.origin_root_depth,
        budget.limits.value_depth,
    )?;
    super::check_count(origins.len() + 1, budget.limits.value_nodes)?;
    let defaulted = inherited_default || (raw.is_none() && value.is_some());
    let kind = if value.is_none() {
        K::OmittedOptional
    } else if defaulted {
        K::Defaulted
    } else if matches!(value, Some(V::Null)) {
        K::ExplicitNull
    } else {
        K::Supplied
    };
    let path_bytes = path.iter().try_fold(1_u64, |total, segment| {
        total
            .checked_add(match segment {
                P::Field(name) => name.len() as u64,
                _ => 1,
            })
            .ok_or(E::Limit)
    })?;
    budget.step(path_bytes)?;
    origins.try_retain(1).map_err(|_| E::Allocation)?;
    origins.push(ValueOrigin {
        path: super::copy::path(path)?,
        kind,
    });
    match value {
        Some(V::Record(fields)) => {
            let raw_fields = if let Some(V::Record(fields)) = raw {
                fields.as_slice()
            } else {
                &[]
            };
            let mut provided = BTreeMap::new();
            for (name, value) in raw_fields {
                budget.key(name.len(), raw_fields.len())?;
                budget.step(provided.len() as u64)?;
                provided
                    .insert(name.as_str(), value.as_ref())
                    .map_err(|_| E::Allocation)?;
            }
            for (name, value) in fields {
                budget.key(name.len(), raw_fields.len())?;
                descend(
                    P::Field(super::copy::text(name)?),
                    provided.get(name.as_str()).copied().flatten(),
                    value.as_ref(),
                    defaulted,
                    path,
                    origins,
                    budget,
                )?;
            }
        }
        Some(V::List(items)) => {
            let raw_items = if let Some(V::List(items)) = raw {
                items.as_slice()
            } else {
                &[]
            };
            for (index, value) in items.iter().enumerate() {
                descend(
                    P::Element(index as u64),
                    raw_items.get(index),
                    Some(value),
                    defaulted,
                    path,
                    origins,
                    budget,
                )?;
            }
        }
        Some(V::Variant { payload, .. }) => {
            let raw_payload = if let Some(V::Variant { payload, .. }) = raw {
                Some(payload.as_ref())
            } else {
                None
            };
            descend(
                P::Payload,
                raw_payload,
                Some(payload),
                defaulted,
                path,
                origins,
                budget,
            )?;
        }
        _ => {}
    }
    Ok(())
}

/// Pushes one safe path segment and restores the caller path even when nested classification fails.
fn descend<R>(
    segment: P,
    raw: Option<&V<R>>,
    value: Option<&V<R>>,
    defaulted: bool,
    path: &mut Vec<P>,
    origins: &mut Vec<ValueOrigin>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    path.try_retain(1).map_err(|_| E::Allocation)?;
    path.push(segment);
    let result = classify(raw, value, defaulted, path, origins, budget);
    path.pop();
    result
}
