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
use neutral_core::allocation::RetainCapacity;

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
    /// Conservative owned-slot/text retention for callers that subsequently copy contracts.
    bytes: u64,
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

/// Preflights borrowed nominal bodies before any recursive source-definition copy.
///
/// # Errors
/// Returns bounded resource/allocation or cancellation failure, without a fake project envelope.
pub fn inspect_composition_bodies<'a>(
    bodies: impl IntoIterator<Item = &'a CompositionBody>,
    limits: ProjectLimits,
    policy: CompositionPolicy,
    cancellation: &CancellationToken,
) -> Result<CompositionBodyInspection, CompositionShapeError> {
    let mut inspector = Inspector {
        bytes: 0,
        limits,
        policy,
        cancellation,
        work: 0,
        text: 0,
        facts: CompositionResourceFacts::from_values([0; 7]),
    };
    inspector.step(1)?;
    for body in bodies {
        inspector.body(body)?;
    }
    Ok(CompositionBodyInspection {
        facts: inspector.facts,
        work: inspector.work,
        retained_bytes: inspector.bytes,
    })
}

/// Inspects captured contracts before an owned catalogue copy or nested validation.
///
/// # Errors
/// Rejects structural/resource/cancellation failures without granting contract authority.
pub fn inspect_composition_catalogue(
    bundles: &[crate::composition::CompositionBundle],
    limits: ProjectLimits,
    policy: CompositionPolicy,
    cancellation: &CancellationToken,
) -> Result<CompositionBodyInspection, CompositionShapeError> {
    let mut inspector = Inspector {
        bytes: 0,
        limits,
        policy,
        cancellation,
        work: 0,
        text: 0,
        facts: CompositionResourceFacts::from_values([0; 7]),
    };
    inspector.count(bundles.len(), policy.bundles)?;
    inspector.catalogue(bundles)?;
    for bundle in bundles {
        inspector.step(bundle.identity.encoding_version().len() as u64)?;
    }
    Ok(CompositionBodyInspection {
        facts: inspector.facts,
        work: inspector.work,
        retained_bytes: inspector.bytes,
    })
}

/// Borrowed contract facts and comparison/copy work, not validated contract authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionBodyInspection {
    /// Owned slots and UTF-8 text, independent of serialized size or allocator overhead.
    pub retained_bytes: u64,
    /// Independently counted raw body occurrences.
    pub facts: CompositionResourceFacts,
    /// Checked traversal and key work available for aggregate caller accounting.
    pub work: u64,
}

impl Inspector<'_> {
    /// Shared catalogue copy projection; dependency metadata remains inert text.
    fn catalogue(
        &mut self,
        bundles: &[crate::composition::CompositionBundle],
    ) -> Result<(), CompositionShapeError> {
        self.storage::<crate::composition::CompositionBundle>(bundles.len())?;
        for bundle in bundles {
            self.storage::<String>(bundle.identity.required_features().len())?;
            self.storage::<crate::composition::CompositionDependency>(bundle.dependencies.len())?;
            self.storage::<crate::composition::CompositionDefinition>(bundle.definitions.len())?;
            self.text(bundle.identity.identity())?;
            self.text(bundle.identity.version())?;
            self.text(bundle.identity.schema_version())?;
            self.storage::<u8>(bundle.identity.encoding_version().len())?;
            for feature in bundle.identity.required_features() {
                self.text(feature)?;
            }
            self.count(
                bundle.dependencies.len(),
                self.policy.dependencies_per_bundle,
            )?;
            self.facts.dependency_edges = self
                .facts
                .dependency_edges
                .checked_add(bundle.dependencies.len() as u64)
                .ok_or(CompositionShapeError::Limit)?;
            if self.facts.dependency_edges > self.policy.dependency_edges {
                return Err(CompositionShapeError::Limit);
            }
            for dependency in &bundle.dependencies {
                self.text(&dependency.identity)?;
                self.text(&dependency.version)?;
            }
            self.count(bundle.definitions.len(), self.policy.total_types)?;
            for definition in &bundle.definitions {
                self.text(&definition.name)?;
                self.body(&definition.body)?;
            }
        }
        Ok(())
    }

    /// Checked model storage projection; never allocates or changes historical acceptance policy.
    fn storage<U>(&mut self, count: usize) -> Result<(), CompositionShapeError> {
        self.bytes = self
            .bytes
            .checked_add(
                (count as u64)
                    .checked_mul(std::mem::size_of::<U>() as u64)
                    .ok_or(CompositionShapeError::Limit)?,
            )
            .ok_or(CompositionShapeError::Limit)?;
        Ok(())
    }
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
        self.storage::<u8>(text.len())?;
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
            self.storage::<T>(1)?;
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
        self.step(1)?;
        let mut stack = Vec::new();
        stack
            .try_retain(1)
            .map_err(|_| CompositionShapeError::Limit)?;
        stack.push((value, 0_u64));
        while let Some((value, depth)) = stack.pop() {
            self.storage::<CompositionValue<R>>(1)?;
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
                        .try_retain(items.len())
                        .map_err(|_| CompositionShapeError::Limit)?;
                    stack.extend(items.iter().map(|value| (value, depth + 1)));
                }
                CompositionValue::Record(fields) => {
                    self.storage::<(String, Option<CompositionValue<R>>)>(fields.len())?;
                    self.count(fields.len(), self.policy.total_fields)?;
                    stack
                        .try_retain(fields.len())
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
                        .try_retain(1)
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
                self.storage::<crate::composition::CompositionField>(fields.len())?;
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
                self.storage::<crate::composition::CompositionAlternative>(alternatives.len())?;
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
    inspect_retained(
        Sections {
            modules: &ir.modules,
            declarations: ir.declarations.len(),
            map_limit: limits.declarations,
            vocabularies: &ir.vocabularies,
            source_maps: &ir.source_maps,
            provenance: &ir.provenance,
            origins: &ir.origins,
            sources: &ir.sources,
            vocabulary_sources: &ir.vocabulary_sources,
        },
        ir.declarations
            .iter()
            .map(|d| (&d.identity, &d.signature, d.value.as_ref())),
        limits,
        policy,
        cancellation,
        &[],
    )
    .map(|inspection| inspection.facts)
}

/// Borrowed historical companion roles, shared without a fabricated project envelope.
#[derive(Clone, Copy)]
struct Sections<'a> {
    /// Complete module graph.
    modules: &'a [crate::project::ProjectModule],
    /// Complete ordinary declaration count.
    declarations: usize,
    /// Independent owner-map limit, preserving historical declaration-only controls.
    map_limit: u64,
    /// Complete nominal vocabulary contracts.
    vocabularies: &'a [crate::composition::CompositionBundle],
    /// Original declaration source mappings.
    source_maps: &'a [crate::project::ProjectSourceMap],
    /// Structural dependency evidence.
    provenance: &'a [crate::project::ProjectProvenance],
    /// Materialized value origin evidence.
    origins: &'a [super::CompositionOrigin],
    /// Original captured source accounting.
    sources: &'a [crate::project::ProjectSource],
    /// Original captured vocabulary accounting.
    vocabulary_sources: &'a [crate::project::ProjectVocabularySource],
}

/// Shares raw collection/type/value accounting across independently selected project profiles.
fn inspect_retained<'a>(
    ir: Sections<'a>,
    declarations: impl IntoIterator<
        Item = (
            &'a ModuleSymbolIdentity,
            &'a CompositionSignature,
            Option<&'a crate::composition::BindingValue>,
        ),
    >,
    limits: ProjectLimits,
    policy: CompositionPolicy,
    cancellation: &CancellationToken,
    expressions: &[&crate::expression::Expression],
) -> Result<CompositionBodyInspection, CompositionShapeError> {
    let mut inspector = Inspector {
        bytes: 0,
        limits,
        policy,
        cancellation,
        work: 0,
        text: 0,
        facts: CompositionResourceFacts::from_values([0; 7]),
    };
    inspector.count(ir.modules.len(), limits.modules)?;
    inspector.count(ir.declarations, limits.declarations)?;
    inspector.count(ir.vocabularies.len(), policy.bundles)?;
    inspector.count(ir.source_maps.len(), ir.map_limit)?;
    inspector.count(ir.provenance.len(), limits.nodes)?;
    inspector.count(ir.origins.len(), limits.nodes)?;
    inspector.count(ir.sources.len(), limits.modules)?;
    inspector.count(ir.vocabulary_sources.len(), policy.bundles)?;
    let mut imports = 0_u64;
    for module in ir.modules {
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
    for (identity, signature, value) in declarations {
        inspector.symbol(identity)?;
        match signature {
            CompositionSignature::Binding(ty) => inspector.ty(ty)?,
            CompositionSignature::Definition(body) => inspector.body(body)?,
        }
        if let Some(value) = value {
            inspector.value(value, |target| Some(target))?;
        }
    }
    inspector.catalogue(ir.vocabularies)?;
    for expression in expressions {
        inspector.expression(expression)?;
    }
    inspect_companions(&ir, &mut inspector)?;
    inspector.facts.bundles = ir.vocabularies.len() as u64;
    Ok(CompositionBodyInspection {
        facts: inspector.facts,
        work: inspector.work,
        retained_bytes: inspector.bytes,
    })
}

/// Checks retained companion collection/path text before independent provenance validation.
fn inspect_companions(
    ir: &Sections<'_>,
    inspector: &mut Inspector<'_>,
) -> Result<(), CompositionShapeError> {
    for source in ir.sources {
        inspector.text(&source.module)?;
        inspector.text(&source.source_id)?;
    }
    for source in ir.vocabulary_sources {
        inspector.text(&source.identity)?;
        inspector.text(&source.version)?;
    }
    for mapping in ir.source_maps {
        inspector.symbol(&mapping.declaration)?;
    }
    for provenance in ir.provenance {
        inspector.symbol(&provenance.from)?;
        inspector.symbol(&provenance.to)?;
    }
    for origin in ir.origins {
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

impl Inspector<'_> {
    /// Counts retained typed-tree leaves without interpreting input as a fabricated value.
    fn expression(
        &mut self,
        root: &crate::expression::Expression,
    ) -> Result<(), CompositionShapeError> {
        use crate::expression::ExpressionNode as N;
        let mut stack = Vec::new();
        stack
            .try_retain(1)
            .map_err(|_| CompositionShapeError::Limit)?;
        stack.push((root, 1_u64));
        while let Some((expression, depth)) = stack.pop() {
            self.step(1)?;
            if depth > crate::expression::profile::MAX_DEPTH as u64 {
                return Err(CompositionShapeError::Limit);
            }
            self.ty(&expression.ty)?;
            let mut child = |e| {
                stack
                    .try_retain(1)
                    .map_err(|_| CompositionShapeError::Limit)?;
                stack.push((e, depth + 1));
                Ok::<_, CompositionShapeError>(())
            };
            match &expression.node {
                N::Value(value) => self.value(value, |target| Some(target))?,
                N::Input(owner) => self.symbol(owner)?,
                N::Output {
                    target,
                    result_type,
                    selector,
                } => {
                    self.symbol(target)?;
                    self.ty(result_type)?;
                    for segment in selector {
                        if let crate::expression::SelectorSegment::Field(name)
                        | crate::expression::SelectorSegment::Alternative(name) = segment
                        {
                            self.text(name)?;
                        }
                    }
                }
                N::Unary { operand, .. } | N::Text(operand) => child(operand.as_ref())?,
                N::Binary { left, right, .. } => {
                    child(left.as_ref())?;
                    child(right.as_ref())?;
                }
                N::If {
                    guard,
                    then_branch,
                    else_branch,
                } => {
                    child(guard.as_ref())?;
                    child(then_branch.as_ref())?;
                    child(else_branch.as_ref())?;
                }
                N::Match { scrutinee, arms } => {
                    child(scrutinee.as_ref())?;
                    for arm in arms {
                        self.text(&arm.tag)?;
                        self.ty(&arm.payload_type)?;
                        child(&arm.body)?;
                    }
                }
                N::List(items) | N::Interpolate(items) => {
                    for item in items {
                        child(item)?;
                    }
                }
                N::Record(fields) => {
                    for (name, value) in fields {
                        self.text(name)?;
                        if let Some(value) = value {
                            child(value)?;
                        }
                    }
                }
                N::Variant { tag, payload } => {
                    self.text(tag)?;
                    child(payload.as_ref())?;
                }
                N::Bound { .. } => {}
            }
        }
        Ok(())
    }
}

/// Recomputes shared retained composition facts for raw expression data, not reader authority.
/// # Errors
/// Rejects independent resource/allocation controls or cancellation atomically.
pub fn inspect_expression_project_resources(
    ir: &crate::expression::project::ExpressionProjectIr,
    limits: ProjectLimits,
    policy: CompositionPolicy,
    cancellation: &CancellationToken,
) -> Result<CompositionResourceFacts, CompositionShapeError> {
    inspect_expression_project_resources_with_work(ir, limits, policy, cancellation)
        .map(|inspection| inspection.facts)
}

/// Reports whole-project inspection work so nested producers can debit an aggregate allowance.
///
/// # Errors
/// Same structural/count/allocation/cancellation failures as the facts-only entry point.
pub fn inspect_expression_project_resources_with_work(
    ir: &crate::expression::project::ExpressionProjectIr,
    limits: ProjectLimits,
    policy: CompositionPolicy,
    cancellation: &CancellationToken,
) -> Result<CompositionBodyInspection, CompositionShapeError> {
    use crate::expression::ExpressionBody;
    if ir.declarations.len() as u64 > limits.declarations
        || ir.public_templates.len() as u64
            > ir.expansion_limits
                .definitions
                .min(crate::expression::profile::MAX_ITEMS)
    {
        return Err(CompositionShapeError::Limit);
    }
    let mut trees = Vec::new();
    for d in &ir.declarations {
        if cancellation.is_cancelled() {
            return Err(CompositionShapeError::Cancelled);
        }
        if let Some(ExpressionBody::Expression(e)) = &d.body {
            trees
                .try_retain(1)
                .map_err(|_| CompositionShapeError::Limit)?;
            trees.push(e);
        }
    }
    for template in &ir.public_templates {
        if cancellation.is_cancelled() {
            return Err(CompositionShapeError::Cancelled);
        }
        if template.parameters.len() as u64
            > crate::expression::templates::ExpansionLimits::hard().parameters
        {
            return Err(CompositionShapeError::Limit);
        }
        trees
            .try_retain(1)
            .map_err(|_| CompositionShapeError::Limit)?;
        trees.push(&template.body);
        for parameter in &template.parameters {
            if let Some(e) = &parameter.default {
                trees
                    .try_retain(1)
                    .map_err(|_| CompositionShapeError::Limit)?;
                trees.push(e);
            }
        }
    }
    inspect_retained(
        Sections {
            modules: &ir.modules,
            declarations: ir.declarations.len(),
            map_limit: limits
                .declarations
                .checked_add(
                    ir.expansion_limits
                        .definitions
                        .min(crate::expression::profile::MAX_ITEMS),
                )
                .ok_or(CompositionShapeError::Limit)?,
            vocabularies: &ir.vocabularies,
            source_maps: &ir.source_maps,
            provenance: &ir.provenance,
            origins: &ir.origins,
            sources: &ir.sources,
            vocabulary_sources: &ir.vocabulary_sources,
        },
        ir.declarations.iter().map(|d| {
            (
                &d.identity,
                &d.signature,
                match &d.body {
                    Some(ExpressionBody::Value(value)) => Some(value),
                    _ => None,
                },
            )
        }),
        limits,
        policy,
        cancellation,
        &trees,
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/composition_project_inspection.rs"]
mod tests;
