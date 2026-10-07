// SPDX-License-Identifier: Apache-2.0

//! Iterative raw producer preflight before any recursive consumer or retained clone.

use super::{
    CompositionPolicy, CompositionProjectIr, CompositionResourceFacts, CompositionSignature,
};
use crate::{
    ModuleSymbolIdentity,
    composition::{CompositionBody, CompositionValue, ValuePathSegment, profile},
    project::{PROJECT_MAX_DEPTH, ProjectLimits},
    project_interface::ProjectPublicType as T,
};
use neutral_core::CancellationToken;

/// Safe structural preflight failures without untrusted names or partial facts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionShapeError {
    /// An independent count, byte, depth, work or allocation bound was exceeded.
    Limit,
    /// Cancellation prevented complete inspection.
    Cancelled,
}

/// Request-local retained-data inspection counters shared by all declarations and companions.
struct Inspector<'a> {
    /// Explicit project retention policy.
    limits: ProjectLimits,
    /// Explicit composition controls.
    policy: CompositionPolicy,
    /// Cooperative caller cancellation.
    cancellation: &'a CancellationToken,
    /// Charged inspection/key work, not asserted producer work.
    work: u64,
    /// Aggregate retained text, excluding advertised source byte lengths.
    text: u64,
    /// Independently recomputed retained facts.
    facts: CompositionResourceFacts,
}

impl Inspector<'_> {
    /// Charges checked inspection work before proportional traversal or allocation.
    fn step(&mut self, n: u64) -> Result<(), CompositionShapeError> {
        if self.cancellation.is_cancelled() {
            return Err(CompositionShapeError::Cancelled);
        }
        self.work = self
            .work
            .checked_add(n)
            .ok_or(CompositionShapeError::Limit)?;
        if self.work > self.policy.work.min(profile::MAX_ITEMS) {
            return Err(CompositionShapeError::Limit);
        }
        Ok(())
    }

    /// Charges aggregate retained key/text bytes before cloning or comparing them.
    fn text(&mut self, text: &str) -> Result<(), CompositionShapeError> {
        self.step(text.len() as u64 + 1)?;
        self.text = self
            .text
            .checked_add(text.len() as u64)
            .ok_or(CompositionShapeError::Limit)?;
        if self.text > self.limits.text_bytes {
            return Err(CompositionShapeError::Limit);
        }
        Ok(())
    }

    /// Bounds an input collection against both an explicit policy and the shared hard item ceiling.
    fn count(&mut self, n: usize, limit: u64) -> Result<(), CompositionShapeError> {
        self.step(1)?;
        if n as u64 > limit.min(profile::MAX_ITEMS) {
            return Err(CompositionShapeError::Limit);
        }
        Ok(())
    }

    /// Checks exact symbol key work without allocating an owner or source path.
    fn symbol(&mut self, symbol: &ModuleSymbolIdentity) -> Result<(), CompositionShapeError> {
        self.text(symbol.module().language_behavior_version())?;
        self.text(symbol.module().module_name())?;
        self.text(symbol.declaration_name())
    }

    /// Bounds every unary wrapper before recursive semantic/type comparison consumers.
    fn ty(&mut self, mut ty: &T) -> Result<(), CompositionShapeError> {
        let mut depth = 0;
        loop {
            self.step(1)?;
            if depth > self.policy.type_depth.min(PROJECT_MAX_DEPTH as u64) {
                return Err(CompositionShapeError::Limit);
            }
            match ty {
                T::List(inner) | T::Ref(inner) | T::Nullable(inner) => {
                    ty = inner;
                    depth += 1;
                }
                T::Nominal(owner) => return self.symbol(owner),
                T::VocabularyNominal {
                    identity,
                    version,
                    name,
                } => {
                    self.text(identity)?;
                    self.text(version)?;
                    return self.text(name);
                }
                _ => return Ok(()),
            }
        }
    }

    /// Counts retained value nodes iteratively; references are keys, never embedded targets.
    fn value<R>(
        &mut self,
        value: &CompositionValue<R>,
        reference: impl Fn(&R) -> Option<&ModuleSymbolIdentity>,
    ) -> Result<(), CompositionShapeError> {
        let mut stack = vec![(value, 0_u64)];
        while let Some((value, depth)) = stack.pop() {
            self.step(1)?;
            if depth > self.policy.value_depth.min(PROJECT_MAX_DEPTH as u64) {
                return Err(CompositionShapeError::Limit);
            }
            self.facts.retained_value_nodes = self
                .facts
                .retained_value_nodes
                .checked_add(1)
                .ok_or(CompositionShapeError::Limit)?;
            if self.facts.retained_value_nodes > self.limits.nodes.min(self.policy.value_nodes) {
                return Err(CompositionShapeError::Limit);
            }
            match value {
                CompositionValue::Number(number) => self.text(number.coefficient())?,
                CompositionValue::String(text)
                | CompositionValue::Url(text)
                | CompositionValue::Path(text) => self.text(text)?,
                CompositionValue::Reference(target) => {
                    if let Some(target) = reference(target) {
                        self.symbol(target)?;
                    }
                }
                CompositionValue::List(items) => {
                    self.count(items.len(), self.limits.nodes)?;
                    stack
                        .try_reserve(items.len())
                        .map_err(|_| CompositionShapeError::Limit)?;
                    stack.extend(items.iter().map(|value| (value, depth + 1)));
                }
                CompositionValue::Record(fields) => {
                    self.count(fields.len(), self.policy.total_fields)?;
                    stack
                        .try_reserve(fields.len())
                        .map_err(|_| CompositionShapeError::Limit)?;
                    for (name, value) in fields {
                        self.text(name)?;
                        if let Some(value) = value {
                            stack.push((value, depth + 1));
                        } else {
                            self.facts.retained_value_nodes = self
                                .facts
                                .retained_value_nodes
                                .checked_add(1)
                                .ok_or(CompositionShapeError::Limit)?;
                        }
                    }
                    if self.facts.retained_value_nodes
                        > self.limits.nodes.min(self.policy.value_nodes)
                    {
                        return Err(CompositionShapeError::Limit);
                    }
                }
                CompositionValue::Variant { tag, payload } => {
                    self.text(tag)?;
                    stack
                        .try_reserve(1)
                        .map_err(|_| CompositionShapeError::Limit)?;
                    stack.push((payload, depth + 1));
                }
                CompositionValue::Null | CompositionValue::Bool(_) => {}
            }
        }
        Ok(())
    }

    /// Checks both definition kinds and all defaults, finite choices and restrictions.
    fn body(&mut self, body: &CompositionBody) -> Result<(), CompositionShapeError> {
        self.facts.types = self
            .facts
            .types
            .checked_add(1)
            .ok_or(CompositionShapeError::Limit)?;
        if self.facts.types > self.policy.total_types {
            return Err(CompositionShapeError::Limit);
        }
        match body {
            CompositionBody::Record(fields) => {
                self.count(fields.len(), self.policy.total_fields)?;
                self.facts.fields = self
                    .facts
                    .fields
                    .checked_add(fields.len() as u64)
                    .ok_or(CompositionShapeError::Limit)?;
                if self.facts.fields > self.policy.total_fields {
                    return Err(CompositionShapeError::Limit);
                }
                for field in fields {
                    self.text(&field.name)?;
                    self.ty(&field.ty)?;
                    let restrictions = &field.restrictions;
                    if let Some(choices) = &restrictions.choices {
                        self.count(choices.len(), self.policy.choices_per_field)?;
                        self.facts.choices = self
                            .facts
                            .choices
                            .checked_add(choices.len() as u64)
                            .ok_or(CompositionShapeError::Limit)?;
                        if self.facts.choices > self.policy.total_choices {
                            return Err(CompositionShapeError::Limit);
                        }
                        for choice in choices {
                            self.value(choice, |never| match *never {})?;
                        }
                    }
                    for bound in [&restrictions.minimum, &restrictions.maximum]
                        .into_iter()
                        .flatten()
                    {
                        self.text(bound.coefficient())?;
                    }
                    if let Some(default) = &field.default {
                        self.value(default, |never| match *never {})?;
                    }
                }
            }
            CompositionBody::Variant(alternatives) => {
                self.count(alternatives.len(), self.policy.alternatives_per_type)?;
                self.facts.alternatives = self
                    .facts
                    .alternatives
                    .checked_add(alternatives.len() as u64)
                    .ok_or(CompositionShapeError::Limit)?;
                if self.facts.alternatives > self.policy.total_alternatives {
                    return Err(CompositionShapeError::Limit);
                }
                for alternative in alternatives {
                    self.text(&alternative.tag)?;
                    self.ty(&alternative.ty)?;
                }
            }
        }
        Ok(())
    }
}

/// Preflights every raw retained value/type/companion before consumers clone or recurse.
///
/// This is structural resource accounting, not semantic validation or authentication.
/// All nominal contracts count once; binding/default/choice occurrences count independently.
///
/// # Errors
/// Returns only bounded limit/allocation or cancellation failure, never partial facts.
pub fn inspect_composition_ir(
    ir: &CompositionProjectIr,
    limits: ProjectLimits,
    policy: CompositionPolicy,
    cancellation: &CancellationToken,
) -> Result<CompositionResourceFacts, CompositionShapeError> {
    let mut inspector = Inspector {
        limits,
        policy,
        cancellation,
        work: 0,
        text: 0,
        facts: CompositionResourceFacts::from_values([0; 7]),
    };
    inspector.count(ir.modules.len(), limits.modules)?;
    inspector.count(ir.declarations.len(), limits.declarations)?;
    inspector.count(ir.vocabularies.len(), policy.bundles)?;
    inspector.count(ir.source_maps.len(), limits.declarations)?;
    inspector.count(ir.provenance.len(), limits.nodes)?;
    inspector.count(ir.origins.len(), limits.nodes)?;
    inspector.count(ir.sources.len(), limits.modules)?;
    inspector.count(ir.vocabulary_sources.len(), policy.bundles)?;
    let mut imports = 0_u64;
    for module in &ir.modules {
        inspector.text(module.identity.module_name())?;
        inspector.count(module.imports.len(), limits.import_edges)?;
        imports = imports
            .checked_add(module.imports.len() as u64)
            .ok_or(CompositionShapeError::Limit)?;
        if imports > limits.import_edges {
            return Err(CompositionShapeError::Limit);
        }
        for target in &module.imports {
            inspector.text(target)?;
        }
    }
    for declaration in &ir.declarations {
        inspector.symbol(&declaration.identity)?;
        match &declaration.signature {
            CompositionSignature::Binding(ty) => inspector.ty(ty)?,
            CompositionSignature::Definition(body) => inspector.body(body)?,
        }
        if let Some(value) = &declaration.value {
            inspector.value(value, |target| Some(target))?;
        }
    }
    for bundle in &ir.vocabularies {
        inspector.text(bundle.identity.identity())?;
        inspector.text(bundle.identity.version())?;
        inspector.text(bundle.identity.schema_version())?;
        for feature in bundle.identity.required_features() {
            inspector.text(feature)?;
        }
        inspector.count(bundle.dependencies.len(), policy.dependencies_per_bundle)?;
        inspector.facts.dependency_edges = inspector
            .facts
            .dependency_edges
            .checked_add(bundle.dependencies.len() as u64)
            .ok_or(CompositionShapeError::Limit)?;
        if inspector.facts.dependency_edges > policy.dependency_edges {
            return Err(CompositionShapeError::Limit);
        }
        for dependency in &bundle.dependencies {
            inspector.text(&dependency.identity)?;
            inspector.text(&dependency.version)?;
        }
        inspector.count(bundle.definitions.len(), policy.total_types)?;
        for definition in &bundle.definitions {
            inspector.text(&definition.name)?;
            inspector.body(&definition.body)?;
        }
    }
    inspect_companions(ir, &mut inspector)?;
    inspector.facts.bundles = ir.vocabularies.len() as u64;
    Ok(inspector.facts)
}

/// Checks retained companion collection/path text before independent provenance validation.
fn inspect_companions(
    ir: &CompositionProjectIr,
    inspector: &mut Inspector<'_>,
) -> Result<(), CompositionShapeError> {
    for source in &ir.sources {
        inspector.text(&source.module)?;
        inspector.text(&source.source_id)?;
    }
    for source in &ir.vocabulary_sources {
        inspector.text(&source.identity)?;
        inspector.text(&source.version)?;
    }
    for mapping in &ir.source_maps {
        inspector.symbol(&mapping.declaration)?;
    }
    for provenance in &ir.provenance {
        inspector.symbol(&provenance.from)?;
        inspector.symbol(&provenance.to)?;
    }
    for origin in &ir.origins {
        inspector.symbol(&origin.binding)?;
        inspector.count(
            origin.path.len(),
            inspector.policy.value_depth.min(PROJECT_MAX_DEPTH as u64),
        )?;
        for segment in &origin.path {
            if let ValuePathSegment::Field(name) = segment {
                inspector.text(name)?;
            }
        }
        if let Some(super::CompositionAttribution::Vocabulary {
            identity,
            version,
            type_name,
            field_name,
            ..
        }) = &origin.attribution
        {
            inspector.text(identity)?;
            inspector.text(version)?;
            inspector.text(type_name)?;
            inspector.text(field_name)?;
        }
    }
    Ok(())
}
