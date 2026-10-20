// SPDX-License-Identifier: Apache-2.0

//! Complete raw assembly and private static evaluation through the shared semantic kernel.
//!
//! Assemble declarations, retained expression trees and original-byte companions
//! atomically under shared limits. Static evaluation may materialize a value;
//! deferred evaluation remains explicit consumer work. Recomputed resource and
//! identity facts describe the complete project, never a selected public view.

use super::{
    Arc, B, CancellationToken, CompositionOrigin, CompositionResourceFacts, E, Edge,
    ExpansionAttribution, Expression, ExpressionAttribution, ExpressionBody, ExpressionDeclaration,
    ExpressionLimits, ExpressionProjectIr, ExpressionResourceFacts, ModuleSymbolIdentity, N,
    NormalizedBinding, ProjectProvenance, ProjectResourceFacts, ProjectSourceMap, PublicTemplate,
    Root, S, SemanticDigest, SourceLocation, State, T, TemplateParameter, TryClone, V, alloc,
    composition_error, copy_slice, derive_expression_resources, error, evaluation_error,
    expression_profile, grow, normalize_binding_with_budget, text, validate_expression_model,
};
use crate::frontend::expression_codes as ec;
use neutral_ir::composition::{
    ValueOriginKind as K, ValuePathSegment as P, project::CompositionAttribution,
};

impl State<'_, '_> {
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
                .map_err(|e| evaluation_error(e, location))?;
        self.budget.debit(before, remaining)?;
        Ok(result)
    }

    /// Normalizes an actual typed subtree in a closed request-local semantic model.
    /// No unavailable declaration is relabelled as null or as an external input.
    pub(super) fn static_value(
        &mut self,
        owner: &Root,
        expression: &Expression,
    ) -> Result<Option<V>, E> {
        self.budget.charge(1)?;
        let mut references = Vec::new();
        collect_targets(expression, &mut references)?;
        for target in references {
            if target != owner.owner && !self.bodies.contains_key(&target) {
                self.ensure_binding(&target)?;
            }
        }
        let expression = self.clone_expression(expression)?;
        let ir = self.assemble(true, Some((&owner.owner, &expression)))?;
        let model = validate_expression_model(
            alloc(Arc::try_new(ir))?,
            self.semantic_limits(),
            self.budget.cancel,
        )
        .map_err(|e| error(e.code(), Some(owner.location)))?;
        match self.normalize(&model, &owner.owner, Some(owner.location))? {
            NormalizedBinding::Value(v) => Ok(Some(v)),
            NormalizedBinding::Residual(_) => Ok(None),
        }
    }

    /// Assembles complete raw sections; only the independent reader can grant artifact authority.
    #[allow(clippy::too_many_lines)] // Keep the closed section grammar together while source integration is qualified.
    pub(super) fn assemble(
        &mut self,
        temporary: bool,
        selected: Option<(&ModuleSymbolIdentity, &Expression)>,
    ) -> Result<ExpressionProjectIr, E> {
        self.budget.charge(1)?;
        let mut declarations = Vec::new();
        let mut maps = Vec::new();
        let roots = self.resolver.retained_roots().map_err(composition_error)?;
        for root in roots {
            let (signature, body) = if let Some((_, expression)) =
                selected.filter(|(owner, _)| **owner == root.owner)
            {
                (
                    S::Binding(alloc(expression.ty.try_clone())?),
                    Some(ExpressionBody::Expression(
                        self.clone_expression(expression)?,
                    )),
                )
            } else if root.ty.is_some() {
                let body = if let Some(body) = self.bodies.get(&root.owner) {
                    match body {
                        ExpressionBody::Input => ExpressionBody::Input,
                        ExpressionBody::Value(v) => ExpressionBody::Value(
                            neutral_ir::expression::try_clone_value_bounded(
                                v,
                                self.budget.expression,
                                self.budget.cancel,
                            )
                            .map_err(|_| error(ec::EXP_LIMIT, Some(root.location)))?,
                        ),
                        ExpressionBody::Expression(e) => ExpressionBody::Expression(
                            e.try_clone_bounded(self.budget.expression, self.budget.cancel)
                                .map_err(|_| error(ec::EXP_LIMIT, Some(root.location)))?,
                        ),
                    }
                } else if self.bindings.get(&root.owner).is_some_and(|s| s.input) {
                    ExpressionBody::Input
                } else if temporary {
                    continue;
                } else {
                    return Err(error(ec::EXP_NAME, Some(root.location)));
                };
                (
                    S::Binding(alloc(self.resolver.types[&root.owner].try_clone())?),
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
                    S::Definition(alloc(definition.definition.body.try_clone())?),
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
                declaration: alloc(root.owner.try_clone())?,
                location: root.location,
            });
            grow(&mut declarations, 1)?;
            declarations.push(ExpressionDeclaration {
                identity: root.owner,
                public,
                signature,
                body,
            });
        }
        if let Some((owner, expression)) = selected
            && !declarations.iter().any(|d| d.identity == *owner)
        {
            grow(&mut declarations, 1)?;
            declarations.push(ExpressionDeclaration {
                identity: alloc(owner.try_clone())?,
                public: false,
                signature: S::Binding(alloc(expression.ty.try_clone())?),
                body: Some(ExpressionBody::Expression(
                    self.clone_expression(expression)?,
                )),
            });
        }
        declarations.sort_unstable_by(|a, b| a.identity.cmp(&b.identity));
        let companions = super::super::assembly::captured_companions(&self.resolver)
            .map_err(composition_error)?;
        let mut public_templates = Vec::new();
        if !temporary {
            for (owner, template) in &self.typed_templates {
                if !self.templates[owner].root.public {
                    continue;
                }
                grow(&mut public_templates, 1)?;
                public_templates.push(copy_template(
                    template,
                    self.budget.expression,
                    self.budget.cancel,
                )?);
                grow(&mut maps, 1)?;
                maps.push(ProjectSourceMap {
                    declaration: alloc(owner.try_clone())?,
                    location: self.templates[owner].root.location,
                });
            }
        }
        maps.sort_unstable_by(|a, b| a.declaration.cmp(&b.declaration));
        let cap = self.captured.resource_facts();
        let limits = self.semantic_limits().project;
        let mut ir = ExpressionProjectIr {
            schema: alloc(text(expression_profile::PROJECT_IR_SCHEMA))?,
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
                if ir
                    .declarations
                    .iter()
                    .any(|d| d.identity == attribution.owner)
                {
                    grow(&mut ir.expansion_attributions, 1)?;
                    ir.expansion_attributions.push(ExpansionAttribution {
                        owner: alloc(attribution.owner.try_clone())?,
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
                signature_edges(&d.identity, &d.signature, location, &mut ir.provenance)?;
                match &d.body {
                    Some(ExpressionBody::Expression(e)) => {
                        tree_edges(&d.identity, e, location, &mut ir.provenance)?;
                        grow(&mut ir.expression_attributions, 1)?;
                        ir.expression_attributions.push(ExpressionAttribution {
                            owner: alloc(d.identity.try_clone())?,
                            node_path: Vec::new(),
                            occurrence: Some(location),
                            related_public_locations: Vec::new(),
                        });
                    }
                    Some(ExpressionBody::Value(value)) => {
                        value_edges(&d.identity, value, location, &mut ir.provenance)?;
                        self.source_reuse_edges(&d.identity, value, &mut ir.provenance)?;
                        materialized_origins(
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
                    &template.identity,
                    &template.result_type,
                    false,
                    location,
                    &mut ir.provenance,
                )?;
                tree_edges(
                    &template.identity,
                    &template.body,
                    location,
                    &mut ir.provenance,
                )?;
                for p in &template.parameters {
                    type_edges(
                        &template.identity,
                        &p.ty,
                        false,
                        location,
                        &mut ir.provenance,
                    )?;
                    if let Some(default) = &p.default {
                        tree_edges(&template.identity, default, location, &mut ir.provenance)?;
                    }
                }
                grow(&mut ir.expression_attributions, 1)?;
                ir.expression_attributions.push(ExpressionAttribution {
                    owner: alloc(template.identity.try_clone())?,
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
        ir.composition_resources =
            neutral_ir::expression::project::inspect_expression_project_resources(
                &ir,
                limits,
                ir.composition_limits,
                self.budget.cancel,
            )
            .map_err(|_| error(ec::EXP_LIMIT, None))?;
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
        ir.expression_resources =
            derive_expression_resources(&ir, self.semantic_limits(), self.budget.cancel)
                .map_err(|e| error(e.code(), None))?;
        Ok(ir)
    }
}

/// Copies retained exported contracts through fallible tree/type ownership.
fn copy_template(
    template: &PublicTemplate,
    limits: ExpressionLimits,
    cancel: &CancellationToken,
) -> Result<PublicTemplate, E> {
    let mut parameters = Vec::new();
    grow(&mut parameters, template.parameters.len())?;
    for p in &template.parameters {
        parameters.push(TemplateParameter {
            name: alloc(p.name.try_clone())?,
            phase: p.phase,
            ty: alloc(p.ty.try_clone())?,
            default: p
                .default
                .as_ref()
                .map(|e| {
                    e.try_clone_bounded(limits, cancel)
                        .map_err(|_| error(ec::EXP_LIMIT, None))
                })
                .transpose()?,
        });
    }
    Ok(PublicTemplate {
        identity: alloc(template.identity.try_clone())?,
        parameters,
        result_type: alloc(template.result_type.try_clone())?,
        body: template
            .body
            .try_clone_bounded(limits, cancel)
            .map_err(|_| error(ec::EXP_LIMIT, None))?,
    })
}
/// Collects typed target owners for request-local static dependency resolution.
fn collect_targets(e: &Expression, targets: &mut Vec<ModuleSymbolIdentity>) -> Result<(), E> {
    match &e.node {
        N::Value(v) => collect_value_targets(v, targets)?,
        N::Input(owner) | N::Output { target: owner, .. } => {
            grow(targets, 1)?;
            targets.push(alloc(owner.try_clone())?);
        }
        N::Unary { operand, .. } | N::Text(operand) => collect_targets(operand, targets)?,
        N::Binary { left, right, .. } => {
            collect_targets(left, targets)?;
            collect_targets(right, targets)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            collect_targets(guard, targets)?;
            collect_targets(then_branch, targets)?;
            collect_targets(else_branch, targets)?;
        }
        N::Match { scrutinee, arms } => {
            collect_targets(scrutinee, targets)?;
            for arm in arms {
                collect_targets(&arm.body, targets)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                collect_targets(item, targets)?;
            }
        }
        N::Record(fields) => {
            for (_, item) in fields {
                if let Some(item) = item {
                    collect_targets(item, targets)?;
                }
            }
        }
        N::Variant { payload, .. } => collect_targets(payload, targets)?,
        N::Bound { .. } => {}
    }
    Ok(())
}
/// Retains only explicit reference owners from already bounded materialized values.
fn collect_value_targets(v: &V, targets: &mut Vec<ModuleSymbolIdentity>) -> Result<(), E> {
    match v {
        V::Reference(owner) => {
            grow(targets, 1)?;
            targets.push(alloc(owner.try_clone())?);
        }
        V::List(items) => {
            for item in items {
                collect_value_targets(item, targets)?;
            }
        }
        V::Record(fields) => {
            for (_, item) in fields {
                if let Some(item) = item {
                    collect_value_targets(item, targets)?;
                }
            }
        }
        V::Variant { payload, .. } => collect_value_targets(payload, targets)?,
        _ => {}
    }
    Ok(())
}
/// Records one original source-owned structural dependency occurrence.
pub(super) fn edge(
    owner: &ModuleSymbolIdentity,
    target: &ModuleSymbolIdentity,
    kind: Edge,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    grow(edges, 1)?;
    edges.push(ProjectProvenance {
        from: alloc(owner.try_clone())?,
        to: alloc(target.try_clone())?,
        kind,
        location,
    });
    Ok(())
}
/// Retains nominal type dependencies separately from value/reference occurrences.
fn type_edges(
    owner: &ModuleSymbolIdentity,
    ty: &T,
    reference: bool,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    match ty {
        T::Nominal(target) => edge(
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
        T::Ref(inner) => type_edges(owner, inner, true, location, edges)?,
        T::List(inner) | T::Nullable(inner) => {
            type_edges(owner, inner, reference, location, edges)?;
        }
        _ => {}
    }
    Ok(())
}
/// Retains all nominal dependencies from a complete definition or binding signature.
fn signature_edges(
    owner: &ModuleSymbolIdentity,
    signature: &S,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    match signature {
        S::Binding(ty) => type_edges(owner, ty, false, location, edges)?,
        S::Definition(B::Record(fields)) => {
            for field in fields {
                type_edges(owner, &field.ty, false, location, edges)?;
            }
        }
        S::Definition(B::Variant(alts)) => {
            for alt in alts {
                type_edges(owner, &alt.ty, false, location, edges)?;
            }
        }
    }
    Ok(())
}
/// Retains actual immutable reference targets without evaluating external ports.
fn value_edges(
    owner: &ModuleSymbolIdentity,
    value: &V,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    match value {
        V::Reference(target) => edge(owner, target, Edge::Reference, location, edges)?,
        V::List(items) => {
            for item in items {
                value_edges(owner, item, location, edges)?;
            }
        }
        V::Record(fields) => {
            for (_, value) in fields {
                if let Some(value) = value {
                    value_edges(owner, value, location, edges)?;
                }
            }
        }
        V::Variant { payload, .. } => value_edges(owner, payload, location, edges)?,
        _ => {}
    }
    Ok(())
}
/// Retains structural type/ref dependencies from every branch of a typed tree.
fn tree_edges(
    owner: &ModuleSymbolIdentity,
    e: &Expression,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) -> Result<(), E> {
    type_edges(owner, &e.ty, false, location, edges)?;
    match &e.node {
        N::Value(v) => value_edges(owner, v, location, edges)?,
        N::Output { result_type, .. } => type_edges(owner, result_type, false, location, edges)?,
        N::Unary { operand, .. } | N::Text(operand) => tree_edges(owner, operand, location, edges)?,
        N::Binary { left, right, .. } => {
            tree_edges(owner, left, location, edges)?;
            tree_edges(owner, right, location, edges)?;
        }
        N::If {
            guard,
            then_branch,
            else_branch,
        } => {
            tree_edges(owner, guard, location, edges)?;
            tree_edges(owner, then_branch, location, edges)?;
            tree_edges(owner, else_branch, location, edges)?;
        }
        N::Match { scrutinee, arms } => {
            tree_edges(owner, scrutinee, location, edges)?;
            for arm in arms {
                type_edges(owner, &arm.payload_type, false, location, edges)?;
                tree_edges(owner, &arm.body, location, edges)?;
            }
        }
        N::List(items) | N::Interpolate(items) => {
            for item in items {
                tree_edges(owner, item, location, edges)?;
            }
        }
        N::Record(fields) => {
            for (_, value) in fields {
                if let Some(value) = value {
                    tree_edges(owner, value, location, edges)?;
                }
            }
        }
        N::Variant { payload, .. } => tree_edges(owner, payload, location, edges)?,
        N::Input(_) | N::Bound { .. } => {}
    }
    Ok(())
}
/// Records canonical supplied/null/omitted occurrences at checked original source coordinates.
fn materialized_origins(
    owner: &ModuleSymbolIdentity,
    value: &V,
    path: &mut Vec<P>,
    location: SourceLocation,
    origins: &mut Vec<CompositionOrigin>,
) -> Result<(), E> {
    grow(origins, 1)?;
    origins.push(CompositionOrigin {
        binding: alloc(owner.try_clone())?,
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
                grow(path, 1)?;
                path.push(P::Element(i as u64));
                materialized_origins(owner, item, path, location, origins)?;
                path.pop();
            }
        }
        V::Record(fields) => {
            for (name, item) in fields {
                grow(path, 1)?;
                path.push(P::Field(alloc(name.try_clone())?));
                if let Some(item) = item {
                    materialized_origins(owner, item, path, location, origins)?;
                } else {
                    grow(origins, 1)?;
                    origins.push(CompositionOrigin {
                        binding: alloc(owner.try_clone())?,
                        path: alloc(path.try_clone())?,
                        kind: K::OmittedOptional,
                        attribution: Some(CompositionAttribution::Source(location)),
                    });
                }
                path.pop();
            }
        }
        V::Variant { payload, .. } => {
            grow(path, 1)?;
            path.push(P::Payload);
            materialized_origins(owner, payload, path, location, origins)?;
            path.pop();
        }
        _ => {}
    }
    Ok(())
}
