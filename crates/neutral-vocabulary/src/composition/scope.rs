// SPDX-License-Identifier: Apache-2.0

//! Shared resolved declaration validation; source parsing and project artifacts remain separate.

use super::{
    Budget, CompositionError as E, CompositionLimits, ValidatedComposition,
    ValidatedCompositionValue, charge, check_count, supplied, values,
};
use neutral_core::allocation::Shared as Arc;
use neutral_core::ordered::{OrderedMap as BTreeMap, OrderedSet as BTreeSet};
use neutral_core::{CancellationToken, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{
        ClosedValue as V, CompositionBody, CompositionDefinition, FieldPresence,
        SourceCompositionDefinition,
    },
    language::{is_protected_name, is_snake_name, is_upper_name},
    project_interface::ProjectPublicType as T,
};

/// Immutable resolved contracts for both source and vocabulary nominal owners.
///
/// This is a type/default boundary, not compiled project IR. It cannot establish
/// source syntax, import visibility, source attribution, binding reference targets,
/// wire compatibility or identity. Those require the explicit compiler boundary.
#[derive(Clone, Eq, PartialEq)]
pub struct ValidatedCompositionScope {
    /// Exact, already validated transitive vocabulary closure.
    catalogue: Arc<ValidatedComposition>,
    /// Canonical source-owned record/variant contracts, never mutably exposed.
    sources: Vec<SourceCompositionDefinition>,
}

impl std::fmt::Debug for ValidatedCompositionScope {
    /// Reports counts rather than private contracts, defaults or source owner names.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValidatedCompositionScope")
            .field("vocabularies", &self.catalogue.bundles().len())
            .field("source_types", &self.sources.len())
            .finish_non_exhaustive()
    }
}

impl ValidatedCompositionScope {
    /// Returns the immutable vocabulary catalogue with exact semantic owners and locks.
    #[must_use]
    pub fn catalogue(&self) -> &Arc<ValidatedComposition> {
        &self.catalogue
    }

    /// Returns complete source contracts for trusted producers; public readers must filter visibility.
    #[must_use]
    pub fn sources(&self) -> &[SourceCompositionDefinition] {
        &self.sources
    }

    /// Materializes closed data under either exact public nominal origin using one value engine.
    ///
    /// Project /2 depth counts root zero and record/list/variant occurrences, not
    /// nullable checks or nominal lookup. Closed data cannot supply source reuse
    /// or a non-null reference. Missing and private roots are indistinguishable.
    ///
    /// # Errors
    /// Returns a safe unknown-type/value/limit/cancellation failure without partial origins.
    pub fn materialize(
        &self,
        owner: &T,
        value: &V,
        limits: CompositionLimits,
        cancellation: &CancellationToken,
    ) -> Result<ValidatedCompositionValue, E> {
        limits.validate()?;
        let mut budget = Budget::new(limits, cancellation);
        budget.origin_root_depth = 0;
        budget.step(1)?;
        let lookup = lookup(&self.catalogue, &self.sources, &mut budget)?;
        let definition = lookup.resolve(owner, &mut budget)?;
        if !definition.public {
            return Err(E::UnknownType);
        }
        let result =
            values::materialize(value, owner, &lookup, &mut budget, 0).map_err(|error| {
                if error == E::InvalidDefault {
                    E::InvalidValue
                } else {
                    error
                }
            })?;
        let mut origins = Vec::new();
        supplied::classify(
            Some(value),
            Some(&result),
            false,
            &mut Vec::new(),
            &mut origins,
            &mut budget,
        )?;
        budget.step(1)?;
        Ok(ValidatedCompositionValue {
            value: result,
            origins,
        })
    }
}

/// Validates resolved source declarations against the exact locked vocabulary catalogue.
///
/// All definitions, including private unused types and unselected alternatives,
/// participate in closure and aggregate bounds. References are non-embedding;
/// lists and nullable wrappers never hide an embedded cycle. Source fields allow
/// required/defaulted presence only; optional fields belong to vocabulary contracts.
/// Raw source owners cannot be created by a vocabulary, and aliases never occur here.
/// The previously validated catalogue remains responsible for captured bytes,
/// exact locks and dependency depth; this boundary does not recapture bundles.
///
/// # Errors
/// Rejects invalid owners/shapes/defaults/restrictions, private/dangling type edges,
/// embedded cycles, exhausted independent bounds or cancellation atomically.
pub fn validate_composition_scope(
    catalogue: Arc<ValidatedComposition>,
    mut sources: Vec<SourceCompositionDefinition>,
    limits: CompositionLimits,
    cancellation: &CancellationToken,
) -> Result<ValidatedCompositionScope, E> {
    limits.validate()?;
    let mut budget = Budget::new(limits, cancellation);
    budget.step(1)?;
    check_count(catalogue.bundles().len(), limits.bundles)?;
    check_count(sources.len(), limits.total_types)?;
    let mut module_types = BTreeMap::new();
    for source in &sources {
        validate_owner(&source.owner, &mut budget)?;
        budget.step(module_types.len() as u64)?;
        charge(
            module_types
                .entry(source.owner.module())
                .map_err(|_| E::Allocation)?
                .or_insert(0),
            1,
            limits.json.types(),
        )?;
        if source.owner.declaration_name() != source.definition.name {
            return Err(E::InvalidContract);
        }
        preflight(&source.definition, true, &mut budget)?;
    }
    for bundle in catalogue.bundles() {
        check_count(bundle.definitions.len(), limits.json.types())?;
        charge(
            &mut budget.edges,
            bundle.dependencies.len() as u64,
            limits.dependency_edges,
        )?;
        check_count(bundle.dependencies.len(), limits.dependencies_per_bundle)?;
        for definition in &bundle.definitions {
            preflight(definition, false, &mut budget)?;
        }
    }
    // Charge comparison work before sorting attacker-controlled owner/member strings.
    for source in &sources {
        budget.key(
            source.owner.module().module_name().len() + source.definition.name.len(),
            sources.len(),
        )?;
    }
    sources.sort_unstable_by(|a, b| a.owner.cmp(&b.owner));
    if sources
        .windows(2)
        .any(|pair| pair[0].owner == pair[1].owner)
    {
        return Err(E::InvalidContract);
    }
    for source in &mut sources {
        match &mut source.definition.body {
            CompositionBody::Record(fields) => fields.sort_unstable_by(|a, b| a.name.cmp(&b.name)),
            CompositionBody::Variant(alternatives) => {
                alternatives.sort_unstable_by(|a, b| a.tag.cmp(&b.tag));
            }
        }
    }
    let contracts = lookup(&catalogue, &sources, &mut budget)?;
    validate_edges(&sources, &contracts, &mut budget)?;
    // Check all defaults against the same complete, immutable scope before publishing
    // normalized source fields. A declaration's position cannot change its meaning.
    let mut finalized = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let CompositionBody::Record(fields) = &source.definition.body else {
            continue;
        };
        for (field_index, field) in fields.iter().enumerate() {
            let restrictions = values::validate_restrictions(field, &mut budget)?;
            let default = field
                .default
                .as_ref()
                .map(|value| {
                    let value = values::materialize(value, &field.ty, &contracts, &mut budget, 0)?;
                    values::check_restrictions(&restrictions, &value, &mut budget)?;
                    Ok::<_, E>(value)
                })
                .transpose()?;
            finalized.try_reserve(1).map_err(|_| E::Allocation)?;
            finalized.push((index, field_index, restrictions, default));
        }
    }
    // Recheck vocabulary defaults under this call's independent policy, too.
    for bundle in catalogue.bundles() {
        for definition in &bundle.definitions {
            if let CompositionBody::Record(fields) = &definition.body {
                for field in fields {
                    let restrictions = values::validate_restrictions(field, &mut budget)?;
                    if let Some(default) = &field.default {
                        let value =
                            values::materialize(default, &field.ty, &contracts, &mut budget, 0)?;
                        values::check_restrictions(&restrictions, &value, &mut budget)?;
                    }
                }
            }
        }
    }
    for (source, field, restrictions, default) in finalized {
        if let CompositionBody::Record(fields) = &mut sources[source].definition.body {
            fields[field].restrictions = restrictions;
            fields[field].default = default;
        }
    }
    budget.step(1)?;
    Ok(ValidatedCompositionScope { catalogue, sources })
}

/// Bounds and validates raw source owner components before map ordering or cloning.
fn validate_owner(owner: &ModuleSymbolIdentity, budget: &mut Budget<'_>) -> Result<(), E> {
    let module = owner.module();
    budget.key(
        module.module_name().len() + owner.declaration_name().len(),
        1,
    )?;
    if module.module_name().len() as u64 > budget.limits.json.string_bytes()
        || owner.declaration_name().len() as u64 > budget.limits.json.string_bytes()
    {
        return Err(E::Limit);
    }
    if module.language_behavior_version() != V1_SOURCE_PROFILE
        || !module.module_name().split("::").all(is_snake_name)
        || !is_upper_name(owner.declaration_name())
        || is_protected_name(owner.declaration_name())
    {
        return Err(E::InvalidContract);
    }
    Ok(())
}

/// Validates keys, presence and independent aggregate counts before retained-value copies.
pub(super) fn preflight(
    definition: &CompositionDefinition,
    source: bool,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    charge(&mut budget.types, 1, budget.limits.total_types)?;
    budget.step(definition.name.len() as u64)?;
    if definition.name.len() as u64 > budget.limits.json.string_bytes() {
        return Err(E::Limit);
    }
    if !is_upper_name(&definition.name) || is_protected_name(&definition.name) {
        return Err(E::InvalidContract);
    }
    match &definition.body {
        CompositionBody::Record(fields) => {
            check_count(fields.len(), budget.limits.json.fields())?;
            charge(
                &mut budget.fields,
                fields.len() as u64,
                budget.limits.total_fields,
            )?;
            let mut names = BTreeSet::new();
            for field in fields {
                member(&field.name, fields.len(), &mut names, budget)?;
                if (source && field.presence == FieldPresence::Optional)
                    || (field.presence == FieldPresence::Defaulted) != field.default.is_some()
                {
                    return Err(E::InvalidContract);
                }
                preflight_type(&field.ty, budget)?;
                preflight_restrictions(field, budget)?;
                if let Some(default) = &field.default {
                    preflight_value(default, budget)?;
                }
            }
        }
        CompositionBody::Variant(alternatives) => {
            if alternatives.is_empty() {
                return Err(E::InvalidContract);
            }
            check_count(alternatives.len(), budget.limits.alternatives_per_type)?;
            charge(
                &mut budget.alternatives,
                alternatives.len() as u64,
                budget.limits.total_alternatives,
            )?;
            let mut tags = BTreeSet::new();
            for alternative in alternatives {
                member(&alternative.tag, alternatives.len(), &mut tags, budget)?;
                preflight_type(&alternative.ty, budget)?;
            }
        }
    }
    Ok(())
}

/// Bounds every restriction value before copying or canonical numeric/scalar comparison.
pub(super) fn preflight_restrictions(
    field: &neutral_ir::composition::CompositionField,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    if let Some(choices) = &field.restrictions.choices {
        check_count(choices.len(), budget.limits.choices_per_field)?;
        charge(
            &mut budget.choices,
            choices.len() as u64,
            budget.limits.total_choices,
        )?;
        for choice in choices {
            preflight_value(choice, budget)?;
        }
    }
    for bound in [&field.restrictions.minimum, &field.restrictions.maximum]
        .into_iter()
        .flatten()
    {
        number(bound, budget)?;
    }
    Ok(())
}

/// Rejects invalid or duplicate member keys before canonical field/tag ordering.
fn member<'a>(
    name: &'a str,
    count: usize,
    seen: &mut BTreeSet<&'a str>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    budget.key(name.len(), count)?;
    if name.len() as u64 > budget.limits.json.string_bytes() {
        return Err(E::Limit);
    }
    budget.step(seen.len() as u64)?;
    if !is_snake_name(name)
        || is_protected_name(name)
        || !seen.insert(name).map_err(|_| E::Allocation)?
    {
        return Err(E::InvalidContract);
    }
    Ok(())
}

/// Checks every wrapper before recursive type consumers; nominal ownership remains exact.
pub(super) fn preflight_type(ty: &T, budget: &mut Budget<'_>) -> Result<(), E> {
    let mut current = ty;
    let mut depth = 0;
    loop {
        budget.depth(depth, budget.limits.type_depth)?;
        match current {
            T::Nullable(inner) if matches!(inner.as_ref(), T::Nullable(_)) => {
                return Err(E::InvalidContract);
            }
            T::Ref(inner)
                if !matches!(inner.as_ref(), T::Nominal(_) | T::VocabularyNominal { .. }) =>
            {
                return Err(E::InvalidContract);
            }
            T::List(inner) | T::Nullable(inner) | T::Ref(inner) => {
                current = inner;
                depth += 1;
            }
            T::Nominal(owner) => {
                validate_owner(owner, budget)?;
                return Ok(());
            }
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                budget.key(identity.len() + version.len() + name.len(), 1)?;
                if [identity.len(), version.len(), name.len()]
                    .into_iter()
                    .any(|n| n as u64 > budget.limits.json.string_bytes())
                {
                    return Err(E::Limit);
                }
                if !is_upper_name(identity)
                    || !neutral_ir::language::is_exact_release_version(version)
                    || !is_upper_name(name)
                    || is_protected_name(name)
                {
                    return Err(E::InvalidContract);
                }
                return Ok(());
            }
            _ => return Ok(()),
        }
    }
}

/// Bounds caller-owned defaults/choices iteratively before any recursive expansion or clone.
fn preflight_value(value: &V, budget: &mut Budget<'_>) -> Result<(), E> {
    let mut stack = Vec::new();
    stack.try_reserve_exact(1).map_err(|_| E::Allocation)?;
    stack.push((value, 0));
    while let Some((value, depth)) = stack.pop() {
        budget.depth(depth, budget.limits.value_depth)?;
        match value {
            V::List(items) => {
                check_count(items.len(), budget.limits.json.array_items())?;
                budget.step(items.len() as u64)?;
                stack.try_reserve(items.len()).map_err(|_| E::Allocation)?;
                stack.extend(items.iter().map(|item| (item, depth + 1)));
            }
            V::Record(fields) => {
                check_count(fields.len(), budget.limits.json.fields())?;
                budget.step(fields.len() as u64)?;
                stack.try_reserve(fields.len()).map_err(|_| E::Allocation)?;
                for (name, value) in fields {
                    budget.key(name.len(), fields.len())?;
                    if name.len() as u64 > budget.limits.json.string_bytes() {
                        return Err(E::Limit);
                    }
                    if let Some(value) = value {
                        stack.push((value, depth + 1));
                    }
                }
            }
            V::Variant { tag, payload } => {
                budget.step(tag.len() as u64)?;
                if tag.len() as u64 > budget.limits.json.string_bytes() {
                    return Err(E::Limit);
                }
                stack.try_reserve(1).map_err(|_| E::Allocation)?;
                stack.push((payload, depth + 1));
            }
            _ => scalar(value, budget)?,
        }
    }
    Ok(())
}

/// Checks scalar retention and exact-number limits without exponent-sized allocation.
fn scalar(value: &V, budget: &mut Budget<'_>) -> Result<(), E> {
    match value {
        V::Number(value) => number(value, budget)?,
        V::String(text) | V::Url(text) | V::Path(text) => {
            budget.step(text.len() as u64)?;
            if text.len() as u64 > budget.limits.json.string_bytes() {
                return Err(E::Limit);
            }
        }
        _ => {
            budget.step(1)?;
        }
    }
    Ok(())
}

/// Checks a borrowed exact number before cloning any restriction bound.
fn number(number: &neutral_ir::ExactNumber, budget: &mut Budget<'_>) -> Result<(), E> {
    budget.step(number.coefficient().len() as u64)?;
    if number.coefficient().len() as u64 > budget.limits.json.numeric_digits
        || number.scale().unsigned_abs() > budget.limits.json.numeric_scale
    {
        return Err(E::Limit);
    }
    Ok(())
}

/// Builds both exact nominal maps once per request, without cloning private contract bodies.
pub(super) fn lookup<'a>(
    catalogue: &'a ValidatedComposition,
    sources: &'a [SourceCompositionDefinition],
    budget: &mut Budget<'_>,
) -> Result<values::Catalogue<'a>, E> {
    let mut result = values::Catalogue::new();
    result.project_depth = true;
    for bundle in catalogue.bundles() {
        for definition in &bundle.definitions {
            budget.key(
                bundle.identity.identity().len()
                    + bundle.identity.version().len()
                    + definition.name.len(),
                result.len() + 1,
            )?;
            budget.step(result.len() as u64)?;
            result.insert(
                (
                    bundle.identity.identity(),
                    bundle.identity.version(),
                    &definition.name,
                ),
                definition,
            )?;
        }
    }
    for source in sources {
        budget.key(
            source.owner.module().module_name().len() + source.definition.name.len(),
            result.len() + 1,
        )?;
        budget.step(result.len() as u64)?;
        result
            .sources
            .insert(&source.owner, &source.definition)
            .map_err(|_| E::Allocation)?;
    }
    Ok(result)
}

/// Checks source public/external closure and records every non-reference embedding edge.
fn validate_edges(
    sources: &[SourceCompositionDefinition],
    lookup: &values::Catalogue<'_>,
    budget: &mut Budget<'_>,
) -> Result<(), E> {
    let mut graph = BTreeMap::<&ModuleSymbolIdentity, BTreeSet<&ModuleSymbolIdentity>>::new();
    for source in sources {
        budget.step(graph.len() as u64)?;
        let targets = graph
            .entry(&source.owner)
            .map_err(|_| E::Allocation)?
            .or_default();
        for ty in values::definition_types(&source.definition) {
            let mut ty = ty;
            let mut reference = false;
            loop {
                budget.step(1)?;
                match ty {
                    T::List(inner) | T::Nullable(inner) => ty = inner,
                    T::Ref(inner) => {
                        reference = true;
                        ty = inner;
                    }
                    T::Nominal(owner) => {
                        let target = lookup.resolve(ty, budget)?;
                        if !target.public
                            && (source.definition.public || owner.module() != source.owner.module())
                        {
                            return Err(E::PrivateType);
                        }
                        if !reference {
                            budget.step(targets.len() as u64)?;
                            targets.insert(owner).map_err(|_| E::Allocation)?;
                        }
                        break;
                    }
                    T::VocabularyNominal { .. } => {
                        if !lookup.resolve(ty, budget)?.public {
                            return Err(E::PrivateType);
                        }
                        break;
                    }
                    _ => break,
                }
            }
        }
    }
    let mut done = BTreeSet::new();
    for root in graph.keys() {
        let mut visiting = BTreeSet::new();
        let mut stack = Vec::new();
        stack.try_reserve_exact(1).map_err(|_| E::Allocation)?;
        stack.push((*root, false));
        while let Some((owner, leaving)) = stack.pop() {
            budget.step(1)?;
            if leaving {
                budget.step(visiting.len() as u64 + done.len() as u64)?;
                visiting.remove(owner);
                done.insert(owner).map_err(|_| E::Allocation)?;
                continue;
            }
            if done.contains(owner) {
                continue;
            }
            budget.step(visiting.len() as u64)?;
            if !visiting.insert(owner).map_err(|_| E::Allocation)? {
                return Err(E::EmbeddedCycle);
            }
            stack
                .try_reserve(1 + graph.get(owner).map_or(0, BTreeSet::len))
                .map_err(|_| E::Allocation)?;
            stack.push((owner, true));
            if let Some(targets) = graph.get(owner) {
                stack.extend(targets.iter().map(|target| (*target, false)));
            }
        }
    }
    Ok(())
}
