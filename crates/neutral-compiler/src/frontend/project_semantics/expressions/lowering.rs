// SPDX-License-Identifier: Apache-2.0

//! Complete raw assembly and private static evaluation through the shared semantic kernel.
//!
//! Assemble declarations, retained expression trees and original-byte companions
//! atomically under shared limits. Static evaluation may materialize a value;
//! deferred evaluation remains explicit consumer work. Recomputed resource and
//! identity facts describe the complete project, never a selected public view.
//!
//! Temporary models allow selected typed subtrees to be checked/folded against
//! request-local dependencies without publishing them as complete artifacts.
//! Complete assembly instead requires every ordinary/generated binding body,
//! retains exported non-callable templates and rebuilds source/expansion evidence.
//! Missing values are never silently replaced with null or invented external inputs.
//!
//! Structural type/reference provenance is distinct from guarded expression output
//! dependencies. Normalized source reuse needs its own original-syntax evidence;
//! source maps and origins remain companions rather than logical identity inputs.
//! Occurrence evidence records the source precision actually retained by this
//! producer; it must not imply finer token-level attribution than was supplied.
//!
//! Catalogue/companion copies, nested semantic preparation and normalization debit
//! the same aggregate request budget. Recomputed retained facts describe the final
//! model, not execution history. Placeholder digest/count fields stay private until
//! final orchestration derives the authoritative producer values and publishes a root.

use super::{
    B, Budget, CompositionOrigin, CompositionResourceFacts, E, Edge, ExpansionAttribution,
    Expression, ExpressionAttribution, ExpressionBody, ExpressionDeclaration, ExpressionProjectIr,
    ExpressionResourceFacts, ModuleSymbolIdentity, N, NormalizedBinding, ProjectProvenance,
    ProjectResourceFacts, ProjectSourceMap, PublicTemplate, Root, S, SemanticDigest,
    SourceLocation, State, T, TemplateParameter, TryClone, V, alloc, composition_error, copy_slice,
    error, evaluation_error, expression_profile, grow, normalize_binding_with_budget,
    prepare_expression_model,
};
use crate::frontend::expression_codes as ec;
use neutral_ir::composition::{
    ValueOriginKind as K, ValuePathSegment as P, project::CompositionAttribution,
};

impl State<'_, '_> {
    /// Default/type preflight and normalization share aggregate source-validation work.
    pub(super) fn source_scope(
        &mut self,
    ) -> Result<neutral_vocabulary::composition::ValidatedCompositionScope, E> {
        use neutral_ir::composition::{CompositionDefinition, SourceCompositionDefinition};
        let project = self.semantic_limits().project;
        let policy = self.captured.composition_limits().policy();
        let mut definitions = Vec::new();
        self.budget
            .retain::<SourceCompositionDefinition>(self.resolver.definitions.len())?;
        grow(&mut definitions, self.resolver.definitions.len())?;
        for source in &self.resolver.definitions {
            definitions.push(SourceCompositionDefinition {
                owner: self.budget.copy_symbol(&source.owner)?,
                definition: CompositionDefinition {
                    name: self.budget.copy_text(&source.definition.name)?,
                    public: source.definition.public,
                    body: self
                        .budget
                        .copy_definition(&source.definition.body, project, policy)?,
                },
            });
        }
        let mut remaining = self.budget.remaining().work;
        let before = remaining;
        let scope = neutral_vocabulary::composition::validate_composition_scope_with_budget(
            self.captured.catalogue(),
            definitions,
            self.captured.composition_limits(),
            &mut remaining,
            self.budget.cancel,
        )
        .map_err(|e| error(e.diagnostic_code(), None))?;
        self.budget.charge(before - remaining)?;
        Ok(scope)
    }
    /// Captured metadata and catalogue copies pay before shared companion assembly allocates.
    fn charge_captured_companions(&mut self) -> Result<(), E> {
        use neutral_ir::project::{ProjectModule, ProjectSource, ProjectVocabularySource};
        self.budget
            .retain::<ProjectModule>(self.captured.sources().len())?;
        self.budget
            .retain::<ProjectSource>(self.captured.sources().len())?;
        self.budget
            .retain::<ProjectVocabularySource>(self.captured.vocabularies().len())?;
        let edges = self.resolver.graph.edges();
        self.budget.charge(
            (edges.len() as u64)
                .checked_mul(self.captured.sources().len() as u64)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
        )?;
        for source in self.captured.sources() {
            self.budget.bytes(
                (super::V1_SOURCE_PROFILE.len()
                    + source.module_id().len() * 2
                    + source.source_id().len()) as u64,
            )?;
            let imports = edges
                .iter()
                .filter(|e| e.from() == source.module_id())
                .count();
            self.budget.retain::<String>(imports)?;
            for edge in edges.iter().filter(|e| e.from() == source.module_id()) {
                self.budget.bytes(edge.target().len() as u64)?;
                self.budget.charge(
                    (edge.target().len() as u64 + 1)
                        .checked_mul(u64::from(imports.bit_width()) + 1)
                        .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
                )?;
            }
        }
        for bundle in self.captured.catalogue().bundles() {
            self.budget.bytes(
                (bundle.identity.identity().len() + bundle.identity.version().len()) as u64,
            )?;
            self.budget.charge(
                (self.captured.vocabularies().len() as u64)
                    .checked_mul(bundle.identity.identity().len() as u64 + 1)
                    .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
            )?;
        }
        let mut policy = self.captured.composition_limits().policy();
        policy.work = policy.work.min(self.budget.remaining().work);
        let catalogue = neutral_ir::composition::project::inspect_composition_catalogue(
            self.captured.catalogue().bundles(),
            self.semantic_limits().project,
            policy,
            self.budget.cancel,
        )
        .map_err(|_| self.budget.resource_error())?;
        self.budget.charge(
            catalogue
                .work
                .checked_mul(2)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
        )?;
        self.budget.storage(catalogue.retained_bytes)?;
        let profiles = self.captured.semantic_profiles();
        self.budget
            .retain::<String>(profiles.required_features.len())?;
        for text in [
            &profiles.expression,
            &profiles.evaluator,
            &profiles.expansion,
            &profiles.decimal,
        ]
        .into_iter()
        .chain(profiles.required_features.iter())
        {
            self.budget.bytes(text.len() as u64)?;
        }
        Ok(())
    }
    /// Producer fact derivation and validation share one pass and the remaining request budget.
    pub(super) fn prepare_model(
        &mut self,
        ir: ExpressionProjectIr,
    ) -> Result<neutral_semantics::ValidatedExpressionModel, E> {
        let mut remaining = self.budget.remaining();
        let before = remaining;
        let model = prepare_expression_model(
            ir,
            self.semantic_limits(),
            &mut remaining,
            self.budget.cancel,
        )
        .map_err(|e| {
            if e == neutral_semantics::ExpressionValidationError::Limit {
                self.budget.resource_error()
            } else {
                error(e.code(), None)
            }
        })?;
        self.budget.debit(before, remaining)?;
        Ok(model)
    }
    /// Nested folding consumes the request's remaining kernel work and scratch retention.
    pub(super) fn normalize(
        &mut self,
        model: &neutral_semantics::ValidatedExpressionModel,
        owner: &ModuleSymbolIdentity,
        location: Option<SourceLocation>,
    ) -> Result<NormalizedBinding, E> {
        let mut remaining = self.budget.remaining();
        let before = remaining;
        let result =
            normalize_binding_with_budget(model, owner, &mut remaining, self.budget.cancel)
                .map_err(|e| {
                    if e == neutral_semantics::EvaluationError::Limit {
                        let mut failure = self.budget.resource_error();
                        failure.location = location;
                        failure
                    } else {
                        evaluation_error(e, location)
                    }
                })?;
        self.budget.debit(before, remaining)?;
        Ok(result)
    }

    /// Normalizes an actual typed subtree in a closed request-local semantic model.
    /// No unavailable declaration is relabelled as null or as an external input.
    /// `None` means a residual/deferred result, not a typed null or an omitted field.
    pub(super) fn static_value(
        &mut self,
        owner: &Root,
        expression: &Expression,
    ) -> Result<Option<V>, E> {
        self.budget.charge(1)?;
        let mut references = Vec::new();
        collect_targets(&mut self.budget, expression, &mut references)?;
        for target in references {
            if target != owner.owner && !self.bodies.contains_key(&target) {
                self.ensure_binding(&target)?;
            }
        }
        let expression = self.budget.copy_expression(expression)?;
        let ir = self.assemble(true, Some((&owner.owner, &expression)))?;
        let model = self.prepare_model(ir)?;
        match self.normalize(&model, &owner.owner, Some(owner.location))? {
            NormalizedBinding::Value(v) => Ok(Some(v)),
            NormalizedBinding::Residual(_) => Ok(None),
        }
    }

    /// Assembles complete raw sections; only the independent reader can grant artifact authority.
    /// Temporary mode may omit unresolved ordinary bindings for local static checks;
    /// it is never the complete publication path. Selected expressions remain private.
    #[allow(clippy::too_many_lines)] // Keep the closed section grammar together while source integration is qualified.
    pub(super) fn assemble(
        &mut self,
        temporary: bool,
        selected: Option<(&ModuleSymbolIdentity, &Expression)>,
    ) -> Result<ExpressionProjectIr, E> {
        self.budget.charge(1)?;
        let project_limits = self.semantic_limits().project;
        let mut declarations = Vec::new();
        let mut maps = Vec::new();
        self.budget
            .retain::<ExpressionDeclaration>(self.resolver.roots.len())?;
        self.budget
            .retain::<ProjectSourceMap>(self.resolver.roots.len())?;
        for root in self.resolver.roots.values() {
            let (signature, body) = if let Some((_, expression)) =
                selected.filter(|(owner, _)| **owner == root.owner)
            {
                (
                    S::Binding(self.budget.copy_type(&expression.ty)?),
                    Some(ExpressionBody::Expression(
                        self.budget.copy_expression(expression)?,
                    )),
                )
            } else if root.ty.is_some() {
                let body = if let Some(body) = self.bodies.get(&root.owner) {
                    match body {
                        ExpressionBody::Input => ExpressionBody::Input,
                        ExpressionBody::Value(v) => {
                            ExpressionBody::Value(self.budget.copy_value(v)?)
                        }
                        ExpressionBody::Expression(e) => {
                            ExpressionBody::Expression(self.budget.copy_expression(e)?)
                        }
                    }
                } else if self.bindings.get(&root.owner).is_some_and(|s| s.input) {
                    ExpressionBody::Input
                } else if temporary {
                    continue;
                } else {
                    return Err(error(ec::EXP_NAME, Some(root.location)));
                };
                (
                    S::Binding(self.budget.copy_type(&self.resolver.types[&root.owner])?),
                    Some(body),
                )
            } else {
                let Some(definition) = self
                    .resolver
                    .definitions
                    .iter()
                    .find(|d| d.owner == root.owner)
                else {
                    return Err(error(ec::EXP_NAME, Some(root.location)));
                };
                (
                    S::Definition(self.budget.copy_definition(
                        &definition.definition.body,
                        project_limits,
                        self.captured.composition_limits().policy(),
                    )?),
                    None,
                )
            };
            let public = if selected.is_some_and(|(owner, _)| *owner == root.owner) {
                false
            } else {
                root.public
            };
            grow(&mut maps, 1)?;
            maps.push(ProjectSourceMap {
                declaration: self.budget.copy_symbol(&root.owner)?,
                location: root.location,
            });
            grow(&mut declarations, 1)?;
            declarations.push(ExpressionDeclaration {
                identity: self.budget.copy_symbol(&root.owner)?,
                public,
                signature,
                body,
            });
        }
        if let Some((owner, expression)) = selected
            && !declarations.iter().any(|d| d.identity == *owner)
        {
            self.budget.retain::<ExpressionDeclaration>(1)?;
            grow(&mut declarations, 1)?;
            declarations.push(ExpressionDeclaration {
                identity: self.budget.copy_symbol(owner)?,
                public: false,
                signature: S::Binding(self.budget.copy_type(&expression.ty)?),
                body: Some(ExpressionBody::Expression(
                    self.budget.copy_expression(expression)?,
                )),
            });
        }
        declarations.sort_unstable_by(|a, b| a.identity.cmp(&b.identity));
        self.charge_captured_companions()?;
        let companions = super::super::nominal::companions::captured_companions(&self.resolver)
            .map_err(composition_error)?;
        let mut public_templates = Vec::new();
        if !temporary {
            for (owner, template) in &self.typed_templates {
                if !self.templates[owner].root.public {
                    continue;
                }
                self.budget.retain::<PublicTemplate>(1)?;
                grow(&mut public_templates, 1)?;
                public_templates.push(copy_template(template, &mut self.budget)?);
                self.budget.retain::<ProjectSourceMap>(1)?;
                grow(&mut maps, 1)?;
                maps.push(ProjectSourceMap {
                    declaration: self.budget.copy_symbol(owner)?,
                    location: self.templates[owner].root.location,
                });
            }
        }
        maps.sort_unstable_by(|a, b| a.declaration.cmp(&b.declaration));
        let cap = self.captured.resource_facts();
        let limits = self.semantic_limits().project;
        let mut ir = ExpressionProjectIr {
            // Placeholder facts/digest are filled by checked derivation before public return.
            schema: self
                .budget
                .copy_text(expression_profile::PROJECT_IR_SCHEMA)?,
            modules: companions.modules,
            declarations,
            vocabularies: alloc(copy_slice(self.captured.catalogue().bundles()))?,
            interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
            sources: companions.sources,
            source_maps: maps,
            provenance: Vec::new(),
            limits,
            resources: ProjectResourceFacts {
                source_units: cap.source_units(),
                source_bytes: cap.total_source_bytes(),
                vocabulary_units: cap.vocabulary_units(),
                vocabulary_bytes: cap.total_vocabulary_bytes(),
                declarations: 0,
                import_edges: 0,
                value_nodes: 0,
            },
            vocabulary_sources: companions.vocabulary_sources,
            origins: Vec::new(),
            composition_limits: self.captured.composition_limits().policy(),
            composition_resources: CompositionResourceFacts::from_values([0; 7]),
            semantic_profiles: alloc(expression_profile::SemanticProfiles::try_frozen())?,
            public_templates,
            expression_limits: self.captured.expression_limits(),
            expansion_limits: self.captured.expansion_limits(),
            expression_resources: ExpressionResourceFacts::default(),
            expression_attributions: Vec::new(),
            expansion_attributions: Vec::new(),
        };
        if !temporary {
            for attribution in &self.expansions {
                self.budget.charge(ir.declarations.len() as u64 + 1)?;
                if ir
                    .declarations
                    .iter()
                    .any(|d| d.identity == attribution.owner)
                {
                    self.budget.retain::<ExpansionAttribution>(1)?;
                    self.budget
                        .retain::<SourceLocation>(attribution.arguments.len())?;
                    self.budget
                        .retain::<u64>(attribution.original_axis_indices.len())?;
                    grow(&mut ir.expansion_attributions, 1)?;
                    ir.expansion_attributions.push(ExpansionAttribution {
                        owner: self.budget.copy_symbol(&attribution.owner)?,
                        application: attribution.application,
                        definition: attribution.definition,
                        arguments: {
                            let mut arguments = Vec::new();
                            grow(&mut arguments, attribution.arguments.len())?;
                            arguments.extend_from_slice(&attribution.arguments);
                            arguments
                        },
                        original_axis_indices: alloc(copy_slice(
                            &attribution.original_axis_indices,
                        ))?,
                    });
                }
            }
            ir.expansion_attributions.sort_unstable_by(|a, b| {
                (
                    &a.owner,
                    a.application,
                    a.definition,
                    &a.arguments,
                    &a.original_axis_indices,
                )
                    .cmp(&(
                        &b.owner,
                        b.application,
                        b.definition,
                        &b.arguments,
                        &b.original_axis_indices,
                    ))
            });
            ir.expansion_attributions.dedup_by(|a, b| a == b);
            for d in &ir.declarations {
                let Some(source_map) = ir.source_maps.iter().find(|m| m.declaration == d.identity)
                else {
                    return Err(error(ec::EXP_SYNTAX, None));
                };
                let location = source_map.location;
                signature_edges(
                    &mut self.budget,
                    &d.identity,
                    &d.signature,
                    location,
                    &mut ir.provenance,
                )?;
                match &d.body {
                    Some(ExpressionBody::Expression(e)) => {
                        tree_edges(
                            &mut self.budget,
                            &d.identity,
                            e,
                            location,
                            &mut ir.provenance,
                        )?;
                        self.budget.retain::<ExpressionAttribution>(1)?;
                        grow(&mut ir.expression_attributions, 1)?;
                        ir.expression_attributions.push(ExpressionAttribution {
                            owner: self.budget.copy_symbol(&d.identity)?,
                            node_path: Vec::new(),
                            occurrence: Some(location),
                            related_public_locations: Vec::new(),
                        });
                    }
                    Some(ExpressionBody::Value(value)) => {
                        value_edges(
                            &mut self.budget,
                            &d.identity,
                            value,
                            location,
                            &mut ir.provenance,
                        )?;
                        self.source_reuse_edges(&d.identity, value, &mut ir.provenance)?;
                        materialized_origins(
                            &mut self.budget,
                            &d.identity,
                            value,
                            &mut Vec::new(),
                            location,
                            &mut ir.origins,
                        )?;
                    }
                    _ => {}
                }
            }
            for template in &ir.public_templates {
                let Some(source_map) = ir
                    .source_maps
                    .iter()
                    .find(|m| m.declaration == template.identity)
                else {
                    return Err(error(ec::EXP_SYNTAX, None));
                };
                let location = source_map.location;
                type_edges(
                    &mut self.budget,
                    &template.identity,
                    &template.result_type,
                    false,
                    location,
                    &mut ir.provenance,
                )?;
                tree_edges(
                    &mut self.budget,
                    &template.identity,
                    &template.body,
                    location,
                    &mut ir.provenance,
                )?;
                for p in &template.parameters {
                    type_edges(
                        &mut self.budget,
                        &template.identity,
                        &p.ty,
                        false,
                        location,
                        &mut ir.provenance,
                    )?;
                    if let Some(default) = &p.default {
                        tree_edges(
                            &mut self.budget,
                            &template.identity,
                            default,
                            location,
                            &mut ir.provenance,
                        )?;
                    }
                }
                self.budget.retain::<ExpressionAttribution>(1)?;
                grow(&mut ir.expression_attributions, 1)?;
                ir.expression_attributions.push(ExpressionAttribution {
                    owner: self.budget.copy_symbol(&template.identity)?,
                    node_path: Vec::new(),
                    occurrence: Some(location),
                    related_public_locations: Vec::new(),
                });
            }
            ir.provenance.sort_unstable_by(|a, b| {
                (&a.from, a.kind, &a.to, a.location).cmp(&(&b.from, b.kind, &b.to, b.location))
            });
            ir.provenance.dedup();
            ir.expression_attributions
                .sort_unstable_by(|a, b| (&a.owner, &a.node_path).cmp(&(&b.owner, &b.node_path)));
        }
        let mut policy = ir.composition_limits;
        policy.work = policy.work.min(self.budget.remaining().work);
        let inspected =
            neutral_ir::composition::project::inspect_expression_project_resources_with_work(
                &ir,
                limits,
                policy,
                self.budget.cancel,
            )
            .map_err(|_| self.budget.resource_error())?;
        self.budget.charge(inspected.work)?;
        ir.composition_resources = inspected.facts;
        ir.resources.declarations = ir.declarations.len() as u64;
        let Some(import_edges) = ir
            .modules
            .iter()
            .map(|m| m.imports.len() as u64)
            .try_fold(0_u64, u64::checked_add)
        else {
            return Err(error(ec::EXP_LIMIT, None));
        };
        ir.resources.import_edges = import_edges;
        ir.resources.value_nodes = ir.composition_resources.retained_value_nodes;
        // These are retained model counts, not the cumulative work already spent assembling it.
        Ok(ir)
    }
}

/// Copies retained exported contracts through fallible tree/type ownership.
fn copy_template(template: &PublicTemplate, budget: &mut Budget<'_>) -> Result<PublicTemplate, E> {
    let mut parameters = Vec::new();
    budget.retain::<TemplateParameter>(template.parameters.len())?;
    grow(&mut parameters, template.parameters.len())?;
    for p in &template.parameters {
        parameters.push(TemplateParameter {
            name: budget.copy_text(&p.name)?,
            phase: p.phase,
            ty: budget.copy_type(&p.ty)?,
            default: p
                .default
                .as_ref()
                .map(|e| budget.copy_expression(e))
                .transpose()?,
        });
    }
    Ok(PublicTemplate {
        identity: budget.copy_symbol(&template.identity)?,
        parameters,
        result_type: budget.copy_type(&template.result_type)?,
        body: budget.copy_expression(&template.body)?,
    })
}
/// Collects typed target owners for request-local static dependency resolution.
fn collect_targets(
    budget: &mut Budget<'_>,
    e: &Expression,
    targets: &mut Vec<ModuleSymbolIdentity>,
) -> Result<(), E> {
    budget.charge(1)?;
    match &e.node {
        N::Value(v) => collect_value_targets(budget, v, targets)?,
        N::Input(owner) | N::Output { target: owner, .. } => {
            grow(targets, 1)?;
            budget.retain::<ModuleSymbolIdentity>(1)?;
            targets.push(budget.copy_symbol(owner)?);
        }
        N::Unary { operand, .. } | N::Text(operand) => collect_targets(budget, operand, targets)?,
        N::Binary { left, right, .. } => {
            collect_targets(budget, left, targets)?;
            collect_targets(budget, right, targets)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            collect_targets(budget, guard, targets)?;
            collect_targets(budget, then_branch, targets)?;
            collect_targets(budget, else_branch, targets)?;
        }
        N::Match { scrutinee, arms } => {
            collect_targets(budget, scrutinee, targets)?;
            for arm in arms {
                collect_targets(budget, &arm.body, targets)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                collect_targets(budget, item, targets)?;
            }
        }
        N::Record(fields) => {
            for (_, item) in fields {
                if let Some(item) = item {
                    collect_targets(budget, item, targets)?;
                }
            }
        }
        N::Variant { payload, .. } => collect_targets(budget, payload, targets)?,
        N::Bound { .. } => {}
    }
    Ok(())
}
/// Retains only explicit reference owners from already bounded materialized values.
fn collect_value_targets(
    budget: &mut Budget<'_>,
    v: &V,
    targets: &mut Vec<ModuleSymbolIdentity>,
) -> Result<(), E> {
    budget.charge(1)?;
    match v {
        V::Reference(owner) => {
            grow(targets, 1)?;
            budget.retain::<ModuleSymbolIdentity>(1)?;
            targets.push(budget.copy_symbol(owner)?);
        }
        V::List(items) => {
            for item in items {
                collect_value_targets(budget, item, targets)?;
            }
        }
        V::Record(fields) => {
            for (_, item) in fields {
                if let Some(item) = item {
                    collect_value_targets(budget, item, targets)?;
                }
            }
        }
        V::Variant { payload, .. } => collect_value_targets(budget, payload, targets)?,
        _ => {}
    }
    Ok(())
}
/// Records one original source-owned structural dependency occurrence.
pub(super) fn edge(
    budget: &mut Budget<'_>,
    owner: &ModuleSymbolIdentity,
    target: &ModuleSymbolIdentity,
    kind: Edge,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    budget.retain::<ProjectProvenance>(1)?;
    grow(edges, 1)?;
    edges.push(ProjectProvenance {
        from: budget.copy_symbol(owner)?,
        to: budget.copy_symbol(target)?,
        kind,
        location,
    });
    Ok(())
}
/// Retains nominal type dependencies separately from value/reference occurrences.
fn type_edges(
    budget: &mut Budget<'_>,
    owner: &ModuleSymbolIdentity,
    ty: &T,
    reference: bool,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    budget.charge(1)?;
    match ty {
        T::Nominal(target) => edge(
            budget,
            owner,
            target,
            if reference {
                Edge::ReferenceType
            } else {
                Edge::Type
            },
            location,
            edges,
        )?,
        T::Ref(inner) => type_edges(budget, owner, inner, true, location, edges)?,
        T::List(inner) | T::Nullable(inner) => {
            type_edges(budget, owner, inner, reference, location, edges)?;
        }
        _ => {}
    }
    Ok(())
}
/// Retains all nominal dependencies from a complete definition or binding signature.
fn signature_edges(
    budget: &mut Budget<'_>,
    owner: &ModuleSymbolIdentity,
    signature: &S,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    budget.charge(1)?;
    match signature {
        S::Binding(ty) => type_edges(budget, owner, ty, false, location, edges)?,
        S::Definition(B::Record(fields)) => {
            for field in fields {
                type_edges(budget, owner, &field.ty, false, location, edges)?;
            }
        }
        S::Definition(B::Variant(alts)) => {
            for alt in alts {
                type_edges(budget, owner, &alt.ty, false, location, edges)?;
            }
        }
    }
    Ok(())
}
/// Retains actual immutable reference targets without evaluating external ports.
fn value_edges(
    budget: &mut Budget<'_>,
    owner: &ModuleSymbolIdentity,
    value: &V,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    budget.charge(1)?;
    match value {
        V::Reference(target) => edge(budget, owner, target, Edge::Reference, location, edges)?,
        V::List(items) => {
            for item in items {
                value_edges(budget, owner, item, location, edges)?;
            }
        }
        V::Record(fields) => {
            for (_, value) in fields {
                if let Some(value) = value {
                    value_edges(budget, owner, value, location, edges)?;
                }
            }
        }
        V::Variant { payload, .. } => value_edges(budget, owner, payload, location, edges)?,
        _ => {}
    }
    Ok(())
}
/// Retains structural type/ref dependencies from every branch of a typed tree.
fn tree_edges(
    budget: &mut Budget<'_>,
    owner: &ModuleSymbolIdentity,
    e: &Expression,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    budget.charge(1)?;
    type_edges(budget, owner, &e.ty, false, location, edges)?;
    match &e.node {
        N::Value(v) => value_edges(budget, owner, v, location, edges)?,
        N::Output { result_type, .. } => {
            type_edges(budget, owner, result_type, false, location, edges)?;
        }
        N::Unary { operand, .. } | N::Text(operand) => {
            tree_edges(budget, owner, operand, location, edges)?;
        }
        N::Binary { left, right, .. } => {
            tree_edges(budget, owner, left, location, edges)?;
            tree_edges(budget, owner, right, location, edges)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            tree_edges(budget, owner, guard, location, edges)?;
            tree_edges(budget, owner, then_branch, location, edges)?;
            tree_edges(budget, owner, else_branch, location, edges)?;
        }
        N::Match { scrutinee, arms } => {
            tree_edges(budget, owner, scrutinee, location, edges)?;
            for arm in arms {
                type_edges(budget, owner, &arm.payload_type, false, location, edges)?;
                tree_edges(budget, owner, &arm.body, location, edges)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                tree_edges(budget, owner, item, location, edges)?;
            }
        }
        N::Record(fields) => {
            for (_, value) in fields {
                if let Some(value) = value {
                    tree_edges(budget, owner, value, location, edges)?;
                }
            }
        }
        N::Variant { payload, .. } => tree_edges(budget, owner, payload, location, edges)?,
        N::Input(_) | N::Bound { .. } => {}
    }
    Ok(())
}
/// Records canonical supplied/null/omitted occurrences at checked original source coordinates.
fn materialized_origins(
    budget: &mut Budget<'_>,
    owner: &ModuleSymbolIdentity,
    value: &V,
    path: &mut Vec<P>,
    location: SourceLocation,
    origins: &mut Vec<CompositionOrigin>,
) -> Result<(), E> {
    budget.charge(1)?;
    budget.retain::<CompositionOrigin>(1)?;
    charge_path(budget, path)?;
    grow(origins, 1)?;
    // Reuse the real retained source occurrence; do not manufacture child byte spans from value shape.
    origins.push(CompositionOrigin {
        binding: budget.copy_symbol(owner)?,
        path: alloc(path.try_clone())?,
        kind: if matches!(value, V::Null) {
            K::ExplicitNull
        } else {
            K::Supplied
        },
        attribution: Some(CompositionAttribution::Source(location)),
    });
    match value {
        V::List(items) => {
            for (i, item) in items.iter().enumerate() {
                budget.retain::<P>(1)?;
                grow(path, 1)?;
                path.push(P::Element(i as u64));
                materialized_origins(budget, owner, item, path, location, origins)?;
                path.pop();
            }
        }
        V::Record(fields) => {
            for (name, item) in fields {
                budget.retain::<P>(1)?;
                grow(path, 1)?;
                path.push(P::Field(budget.copy_text(name)?));
                if let Some(item) = item {
                    materialized_origins(budget, owner, item, path, location, origins)?;
                } else {
                    budget.retain::<CompositionOrigin>(1)?;
                    charge_path(budget, path)?;
                    grow(origins, 1)?;
                    origins.push(CompositionOrigin {
                        binding: budget.copy_symbol(owner)?,
                        path: alloc(path.try_clone())?,
                        kind: K::OmittedOptional,
                        attribution: Some(CompositionAttribution::Source(location)),
                    });
                }
                path.pop();
            }
        }
        V::Variant { payload, .. } => {
            budget.retain::<P>(1)?;
            grow(path, 1)?;
            path.push(P::Payload);
            materialized_origins(budget, owner, payload, path, location, origins)?;
            path.pop();
        }
        _ => {}
    }
    Ok(())
}

/// Path copies retain each owned field spelling before allocating a companion.
fn charge_path(budget: &mut Budget<'_>, path: &[P]) -> Result<(), E> {
    budget.retain::<P>(path.len())?;
    for segment in path {
        if let P::Field(name) = segment {
            budget.bytes(name.len() as u64)?;
        }
    }
    Ok(())
}
