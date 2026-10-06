// SPDX-License-Identifier: Apache-2.0

//! Shared contextual closed-default and restriction validation with bounded expansion.

use super::{Budget, CompositionError as E};
use neutral_ir::{
    ExactNumber,
    composition::{
        ClosedValue as V, CompositionBody, CompositionBundle, CompositionDefinition,
        CompositionField, FieldPresence, FieldRestrictions, compare_exact_numbers,
    },
    project_interface::ProjectPublicType as T,
};
use std::{cmp::Ordering, collections::BTreeMap};

/// Immutable lookup for contextual nominal owners while materializing defaults.
pub(super) type Catalogue<'a> = BTreeMap<(&'a str, &'a str, &'a str), &'a CompositionDefinition>;

/// Validates every restriction/default, even on private unused types, before publishing a catalogue.
pub(super) fn validate_defaults(
    bundles: &mut [CompositionBundle],
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let catalogue = bundles
        .iter()
        .flat_map(|bundle| {
            bundle.definitions.iter().map(move |definition| {
                (
                    (
                        bundle.identity.identity(),
                        bundle.identity.version(),
                        definition.name.as_str(),
                    ),
                    definition,
                )
            })
        })
        .collect::<Catalogue<'_>>();
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
                        let value = materialize(value, &field.ty, &catalogue, budget, 1)?;
                        check_restrictions(&restrictions, &value, budget)?;
                        Ok::<_, E>(value)
                    })
                    .transpose()?;
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
fn scalar_matches(value: &V, ty: &T) -> bool {
    matches!(
        (value, ty),
        (V::Number(_), T::Num)
            | (V::String(_), T::String)
            | (V::Bool(_), T::Bool)
            | (V::Url(_), T::Url)
            | (V::Path(_), T::Path)
    )
}

/// Compares compatible scalar choices in canonical order; malformed kinds are rejected earlier.
fn compare_scalars(left: &V, right: &V) -> Ordering {
    match (left, right) {
        (V::Number(a), V::Number(b)) => compare_exact_numbers(a, b),
        (V::String(a), V::String(b)) | (V::Url(a), V::Url(b)) | (V::Path(a), V::Path(b)) => {
            a.cmp(b)
        }
        (V::Bool(a), V::Bool(b)) => a.cmp(b),
        _ => unreachable!("choices were checked against one exact scalar type"),
    }
}

/// Returns bounded key work for an already verified scalar, for sort/search reservations.
fn scalar_size(value: &V) -> u64 {
    match value {
        V::Number(number) => number.coefficient().len() as u64,
        V::String(text) | V::Url(text) | V::Path(text) => text.len() as u64,
        _ => 1,
    }
}

/// Rejects contradictory/inapplicable bounds and canonicalizes finite scalar choices.
fn validate_restrictions(
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
    let mut result = raw.clone();
    if let Some(choices) = &mut result.choices {
        choices.sort_by(compare_scalars);
        if choices.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(E::DuplicateChoice);
        }
        let without_choices = FieldRestrictions {
            choices: None,
            minimum: raw.minimum.clone(),
            maximum: raw.maximum.clone(),
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
fn check_restrictions(
    restrictions: &FieldRestrictions,
    value: &V,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.step(1)?;
    if matches!(value, V::Null) {
        return Ok(());
    }
    if let V::Number(number) = value {
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
        V::String(text) | V::Url(text) | V::Path(text) => {
            budget.step(text.len() as u64)?;
            Some(text.chars().count() as u64)
        }
        V::List(values) => Some(values.len() as u64),
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
    budget.depth(depth, budget.limits.value_depth)?;
    budget.value_node()?;
    if let T::Nullable(inner) = ty {
        return if matches!(value, V::Null) {
            Ok(V::Null)
        } else {
            materialize(value, inner, catalogue, budget, depth + 1)
        };
    }
    if scalar_matches(value, ty) {
        budget.step(scalar_size(value))?;
        match value {
            V::Number(number)
                if number.coefficient().len() as u64 > budget.limits.json.numeric_digits
                    || number.scale().unsigned_abs() > budget.limits.json.numeric_scale =>
            {
                return Err(E::Limit);
            }
            V::String(text) | V::Url(text) | V::Path(text)
                if text.len() as u64 > budget.limits.json.string_bytes =>
            {
                return Err(E::Limit);
            }
            _ => {}
        }
        return Ok(value.clone());
    }
    match (value, ty) {
        (V::List(values), T::List(inner)) => {
            super::check_count(values.len(), budget.limits.json.array_items)?;
            super::check_count(
                values.len(),
                budget.limits.value_nodes.saturating_sub(budget.value_nodes),
            )?;
            budget.step(values.len() as u64)?;
            let mut result = Vec::new();
            result
                .try_reserve(values.len())
                .map_err(|_| E::Allocation)?;
            for value in values {
                result.push(materialize(value, inner, catalogue, budget, depth + 1)?);
            }
            Ok(V::List(result))
        }
        (
            _,
            T::VocabularyNominal {
                identity,
                version,
                name,
            },
        ) => {
            budget.key(identity.len() + version.len() + name.len(), catalogue.len())?;
            let definition = catalogue
                .get(&(identity.as_str(), version.as_str(), name.as_str()))
                .ok_or(E::UnknownType)?;
            match (&definition.body, value) {
                (CompositionBody::Record(fields), V::Record(values)) => {
                    materialize_record(values, fields, catalogue, budget, depth + 1)
                }
                (CompositionBody::Variant(alternatives), V::Variant { tag, payload }) => {
                    if tag.len() as u64 > budget.limits.json.string_bytes {
                        return Err(E::Limit);
                    }
                    budget.key(tag.len(), alternatives.len())?;
                    let index = alternatives
                        .binary_search_by(|alternative| alternative.tag.cmp(tag))
                        .map_err(|_| E::InvalidDefault)?;
                    let alternative = &alternatives[index];
                    Ok(V::Variant {
                        tag: tag.clone(),
                        payload: Box::new(materialize(
                            payload,
                            &alternative.ty,
                            catalogue,
                            budget,
                            depth + 1,
                        )?),
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
fn materialize_record(
    values: &[(String, Option<V>)],
    fields: &[CompositionField],
    catalogue: &Catalogue<'_>,
    budget: &mut Budget<'_>,
    depth: u64,
) -> Result<V, E> {
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
        if supplied.insert(name.as_str(), value).is_some()
            || fields
                .binary_search_by(|field| field.name.as_str().cmp(name))
                .is_err()
        {
            return Err(E::InvalidDefault);
        }
    }
    let mut result = Vec::new();
    result
        .try_reserve(fields.len())
        .map_err(|_| E::Allocation)?;
    for field in fields {
        budget.step(1)?;
        let restrictions = validate_restrictions(field, budget)?;
        let provided = supplied
            .get(field.name.as_str())
            .and_then(|value| value.as_ref());
        let value = match provided {
            Some(value) => Some(materialize(value, &field.ty, catalogue, budget, depth + 1)?),
            None => match field.presence {
                FieldPresence::Required => return Err(E::InvalidDefault),
                FieldPresence::Optional => {
                    budget.value_node()?;
                    None
                }
                FieldPresence::Defaulted => Some(materialize(
                    field.default.as_ref().ok_or(E::InvalidDefault)?,
                    &field.ty,
                    catalogue,
                    budget,
                    depth + 1,
                )?),
            },
        };
        if let Some(value) = &value {
            check_restrictions(&restrictions, value, budget)?;
        }
        result.push((field.name.clone(), value));
    }
    Ok(V::Record(result))
}
