// SPDX-License-Identifier: Apache-2.0

//! Shared contextual closed-default and restriction validation with bounded expansion.

use super::{Budget, CompositionError as E};
use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::{TryClone, boxed};
use neutral_core::ordered::OrderedMap as BTreeMap;
use neutral_ir::{
    ExactNumber, ModuleSymbolIdentity,
    composition::{
        ClosedValue as V, CompositionBody, CompositionBundle, CompositionDefinition,
        CompositionField, CompositionValue as W, FieldPresence, FieldRestrictions,
        compare_exact_numbers,
    },
    project_interface::ProjectPublicType as T,
};
use std::cmp::Ordering;

/// Immutable lookup shared by both declaration origins, with explicit depth semantics.
#[derive(Default)]
pub(super) struct Catalogue<'a> {
    /// Existing vocabulary tuples, with no aliases or source-owner conversions.
    vocabularies: BTreeMap<(&'a str, &'a str, &'a str), &'a CompositionDefinition>,
    /// Source module-symbol owners, populated only by the validated scope boundary.
    pub(super) sources: BTreeMap<&'a ModuleSymbolIdentity, &'a CompositionDefinition>,
    /// Project /2 counts occurrence depth, not nullable/nominal interpretation layers.
    pub(super) project_depth: bool,
}

impl<'a> Catalogue<'a> {
    /// Starts an empty standalone catalogue without selecting project /2.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Returns the total number of exact nominal owners across both origins.
    pub(super) fn len(&self) -> usize {
        self.vocabularies.len() + self.sources.len()
    }

    /// Inserts one vocabulary definition while preserving its exact owner tuple.
    pub(super) fn insert(
        &mut self,
        owner: (&'a str, &'a str, &'a str),
        definition: &'a CompositionDefinition,
    ) -> Result<(), E> {
        self.vocabularies
            .insert(owner, definition)
            .map_err(|_| E::Allocation)?;
        Ok(())
    }

    /// Looks up one exact vocabulary nominal without trying another origin.
    pub(super) fn get(&self, owner: &(&str, &str, &str)) -> Option<&'a CompositionDefinition> {
        self.vocabularies.get(owner).copied()
    }

    /// Resolves a nominal while charging key inspection before ordered lookup.
    pub(super) fn resolve(
        &self,
        ty: &T,
        budget: &mut Budget<'_>,
    ) -> Result<&'a CompositionDefinition, E> {
        match ty {
            T::Nominal(owner) => {
                budget.key(
                    owner.module().module_name().len() + owner.declaration_name().len(),
                    self.len(),
                )?;
                self.sources.get(owner).copied().ok_or(E::UnknownType)
            }
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                budget.key(identity.len() + version.len() + name.len(), self.len())?;
                self.get(&(identity, version, name)).ok_or(E::UnknownType)
            }
            _ => Err(E::UnknownType),
        }
    }
}

/// Enumerates both definition kinds without allocating a temporary type vector.
pub(super) fn definition_types(definition: &CompositionDefinition) -> impl Iterator<Item = &T> {
    let (fields, alternatives) = match &definition.body {
        CompositionBody::Record(fields) => (fields.as_slice(), &[][..]),
        CompositionBody::Variant(alternatives) => (&[][..], alternatives.as_slice()),
    };
    fields
        .iter()
        .map(|field| &field.ty)
        .chain(alternatives.iter().map(|alternative| &alternative.ty))
}

/// Validates every restriction/default, even on private unused types, before publishing a catalogue.
pub(super) fn validate_defaults(
    bundles: &mut [CompositionBundle],
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    validate_defaults_policy(bundles, budget, false)
}

/// Validates every unused default with an explicitly selected occurrence-depth policy.
pub(super) fn validate_defaults_policy(
    bundles: &mut [CompositionBundle],
    budget: &mut Budget<'_>,
    project_depth: bool,
) -> Result<(), E> {
    let mut catalogue = Catalogue::new();
    for bundle in bundles.iter() {
        for definition in &bundle.definitions {
            budget.step(catalogue.len() as u64 + 1)?;
            catalogue.insert(
                (
                    bundle.identity.identity(),
                    bundle.identity.version(),
                    definition.name.as_str(),
                ),
                definition,
            )?;
        }
    }
    catalogue.project_depth = project_depth;
    let mut finalized = Vec::new();
    for (bundle_index, bundle) in bundles.iter().enumerate() {
        for (definition_index, definition) in bundle.definitions.iter().enumerate() {
            let CompositionBody::Record(fields) = &definition.body else {
                continue;
            };
            for (field_index, field) in fields.iter().enumerate() {
                budget.step(1)?;
                let restrictions = validate_restrictions(field, budget)?;
                let default = field
                    .default
                    .as_ref()
                    .map(|value| {
                        let value = materialize(
                            value,
                            &field.ty,
                            &catalogue,
                            budget,
                            u64::from(!project_depth),
                        )?;
                        check_restrictions(&restrictions, &value, budget)?;
                        Ok::<_, E>(value)
                    })
                    .transpose()?;
                finalized.try_retain(1).map_err(|_| E::Allocation)?;
                finalized.push((
                    bundle_index,
                    definition_index,
                    field_index,
                    restrictions,
                    default,
                ));
            }
        }
    }
    // Publication happens only after all defaults validated against the same
    // immutable complete catalogue; source order cannot influence expansion.
    for (bundle, definition, field, restrictions, default) in finalized {
        if let CompositionBody::Record(fields) = &mut bundles[bundle].definitions[definition].body {
            fields[field].restrictions = restrictions;
            fields[field].default = default;
        }
    }
    Ok(())
}

/// Charges numeric comparison by coefficient size, never by exponent magnitude.
fn compare_numbers(
    left: &ExactNumber,
    right: &ExactNumber,
    budget: &mut Budget<'_>,
) -> Result<Ordering, E> {
    budget.step(left.coefficient().len().max(right.coefficient().len()) as u64)?;
    Ok(compare_exact_numbers(left, right))
}

/// Returns exact scalar compatibility without structural coercion or inert-text interpretation.
fn scalar_matches<R>(value: &W<R>, ty: &T) -> bool {
    matches!(
        (value, ty),
        (W::Number(_), T::Num)
            | (W::String(_), T::String)
            | (W::Bool(_), T::Bool)
            | (W::Url(_), T::Url)
            | (W::Path(_), T::Path)
    )
}

/// Compares compatible scalar choices in canonical order; malformed kinds are rejected earlier.
fn compare_scalars<L, R>(left: &W<L>, right: &W<R>) -> Ordering {
    neutral_ir::composition::compare_composition_scalars(left, right)
        .expect("choices were checked against one exact scalar type")
}

/// Returns bounded key work for an already verified scalar, for sort/search reservations.
fn scalar_size<R>(value: &W<R>) -> u64 {
    match value {
        W::Number(number) => number.coefficient().len() as u64,
        W::String(text) | W::Url(text) | W::Path(text) => text.len() as u64,
        _ => 1,
    }
}

/// Checks one field's declarative restrictions using the common exact comparison engine.
///
/// This diagnostic helper does not establish type, reference, scope or project
/// authority. Complete materialization must still pass the shared binding boundary.
///
/// # Errors
/// Rejects invalid restrictions, violated choices/bounds, limits or cancellation.
pub fn check_composition_field_restrictions(
    field: &CompositionField,
    value: &neutral_ir::composition::BindingValue,
    limits: super::CompositionLimits,
    cancellation: &neutral_core::CancellationToken,
) -> Result<(), E> {
    limits.validate()?;
    let mut budget = Budget::new(limits, cancellation);
    super::scope::preflight_type(&field.ty, &mut budget)?;
    super::scope::preflight_restrictions(field, &mut budget)?;
    let restrictions = validate_restrictions(field, &mut budget)?;
    let ty = if let T::Nullable(inner) = &field.ty {
        inner.as_ref()
    } else {
        &field.ty
    };
    if !matches!(value, W::Null)
        && !scalar_matches(value, ty)
        && !matches!((value, ty), (W::List(_), T::List(_)))
    {
        return Err(E::InvalidValue);
    }
    check_restrictions(&restrictions, value, &mut budget).map_err(|e| {
        if e == E::InvalidDefault {
            E::InvalidValue
        } else {
            e
        }
    })
}

/// Rejects contradictory/inapplicable bounds and canonicalizes finite scalar choices.
pub(super) fn validate_restrictions(
    field: &CompositionField,
    budget: &mut Budget<'_>,
) -> Result<FieldRestrictions, E> {
    let ty = if let T::Nullable(inner) = &field.ty {
        inner.as_ref()
    } else {
        &field.ty
    };
    let raw = &field.restrictions;
    if (raw.minimum.is_some() || raw.maximum.is_some()) && !matches!(ty, T::Num) {
        return Err(E::InvalidRestrictions);
    }
    if (raw.min_length.is_some() || raw.max_length.is_some())
        && !matches!(ty, T::String | T::Url | T::Path | T::List(_))
    {
        return Err(E::InvalidRestrictions);
    }
    if let (Some(min), Some(max)) = (&raw.minimum, &raw.maximum)
        && compare_numbers(min, max, budget)? == Ordering::Greater
    {
        return Err(E::InvalidRestrictions);
    }
    if raw
        .min_length
        .zip(raw.max_length)
        .is_some_and(|(min, max)| min > max)
    {
        return Err(E::InvalidRestrictions);
    }
    if let Some(choices) = &raw.choices {
        budget.step(choices.len() as u64)?;
        if choices.is_empty() || choices.iter().any(|value| !scalar_matches(value, ty)) {
            return Err(E::InvalidRestrictions);
        }
        // Pay a conservative comparison/key-inspection budget before sorting or
        // cloning unusually long choice keys. Exponents never expand a key.
        let per_key = choices.iter().map(scalar_size).max().unwrap_or(1).max(1);
        let sorting = (choices.len() as u64)
            .checked_mul(per_key)
            .and_then(|n| n.checked_mul(u64::from(choices.len().ilog2()) + 1))
            .and_then(|n| n.checked_mul(4))
            .ok_or(E::Limit)?;
        budget.step(sorting)?;
    }
    let mut result = raw.try_clone().map_err(|_| E::Allocation)?;
    if let Some(choices) = &mut result.choices {
        choices.sort_unstable_by(compare_scalars);
        if choices.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(E::DuplicateChoice);
        }
        let without_choices = FieldRestrictions {
            choices: None,
            minimum: raw.minimum.try_clone().map_err(|_| E::Allocation)?,
            maximum: raw.maximum.try_clone().map_err(|_| E::Allocation)?,
            min_length: raw.min_length,
            max_length: raw.max_length,
        };
        for choice in choices {
            check_restrictions(&without_choices, choice, budget).map_err(|error| {
                if error == E::InvalidDefault {
                    E::InvalidRestrictions
                } else {
                    error
                }
            })?;
        }
    }
    Ok(result)
}

/// Checks an already typed present value against inclusive bounds and finite choices.
pub(super) fn check_restrictions<R>(
    restrictions: &FieldRestrictions,
    value: &W<R>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(1)?;
    if matches!(value, W::Null) {
        return Ok(());
    }
    if let W::Number(number) = value {
        if let Some(min) = &restrictions.minimum
            && compare_numbers(number, min, budget)? == Ordering::Less
        {
            return Err(E::InvalidDefault);
        }
        if let Some(max) = &restrictions.maximum
            && compare_numbers(number, max, budget)? == Ordering::Greater
        {
            return Err(E::InvalidDefault);
        }
    }
    let length = match value {
        W::String(text) | W::Url(text) | W::Path(text) => {
            budget.step(text.len() as u64)?;
            Some(text.chars().count() as u64)
        }
        W::List(values) => Some(values.len() as u64),
        _ => None,
    };
    if let Some(length) = length
        && (restrictions.min_length.is_some_and(|min| length < min)
            || restrictions.max_length.is_some_and(|max| length > max))
    {
        return Err(E::InvalidDefault);
    }
    if let Some(choices) = &restrictions.choices {
        let per_key = choices
            .iter()
            .map(scalar_size)
            .max()
            .unwrap_or(1)
            .max(scalar_size(value))
            .max(1);
        budget.step(
            per_key
                .checked_mul(u64::from(choices.len().ilog2()) + 1)
                .ok_or(E::Limit)?,
        )?;
        if choices
            .binary_search_by(|choice| compare_scalars(choice, value))
            .is_err()
        {
            return Err(E::InvalidDefault);
        }
    }
    Ok(())
}

/// Materializes one closed default under its expected type and bounded nominal expansion.
pub(super) fn materialize(
    value: &V,
    ty: &T,
    catalogue: &Catalogue<'_>,
    budget: &mut Budget<'_>,
    depth: u64,
) -> Result<V, E> {
    materialize_with(value, ty, catalogue, budget, depth, &ClosedReferences)
}

/// Checks identity-only reference targets without evaluating or embedding their values.
pub(super) trait ReferencePolicy<R> {
    /// Verifies the exact invariant target type, visibility and lookup budget.
    fn check(&self, target: &R, expected: &T, budget: &mut Budget<'_>) -> Result<(), E>;
}

/// Closed defaults cannot construct a reference, including through nested defaults.
struct ClosedReferences;

impl ReferencePolicy<std::convert::Infallible> for ClosedReferences {
    /// Eliminates an impossible closed reference without adding a runtime escape hatch.
    fn check(&self, target: &std::convert::Infallible, _: &T, _: &mut Budget<'_>) -> Result<(), E> {
        match *target {}
    }
}

/// Materializes both closed and binding values with one contextual/default/restriction engine.
pub(super) fn materialize_with<R: TryClone>(
    value: &W<R>,
    ty: &T,
    catalogue: &Catalogue<'_>,
    budget: &mut Budget<'_>,
    depth: u64,
    references: &impl ReferencePolicy<R>,
) -> Result<W<R>, E> {
    budget.depth(depth, budget.limits.value_depth)?;
    budget.value_node()?;
    if let T::Nullable(inner) = ty {
        return if matches!(value, W::Null) {
            Ok(W::Null)
        } else {
            materialize_with(
                value,
                inner,
                catalogue,
                budget,
                depth + u64::from(!catalogue.project_depth),
                references,
            )
        };
    }
    if scalar_matches(value, ty) {
        budget.step(scalar_size(value))?;
        match value {
            W::Number(number)
                if number.coefficient().len() as u64 > budget.limits.json.numeric_digits
                    || number.scale().unsigned_abs() > budget.limits.json.numeric_scale =>
            {
                return Err(E::Limit);
            }
            W::String(text) | W::Url(text) | W::Path(text)
                if text.len() as u64 > budget.limits.json.string_bytes =>
            {
                return Err(E::Limit);
            }
            _ => {}
        }
        return value.try_clone().map_err(|_| E::Allocation);
    }
    match (value, ty) {
        (W::Reference(target), T::Ref(inner)) => {
            references.check(target, inner, budget)?;
            Ok(W::Reference(target.try_clone().map_err(|_| E::Allocation)?))
        }
        (W::List(values), T::List(inner)) => {
            super::check_count(values.len(), budget.limits.json.array_items)?;
            super::check_count(
                values.len(),
                budget.limits.value_nodes.saturating_sub(budget.value_nodes),
            )?;
            budget.step(values.len() as u64)?;
            let mut result = Vec::new();
            result.try_retain(values.len()).map_err(|_| E::Allocation)?;
            for value in values {
                result.push(materialize_with(
                    value,
                    inner,
                    catalogue,
                    budget,
                    depth + 1,
                    references,
                )?);
            }
            Ok(W::List(result))
        }
        (_, T::VocabularyNominal { .. } | T::Nominal(_)) => {
            let definition = catalogue.resolve(ty, budget)?;
            match (&definition.body, value) {
                (CompositionBody::Record(fields), W::Record(values)) => materialize_record(
                    values,
                    fields,
                    catalogue,
                    budget,
                    depth + u64::from(!catalogue.project_depth),
                    references,
                ),
                (CompositionBody::Variant(alternatives), W::Variant { tag, payload }) => {
                    if tag.len() as u64 > budget.limits.json.string_bytes {
                        return Err(E::Limit);
                    }
                    budget.key(tag.len(), alternatives.len())?;
                    let index = alternatives
                        .binary_search_by(|alternative| alternative.tag.cmp(tag))
                        .map_err(|_| E::InvalidDefault)?;
                    let alternative = &alternatives[index];
                    Ok(W::Variant {
                        tag: tag.try_clone().map_err(|_| E::Allocation)?,
                        payload: boxed(materialize_with(
                            payload,
                            &alternative.ty,
                            catalogue,
                            budget,
                            depth + 1,
                            references,
                        )?)
                        .map_err(|_| E::Allocation)?,
                    })
                }
                _ => Err(E::InvalidDefault),
            }
        }
        // A closed default has no reference representation. Nullable Ref null
        // was handled above and never creates a target edge.
        _ => Err(E::InvalidDefault),
    }
}

/// Applies required/optional/defaulted field policies without conflating absence and null.
fn materialize_record<R: TryClone>(
    values: &[(String, Option<W<R>>)],
    fields: &[CompositionField],
    catalogue: &Catalogue<'_>,
    budget: &mut Budget<'_>,
    depth: u64,
    references: &impl ReferencePolicy<R>,
) -> Result<W<R>, E> {
    budget.depth(depth, budget.limits.value_depth)?;
    super::check_count(values.len(), budget.limits.json.fields)?;
    super::check_count(fields.len(), budget.limits.json.fields)?;
    super::check_count(
        fields.len(),
        budget.limits.value_nodes.saturating_sub(budget.value_nodes),
    )?;
    budget.step(values.len() as u64 + fields.len() as u64)?;
    let mut supplied = BTreeMap::new();
    for (name, value) in values {
        budget.step(1)?;
        budget.key(name.len(), values.len())?;
        budget.step(supplied.len() as u64)?;
        if supplied
            .insert(name.as_str(), value)
            .map_err(|_| E::Allocation)?
            .is_some()
            || fields
                .binary_search_by(|field| field.name.as_str().cmp(name))
                .is_err()
        {
            return Err(E::InvalidDefault);
        }
    }
    let mut result = Vec::new();
    result.try_retain(fields.len()).map_err(|_| E::Allocation)?;
    for field in fields {
        budget.step(1)?;
        let restrictions = validate_restrictions(field, budget)?;
        let provided = supplied
            .get(field.name.as_str())
            .and_then(|value| value.as_ref());
        let value = match provided {
            Some(value) => Some(materialize_with(
                value,
                &field.ty,
                catalogue,
                budget,
                depth + 1,
                references,
            )?),
            None => match field.presence {
                FieldPresence::Required => return Err(E::InvalidDefault),
                FieldPresence::Optional => {
                    budget.value_node()?;
                    None
                }
                FieldPresence::Defaulted => {
                    let default = lift_closed(
                        field.default.as_ref().ok_or(E::InvalidDefault)?,
                        budget,
                        depth + 1,
                    )?;
                    Some(materialize_with(
                        &default,
                        &field.ty,
                        catalogue,
                        budget,
                        depth + 1,
                        references,
                    )?)
                }
            },
        };
        if let Some(value) = &value {
            check_restrictions(&restrictions, value, budget)?;
        }
        result.push((field.name.try_clone().map_err(|_| E::Allocation)?, value));
    }
    Ok(W::Record(result))
}

/// Copies a closed default into a reference-capable representation without inventing a target.
fn lift_closed<R>(value: &V, budget: &mut Budget<'_>, depth: u64) -> Result<W<R>, E> {
    budget.depth(depth, budget.limits.value_depth)?;
    budget.step(scalar_size(value))?;
    Ok(match value {
        V::Number(value) => W::Number(value.try_clone().map_err(|_| E::Allocation)?),
        V::String(value) => W::String(value.try_clone().map_err(|_| E::Allocation)?),
        V::Bool(value) => W::Bool(*value),
        V::Url(value) => W::Url(value.try_clone().map_err(|_| E::Allocation)?),
        V::Path(value) => W::Path(value.try_clone().map_err(|_| E::Allocation)?),
        V::Null => W::Null,
        V::Reference(never) => match *never {},
        V::List(items) => {
            super::check_count(items.len(), budget.limits.json.array_items())?;
            let mut result = Vec::new();
            result.try_retain(items.len()).map_err(|_| E::Allocation)?;
            for item in items {
                result.push(lift_closed(item, budget, depth + 1)?);
            }
            W::List(result)
        }
        V::Record(fields) => {
            super::check_count(fields.len(), budget.limits.json.fields())?;
            let mut result = Vec::new();
            result.try_retain(fields.len()).map_err(|_| E::Allocation)?;
            for (name, value) in fields {
                budget.step(name.len() as u64)?;
                result.push((
                    name.try_clone().map_err(|_| E::Allocation)?,
                    value
                        .as_ref()
                        .map(|value| lift_closed(value, budget, depth + 1))
                        .transpose()?,
                ));
            }
            W::Record(result)
        }
        V::Variant { tag, payload } => {
            budget.step(tag.len() as u64)?;
            W::Variant {
                tag: tag.try_clone().map_err(|_| E::Allocation)?,
                payload: boxed(lift_closed(payload, budget, depth + 1)?)
                    .map_err(|_| E::Allocation)?,
            }
        }
    })
}
