// SPDX-License-Identifier: Apache-2.0

//! Explicit capture/2 source typing; the old parser and compiler entry points stay unchanged.

use super::{
    Token, TokenKind, TypeExpr, is_requirement_or_import, lexer, parse_complete_type, parse_name,
    parse_reference,
};
use crate::{CapturedCompositionProject, ModuleGraph};
use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceLocation, profile::V1_SOURCE_PROFILE,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, ClosedValue, CompositionAlternative, CompositionBinding,
        CompositionBody as B, CompositionDefinition, CompositionField, CompositionValue,
        FieldPresence, FieldRestrictions, SourceCompositionDefinition, profile,
        project::{
            CompositionDeclaration, CompositionOrigin, CompositionProjectIr as Ir,
            CompositionResourceFacts, CompositionSignature as S, inspect_composition_ir,
        },
    },
    language::{is_protected_name, is_snake_name, is_upper_name},
    project::{
        PROJECT_MAX_DEPTH, ProjectLimits, ProjectModule, ProjectProvenance, ProjectResourceFacts,
        ProjectSource, ProjectSourceMap, ProjectVocabularySource,
    },
    project_identity::{IdentityLimits, composition_interface},
    project_interface::{ProjectPublicEdgeKind as Edge, ProjectPublicType as T},
};
use neutral_vocabulary::composition::{
    CompositionError, ValidatedCompositionScope, validate_composition_bindings,
    validate_composition_scope,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Safe successor compilation failure; no variant publishes partial logical output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionCompileFailure {
    /// Frozen source diagnostic classification.
    pub code: &'static str,
    /// Checked original-byte occurrence when available.
    pub location: Option<SourceLocation>,
}
impl CompositionCompileFailure {
    /// Returns the explicitly selected successor whole-request envelope.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        profile::PROJECT_RESULT_SCHEMA
    }
}
/// Short request-local diagnostic result name.
type E = CompositionCompileFailure;
/// Checked occurrence locations; raw source stays private to the compiler.
#[path = "composition_attribution.rs"]
mod attribution;
/// Source diagnostic spellings; package releases never select them.
#[path = "composition_codes.rs"]
pub mod codes;

/// Parsed declaration; initializer/member tokens are private and reused within this request only.
#[derive(Clone)]
struct Root {
    /// Stable resolved owner.
    owner: ModuleSymbolIdentity,
    /// Explicit source visibility.
    public: bool,
    /// Complete original declaration span.
    location: SourceLocation,
    /// Resolved binding type, absent for nominal declarations.
    ty: Option<TypeExpr>,
    /// Record fields or variant alternatives in source order.
    fields: Vec<(String, TypeExpr)>,
    /// Initializer or field/default body tokens.
    tokens: Vec<Token>,
    /// Nominal variant rather than record.
    variant: bool,
}

/// Request-local resolver; no filesystem, URL acquisition, ambient environment or persistent cache.
struct Resolver<'a> {
    /// Complete explicit captured closure.
    captured: &'a CapturedCompositionProject,
    /// Independently graph-checked explicit imports/aliases.
    graph: ModuleGraph,
    /// Canonical parsed declarations, including private/disconnected units.
    roots: BTreeMap<ModuleSymbolIdentity, Root>,
    /// Exact resolved binding signatures.
    types: BTreeMap<ModuleSymbolIdentity, T>,
    /// Complete source contract bodies, before or after default materialization.
    definitions: Vec<SourceCompositionDefinition>,
    /// Request-only memoized immutable initializer values.
    values: BTreeMap<ModuleSymbolIdentity, V>,
    /// Active ordinary reuse chain; references never enter it.
    active: BTreeSet<ModuleSymbolIdentity>,
    /// Source-accounted ordinary reuse occurrences.
    provenance: Vec<ProjectProvenance>,
    /// Charged source/resolution traversal work.
    work: u64,
    /// Explicit cooperative cancellation.
    cancel: &'a CancellationToken,
}

/// Compiles every capture/2 source under the shared record/variant/default/constraint model.
///
/// The caller must independently open the returned producer data through the
/// successor reader before encoding it. No root selection or host acquisition is
/// part of compilation, and no package release or source header selects this API.
///
/// # Errors
/// Rejects graph, grammar, visibility, cycles, type/default/value, bounds or
/// cancellation failures atomically without returning any successful prefix.
pub fn compile_composition_project(
    captured: &CapturedCompositionProject,
    cancel: &CancellationToken,
) -> Result<Arc<Ir>, E> {
    let graph = captured.module_graph(cancel).map_err(|failure| E {
        code: match failure
            .diagnostics()
            .first()
            .map(crate::module_graph::ModuleGraphDiagnostic::code)
        {
            Some(crate::module_graph::diagnostics::CANCELLED) => codes::CANCELLED,
            Some(crate::module_graph::diagnostics::LIMIT_EXCEEDED) => codes::LIMIT,
            _ => codes::INVALID_SOURCE,
        },
        location: failure
            .diagnostics()
            .first()
            .and_then(crate::module_graph::ModuleGraphDiagnostic::source_location),
    })?;
    let mut resolver = Resolver {
        captured,
        graph,
        roots: BTreeMap::new(),
        types: BTreeMap::new(),
        definitions: Vec::new(),
        values: BTreeMap::new(),
        active: BTreeSet::new(),
        provenance: Vec::new(),
        work: 0,
        cancel,
    };
    resolver.step(1, None)?;
    for source in captured.sources() {
        resolver.step(source.bytes().len() as u64, None)?;
        for root in parse(source)? {
            if resolver.roots.len() as u64 >= captured.limits().values().declarations {
                return Err(fail(codes::LIMIT, Some(root.location)));
            }
            if resolver
                .roots
                .insert(root.owner.clone(), root.clone())
                .is_some()
            {
                return Err(fail(codes::DUPLICATE_MEMBER, Some(root.location)));
            }
        }
    }
    let roots = resolver.roots.values().cloned().collect::<Vec<_>>();
    resolver.signatures(&roots)?;
    // Visibility and embedded closure precede all source default/value interpretation.
    validate_composition_scope(
        captured.catalogue(),
        resolver.definitions.clone(),
        captured.composition_limits(),
        cancel,
    )
    .map_err(|e| contract(&e, None))?;
    resolver.source_defaults(&roots)?;
    let scope = Arc::new(
        validate_composition_scope(
            captured.catalogue(),
            resolver.definitions.clone(),
            captured.composition_limits(),
            cancel,
        )
        .map_err(|e| contract(&e, None))?,
    );
    resolver.definitions = scope.sources().to_vec();
    let mut bindings = Vec::new();
    for root in roots.iter().filter(|r| r.ty.is_some()) {
        let value = resolver.binding(&root.owner, 0)?;
        bindings.push(CompositionBinding {
            owner: root.owner.clone(),
            public: root.public,
            ty: resolver.types[&root.owner].clone(),
            value,
        });
    }
    let bindings = validate_composition_bindings(
        scope.clone(),
        &bindings,
        captured.composition_limits(),
        cancel,
    )
    .map_err(|e| contract(&e, None))?;
    assemble(&mut resolver, &scope, &bindings)
}

/// Builds complete canonical logical data and separately retained source/resource companions.
fn assemble(
    resolver: &mut Resolver<'_>,
    scope: &ValidatedCompositionScope,
    bindings: &neutral_vocabulary::composition::ValidatedCompositionBindings,
) -> Result<Arc<Ir>, E> {
    let captured = resolver.captured;
    let cap = captured.limits().values();
    let LoweredDeclarations {
        declarations,
        maps,
        origins,
    } = declarations(resolver, scope, bindings)?;
    let CapturedCompanions {
        modules,
        sources,
        vocabulary_sources,
    } = captured_companions(resolver)?;
    complete_provenance(resolver, &declarations, bindings);
    let limits = ProjectLimits {
        modules: cap.source_units,
        declarations: cap.declarations,
        import_edges: cap.import_edges,
        nodes: captured.composition_limits().value_nodes,
        text_bytes: cap
            .total_source_bytes
            .checked_add(cap.total_vocabulary_bytes)
            .ok_or_else(|| fail(codes::LIMIT, None))?,
        artifact_bytes: cap.output_bytes,
    };
    let facts = captured.resource_facts();
    let mut ir = Ir {
        schema: profile::PROJECT_IR_SCHEMA.to_owned(),
        modules,
        declarations,
        vocabularies: scope.catalogue().bundles().to_vec(),
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources,
        source_maps: maps,
        provenance: resolver.provenance.clone(),
        limits,
        resources: ProjectResourceFacts {
            source_units: facts.source_units(),
            source_bytes: facts.total_source_bytes(),
            vocabulary_units: facts.vocabulary_units(),
            vocabulary_bytes: facts.total_vocabulary_bytes(),
            declarations: resolver.roots.len() as u64,
            import_edges: resolver.graph.edges().len() as u64,
            value_nodes: 0,
        },
        vocabulary_sources,
        origins,
        composition_limits: captured.composition_limits().policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
    };
    ir.composition_resources =
        inspect_composition_ir(&ir, limits, ir.composition_limits, resolver.cancel).map_err(
            |e| {
                fail(
                    match e {
                        neutral_ir::composition::project::CompositionShapeError::Cancelled => {
                            codes::CANCELLED
                        }
                        neutral_ir::composition::project::CompositionShapeError::Limit => {
                            codes::LIMIT
                        }
                    },
                    None,
                )
            },
        )?;
    ir.resources.value_nodes = ir.composition_resources.retained_value_nodes;
    ir.interface_digest = composition_interface(
        &ir,
        IdentityLimits {
            bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
            nodes: captured.composition_limits().work,
        },
        resolver.cancel,
    )
    .map_err(|e| {
        fail(
            match e {
                neutral_ir::project_identity::IdentityError::Cancelled => codes::CANCELLED,
                _ => codes::LIMIT,
            },
            None,
        )
    })?
    .identity();
    resolver.step(1, None)?;
    Ok(Arc::new(ir))
}

/// Completes canonical source type and reference edges after all values have been validated.
fn complete_provenance(
    resolver: &mut Resolver<'_>,
    declarations: &[CompositionDeclaration],
    bindings: &neutral_vocabulary::composition::ValidatedCompositionBindings,
) {
    for d in declarations {
        let root = &resolver.roots[&d.identity];
        let types = match &d.signature {
            S::Binding(t) => vec![t],
            S::Definition(B::Record(fs)) => fs.iter().map(|f| &f.ty).collect(),
            S::Definition(B::Variant(as_)) => as_.iter().map(|a| &a.ty).collect(),
        };
        for t in types {
            type_edges(
                &d.identity,
                t,
                false,
                root.location,
                &mut resolver.provenance,
            );
        }
    }
    for b in bindings.bindings() {
        for r in b.references() {
            resolver.provenance.push(ProjectProvenance {
                from: b.binding().owner.clone(),
                to: r.target.clone(),
                kind: Edge::Reference,
                location: resolver.roots[&b.binding().owner].location,
            });
        }
    }
    resolver.provenance.sort_by(|a, b| {
        (&a.from, a.kind, &a.to, a.location).cmp(&(&b.from, b.kind, &b.to, b.location))
    });
    resolver.provenance.dedup();
}

/// Canonical source declarations with separate occurrence and coverage companions.
struct LoweredDeclarations {
    /// Complete private/public signatures and values.
    declarations: Vec<CompositionDeclaration>,
    /// Complete original-byte declaration coverage.
    maps: Vec<ProjectSourceMap>,
    /// Non-semantic supplied/default/null/omitted occurrences.
    origins: Vec<CompositionOrigin>,
}
/// Projects already validated complete source contracts and materialized bindings.
fn declarations(
    resolver: &mut Resolver<'_>,
    scope: &ValidatedCompositionScope,
    bindings: &neutral_vocabulary::composition::ValidatedCompositionBindings,
) -> Result<LoweredDeclarations, E> {
    let mut declarations = Vec::new();
    let mut maps = Vec::new();
    let mut origins = Vec::new();
    let roots = resolver.roots.values().cloned().collect::<Vec<_>>();
    for root in &roots {
        let (signature, value) = if root.ty.is_some() {
            let b = bindings
                .bindings()
                .iter()
                .find(|b| b.binding().owner == root.owner)
                .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(root.location)))?;
            for o in b.origins() {
                origins.push(CompositionOrigin {
                    binding: root.owner.clone(),
                    path: o.path.clone(),
                    kind: o.kind,
                    attribution: attribution::origin(
                        resolver,
                        root,
                        &b.binding().ty,
                        &o.path,
                        o.kind,
                    )?,
                });
            }
            (
                S::Binding(b.binding().ty.clone()),
                Some(b.binding().value.clone()),
            )
        } else {
            let d = scope
                .sources()
                .iter()
                .find(|d| d.owner == root.owner)
                .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(root.location)))?;
            (S::Definition(d.definition.body.clone()), None)
        };
        declarations.push(CompositionDeclaration {
            identity: root.owner.clone(),
            public: root.public,
            signature,
            value,
        });
        maps.push(ProjectSourceMap {
            declaration: root.owner.clone(),
            location: root.location,
        });
    }
    Ok(LoweredDeclarations {
        declarations,
        maps,
        origins,
    })
}
/// Exact captured accounting and complete logical module topology.
struct CapturedCompanions {
    /// All modules, including disconnected and private-only units.
    modules: Vec<ProjectModule>,
    /// Logical source identities and byte lengths.
    sources: Vec<ProjectSource>,
    /// Exact locked vocabulary capture evidence.
    vocabulary_sources: Vec<ProjectVocabularySource>,
}
/// Derives checked captured companions without acquiring any input.
fn captured_companions(resolver: &Resolver<'_>) -> Result<CapturedCompanions, E> {
    let captured = resolver.captured;
    let modules = captured
        .sources()
        .iter()
        .map(|s| {
            let mut imports = resolver
                .graph
                .edges()
                .iter()
                .filter(|e| e.from() == s.module_id())
                .map(|e| e.target().to_owned())
                .collect::<Vec<_>>();
            imports.sort();
            ProjectModule {
                identity: LogicalModuleIdentity::new(V1_SOURCE_PROFILE, s.module_id()),
                imports,
            }
        })
        .collect::<Vec<_>>();
    let sources = captured
        .sources()
        .iter()
        .map(|s| ProjectSource {
            module: s.module_id().to_owned(),
            source_id: s.source_id().to_owned(),
            digest: s.digest(),
            byte_len: s.bytes().len() as u64,
        })
        .collect::<Vec<_>>();
    let vocabulary_sources = captured
        .catalogue()
        .bundles()
        .iter()
        .map(|b| {
            let source = captured
                .vocabularies()
                .iter()
                .find(|v| v.lock().identity() == b.identity.identity())
                .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, None))?;
            Ok(ProjectVocabularySource {
                identity: b.identity.identity().to_owned(),
                version: b.identity.version().to_owned(),
                digest: b.identity.content_digest(),
                byte_len: source.bytes().len() as u64,
            })
        })
        .collect::<Result<Vec<_>, E>>()?;
    Ok(CapturedCompanions {
        modules,
        sources,
        vocabulary_sources,
    })
}
impl Resolver<'_> {
    /// Resolves all source signature owners before any value evaluation.
    fn signatures(&mut self, roots: &[Root]) -> Result<(), E> {
        // Resolve every signature before interpreting a default or initializer.
        for root in roots {
            if let Some(ty) = &root.ty {
                let ty = self.ty(root, ty)?;
                self.types.insert(root.owner.clone(), ty);
            } else {
                let mut fields = Vec::new();
                let mut alternatives = Vec::new();
                for (name, ty) in &root.fields {
                    let ty = self.ty(root, ty)?;
                    if root.variant {
                        alternatives.push(CompositionAlternative {
                            tag: name.clone(),
                            ty,
                        });
                    } else {
                        fields.push(CompositionField {
                            name: name.clone(),
                            ty,
                            presence: FieldPresence::Required,
                            restrictions: FieldRestrictions::default(),
                            default: None,
                        });
                    }
                }
                fields.sort_by(|a, b| a.name.cmp(&b.name));
                alternatives.sort_by(|a, b| a.tag.cmp(&b.tag));
                self.definitions.push(SourceCompositionDefinition {
                    owner: root.owner.clone(),
                    definition: CompositionDefinition {
                        name: root.owner.declaration_name().to_owned(),
                        public: root.public,
                        body: if root.variant {
                            B::Variant(alternatives)
                        } else {
                            B::Record(fields)
                        },
                    },
                });
            }
        }
        Ok(())
    }

    /// Resolves closed source defaults against the complete nominal type set.
    fn source_defaults(&mut self, roots: &[Root]) -> Result<(), E> {
        // Parse closed defaults against the complete set of source and vocabulary types.
        let mut defaults = Vec::new();
        for root in roots.iter().filter(|r| r.ty.is_none() && !r.variant) {
            for member in split(&root.tokens)? {
                if let Some(eq) = member
                    .iter()
                    .position(|t| matches!(t.kind, TokenKind::Equals))
                {
                    let Some(Token {
                        kind: TokenKind::Identifier(name),
                        ..
                    }) = member.get(eq.saturating_sub(1))
                    else {
                        return Err(fail(codes::INVALID_SOURCE, Some(root.location)));
                    };
                    let ty = self
                        .definitions
                        .iter()
                        .find(|d| d.owner == root.owner)
                        .and_then(|d| match &d.definition.body {
                            B::Record(fs) => {
                                fs.iter().find(|f| f.name == *name).map(|f| f.ty.clone())
                            }
                            B::Variant(_) => None,
                        })
                        .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(root.location)))?;
                    let value = self.value(root, &ty, &member[eq + 1..], true, 0)?;
                    defaults.push((root.owner.clone(), name.clone(), closed(value)?));
                }
            }
        }
        for (owner, name, value) in defaults {
            let d = self
                .definitions
                .iter_mut()
                .find(|d| d.owner == owner)
                .ok_or_else(|| fail(codes::INVALID_SOURCE, None))?;
            let B::Record(fields) = &mut d.definition.body else {
                return Err(fail(codes::INVALID_SOURCE, None));
            };
            let f = fields
                .iter_mut()
                .find(|f| f.name == name)
                .ok_or_else(|| fail(codes::INVALID_SOURCE, None))?;
            f.presence = FieldPresence::Defaulted;
            f.default = Some(value);
        }
        Ok(())
    }
    /// Charges source/token/key/value work before recursion or proportional materialization.
    fn step(&mut self, n: u64, location: Option<SourceLocation>) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, location));
        }
        self.work = self
            .work
            .checked_add(n)
            .ok_or_else(|| fail(codes::LIMIT, location))?;
        if self.work > self.captured.composition_limits().work {
            return Err(fail(codes::LIMIT, location));
        }
        Ok(())
    }
    /// Resolves explicit local/import/vocabulary qualifiers without path/URL interpretation.
    fn name(
        &mut self,
        root: &Root,
        alias: Option<&str>,
        name: &str,
        span: ByteSpan,
        nominal: bool,
    ) -> Result<ModuleSymbolIdentity, E> {
        let module = root.owner.module().module_name();
        self.step(
            (name.len() + alias.map_or(0, str::len)) as u64,
            Some(SourceLocation::new(root.location.source(), span)),
        )?;
        let target = if let Some(alias) = alias {
            self.graph
                .edges()
                .iter()
                .find(|e| e.from() == module && e.alias() == alias)
                .map(crate::module_graph::GraphEdge::target)
                .ok_or_else(|| {
                    fail(
                        codes::INCOMPATIBLE_VALUE,
                        Some(SourceLocation::new(root.location.source(), span)),
                    )
                })?
        } else {
            module
        };
        let owner =
            ModuleSymbolIdentity::new(LogicalModuleIdentity::new(V1_SOURCE_PROFILE, target), name);
        let declaration = self.roots.get(&owner).ok_or_else(|| {
            fail(
                codes::INCOMPATIBLE_VALUE,
                Some(SourceLocation::new(root.location.source(), span)),
            )
        })?;
        if nominal == declaration.ty.is_some() {
            return Err(fail(codes::INCOMPATIBLE_VALUE, Some(root.location)));
        }
        if target != module && !declaration.public {
            return Err(fail(codes::PRIVATE_EXPOSURE, Some(root.location)));
        }
        Ok(owner)
    }
    /// Converts parsed type wrappers to exact source/vocabulary nominal owners.
    fn ty(&mut self, root: &Root, ty: &TypeExpr) -> Result<T, E> {
        self.step(1, Some(root.location))?;
        Ok(match ty {
            TypeExpr::Num => T::Num,
            TypeExpr::String => T::String,
            TypeExpr::Bool => T::Bool,
            TypeExpr::Url => T::Url,
            TypeExpr::Path => T::Path,
            TypeExpr::List(t) => T::List(Box::new(self.ty(root, t)?)),
            TypeExpr::Ref(t) => T::Ref(Box::new(self.ty(root, t)?)),
            TypeExpr::Nullable(t) => T::Nullable(Box::new(self.ty(root, t)?)),
            TypeExpr::Nominal(alias, name, span) => {
                if let Some(v) = alias.as_deref().and_then(|a| {
                    self.captured
                        .vocabulary_for_alias(root.owner.module().module_name(), a)
                }) {
                    let b = self.captured.catalogue();
                    let public = b
                        .bundles()
                        .iter()
                        .find(|b| b.identity == *v)
                        .and_then(|b| b.definitions.iter().find(|d| d.name == *name))
                        .is_some_and(|d| d.public);
                    if !public {
                        return Err(fail(
                            codes::PRIVATE_EXPOSURE,
                            Some(SourceLocation::new(root.location.source(), *span)),
                        ));
                    }
                    T::VocabularyNominal {
                        identity: v.identity().to_owned(),
                        version: v.version().to_owned(),
                        name: name.clone(),
                    }
                } else {
                    let owner = self.name(root, alias.as_deref(), name, *span, true)?;
                    T::Nominal(owner)
                }
            }
        })
    }
    /// Looks up complete nominal bodies; all alternatives are known before value interpretation.
    fn body(&mut self, ty: &T) -> Result<B, E> {
        let catalogue = self.captured.catalogue();
        let body = match ty {
            T::Nominal(owner) => self
                .definitions
                .iter()
                .find(|d| d.owner == *owner)
                .map(|d| &d.definition.body),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => catalogue
                .bundles()
                .iter()
                .find(|b| b.identity.identity() == identity && b.identity.version() == version)
                .and_then(|b| b.definitions.iter().find(|d| d.name == *name))
                .map(|d| &d.body),
            _ => None,
        }
        .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, None))?;
        let work = body_clone_work(body, self.cancel)?;
        // Charge before retaining a recursive copy; repeated nominal use must not amplify unchecked allocation.
        let remaining = self
            .captured
            .composition_limits()
            .work
            .saturating_sub(self.work);
        if work > remaining {
            return Err(fail(codes::LIMIT, None));
        }
        let copied = body.clone();
        self.step(work, None)?;
        Ok(copied)
    }
    /// Resolves ordinary reuse with explicit cycle/depth guards; references do not evaluate targets.
    fn binding(&mut self, owner: &ModuleSymbolIdentity, depth: usize) -> Result<V, E> {
        self.step(1, None)?;
        if depth > PROJECT_MAX_DEPTH {
            return Err(fail(codes::LIMIT, None));
        }
        if let Some(v) = self.values.get(owner) {
            let work = value_clone_work(v, self.cancel, symbol_work)?;
            self.step(work, None)?;
            return Ok(self.values[owner].clone());
        }
        if !self.active.insert(owner.clone()) {
            return Err(fail(codes::REUSE_CYCLE, Some(self.roots[owner].location)));
        }
        let root = self.roots[owner].clone();
        let ty = self.types[owner].clone();
        let value = self.value(&root, &ty, &root.tokens, false, 0)?;
        self.active.remove(owner);
        self.step(
            value_clone_work(&value, self.cancel, symbol_work)?,
            Some(root.location),
        )?;
        self.values.insert(owner.clone(), value.clone());
        Ok(value)
    }
    /// Types exact scalar literals without host numeric or inert-location coercion.
    fn scalar(&self, ty: &T, token: &Token, location: SourceLocation) -> Result<V, E> {
        match &token.kind {
            TokenKind::Null => {
                if matches!(ty, T::Nullable(_)) {
                    Ok(V::Null)
                } else {
                    Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)))
                }
            }
            TokenKind::True | TokenKind::False => {
                let inner = if let T::Nullable(inner) = ty {
                    inner.as_ref()
                } else {
                    ty
                };
                if !matches!(inner, T::Bool) {
                    return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
                }
                Ok(V::Bool(matches!(token.kind, TokenKind::True)))
            }
            TokenKind::Number(s) => {
                let inner = if let T::Nullable(inner) = ty {
                    inner.as_ref()
                } else {
                    ty
                };
                if !matches!(inner, T::Num) {
                    return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
                }
                ExactNumber::from_source(
                    s,
                    self.captured.composition_limits().json.numeric_digits(),
                    self.captured.composition_limits().json.numeric_scale(),
                )
                .map(V::Number)
                .map_err(|_| fail(codes::LIMIT, Some(location)))
            }
            TokenKind::StringLiteral(s) => {
                let inner = if let T::Nullable(inner) = ty {
                    inner.as_ref()
                } else {
                    ty
                };
                if !matches!(inner, T::String | T::Url | T::Path) {
                    return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
                }
                Ok(match ty {
                    T::Url => V::Url(s.value.clone()),
                    T::Path => V::Path(s.value.clone()),
                    T::Nullable(inner) if matches!(**inner, T::Url) => V::Url(s.value.clone()),
                    T::Nullable(inner) if matches!(**inner, T::Path) => V::Path(s.value.clone()),
                    _ => V::String(s.value.clone()),
                })
            }
            _ => Err(fail(codes::INVALID_SOURCE, Some(location))),
        }
    }
    /// Types a closed contextual record/variant object with complete member checks.
    fn record_value(
        &mut self,
        root: &Root,
        ty: &T,
        tokens: &[Token],
        closed: bool,
        depth: usize,
    ) -> Result<V, E> {
        let location = SourceLocation::new(root.location.source(), tokens[0].span);
        let mut members = BTreeMap::new();
        for field in split(&tokens[1..tokens.len() - 1])? {
            let [
                Token {
                    kind: TokenKind::Identifier(name),
                    ..
                },
                Token {
                    kind: TokenKind::Colon,
                    ..
                },
                rest @ ..,
            ] = field
            else {
                return Err(fail(codes::INVALID_SOURCE, Some(location)));
            };
            if members.insert(name.as_str(), rest).is_some() {
                return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
            }
        }
        match self.body(ty)? {
            B::Record(fields) => {
                if !closed
                    && fields.iter().any(|f| {
                        f.presence == FieldPresence::Required
                            && !members.contains_key(f.name.as_str())
                    })
                {
                    let end =
                        SourceLocation::new(root.location.source(), tokens[tokens.len() - 1].span);
                    return Err(fail(codes::MISSING_REQUIRED_FIELD, Some(end)));
                }
                let mut value = Vec::new();
                for (name, tokens) in members {
                    let field = fields
                        .iter()
                        .find(|f| f.name == name)
                        .ok_or_else(|| fail(codes::INCOMPATIBLE_VALUE, Some(location)))?;
                    let v = self.value(root, &field.ty, tokens, closed, depth + 1)?;
                    if field.restrictions != FieldRestrictions::default() {
                        neutral_vocabulary::composition::check_composition_field_restrictions(
                            field,
                            &v,
                            self.captured.composition_limits(),
                            self.cancel,
                        )
                        .map_err(|e| {
                            if e == CompositionError::InvalidValue {
                                fail(codes::RESTRICTION_VIOLATION, Some(location))
                            } else {
                                contract(&e, Some(location))
                            }
                        })?;
                    }
                    value.push((name.to_owned(), Some(v)));
                }
                Ok(V::Record(value))
            }
            B::Variant(alternatives) => {
                if members.len() != 2
                    || !members.contains_key(profile::TAG)
                    || !members.contains_key(profile::PAYLOAD)
                {
                    return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
                }
                let [
                    Token {
                        kind: TokenKind::StringLiteral(tag),
                        span,
                    },
                ] = members[profile::TAG]
                else {
                    return Err(fail(codes::INVALID_SOURCE, Some(location)));
                };
                let alternative = alternatives
                    .iter()
                    .find(|a| a.tag == tag.value)
                    .ok_or_else(|| {
                        fail(
                            codes::UNKNOWN_TAG,
                            Some(SourceLocation::new(root.location.source(), *span)),
                        )
                    })?;
                let payload = self.value(
                    root,
                    &alternative.ty,
                    members[profile::PAYLOAD],
                    closed,
                    depth + 1,
                )?;
                Ok(V::Variant {
                    tag: tag.value.clone(),
                    payload: Box::new(payload),
                })
            }
        }
    }

    /// Parses contextually typed values and exact selected variant payloads using the shared bodies.
    fn value(
        &mut self,
        root: &Root,
        ty: &T,
        tokens: &[Token],
        closed: bool,
        depth: usize,
    ) -> Result<V, E> {
        self.step(tokens.len() as u64 + 1, Some(root.location))?;
        if depth as u64 > self.captured.composition_limits().value_depth
            || depth > PROJECT_MAX_DEPTH
        {
            return Err(fail(codes::LIMIT, Some(root.location)));
        }
        let location = tokens.first().map_or(root.location, |t| {
            SourceLocation::new(root.location.source(), t.span)
        });
        if tokens.is_empty() {
            return Err(fail(codes::INVALID_SOURCE, Some(root.location)));
        }
        if let Some((name, end)) = parse_reference(tokens, 0)
            && end == tokens.len()
        {
            if closed {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            }
            let target = self.name(root, name.alias.as_deref(), &name.name, name.span, false)?;
            let expected = if let T::Nullable(inner) = ty {
                inner.as_ref()
            } else {
                ty
            };
            let T::Ref(expected) = expected else {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            };
            if self.types.get(&target) != Some(expected.as_ref()) {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            }
            if root.public && !self.roots[&target].public {
                return Err(fail(codes::PRIVATE_EXPOSURE, Some(location)));
            }
            return Ok(V::Reference(target));
        }
        if let Some((name, end)) = parse_name(tokens, 0)
            && end == tokens.len()
        {
            if closed {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            }
            let target = self.name(root, name.alias.as_deref(), &name.name, name.span, false)?;
            if self.types.get(&target) != Some(ty) {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            }
            self.provenance.push(ProjectProvenance {
                from: root.owner.clone(),
                to: target.clone(),
                kind: Edge::Value,
                location,
            });
            return self.binding(&target, self.active.len());
        }
        if let [token] = tokens
            && matches!(
                token.kind,
                TokenKind::Null
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Number(_)
                    | TokenKind::StringLiteral(_)
            )
        {
            return self.scalar(ty, token, location);
        }
        if let T::Nullable(inner) = ty {
            return self.value(root, inner, tokens, closed, depth);
        }
        if matches!(tokens[0].kind, TokenKind::OpenBracket)
            && matches!(
                tokens.last().map(|t| &t.kind),
                Some(TokenKind::CloseBracket)
            )
        {
            let T::List(inner) = ty else {
                return Err(fail(codes::INCOMPATIBLE_VALUE, Some(location)));
            };
            let mut values = Vec::new();
            for item in split(&tokens[1..tokens.len() - 1])? {
                values.push(self.value(root, inner, item, closed, depth + 1)?);
            }
            return Ok(V::List(values));
        }
        if matches!(tokens[0].kind, TokenKind::OpenBrace)
            && matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::CloseBrace))
        {
            return self.record_value(root, ty, tokens, closed, depth);
        }
        Err(fail(codes::INVALID_SOURCE, Some(location)))
    }
}

/// Parses declarations once while preserving physical statement and original-byte boundaries.
fn parse(source: &crate::CapturedProjectSource) -> Result<Vec<Root>, E> {
    let tokens = lexer::lex(source.bytes())
        .map_err(|e| {
            fail(
                codes::INVALID_SOURCE,
                Some(SourceLocation::new(source.digest(), e.span)),
            )
        })?
        .tokens;
    let mut statement = Vec::new();
    let mut roots = Vec::new();
    let mut depth = 0;
    let mut headers = 0;
    for token in tokens {
        if matches!(
            token.kind,
            TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
        ) {
            if depth == 0 && !statement.is_empty() {
                if headers < 2 {
                    headers += 1;
                } else if !is_requirement_or_import(&statement) {
                    roots.push(parse_declaration(source, &statement)?);
                }
                statement.clear();
            }
            continue;
        }
        match token.kind {
            TokenKind::OpenBrace | TokenKind::OpenBracket | TokenKind::OpenParen => depth += 1,
            TokenKind::CloseBrace | TokenKind::CloseBracket | TokenKind::CloseParen => {
                if depth == 0 {
                    return Err(fail(
                        codes::INVALID_SOURCE,
                        Some(SourceLocation::new(source.digest(), token.span)),
                    ));
                }
                depth -= 1;
            }
            _ => {}
        }
        if depth > PROJECT_MAX_DEPTH {
            return Err(fail(
                codes::LIMIT,
                Some(SourceLocation::new(source.digest(), token.span)),
            ));
        }
        statement.push(token);
    }
    if !statement.is_empty() {
        return Err(fail(
            codes::INVALID_SOURCE,
            Some(SourceLocation::new(source.digest(), statement[0].span)),
        ));
    }
    Ok(roots)
}

/// Counts stable owner text before retaining a copied reference key.
fn symbol_work(owner: &ModuleSymbolIdentity) -> u64 {
    (owner.module().language_behavior_version().len()
        + owner.module().module_name().len()
        + owner.declaration_name().len()) as u64
        + 1
}

/// Bounds recursive value copies iteratively before reuse/default expansion can allocate them.
fn value_clone_work<R>(
    value: &CompositionValue<R>,
    cancel: &CancellationToken,
    reference: impl Fn(&R) -> u64,
) -> Result<u64, E> {
    let mut work = 0_u64;
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        if cancel.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        let extra = match value {
            CompositionValue::Number(n) => n.coefficient().len() as u64,
            CompositionValue::String(s) | CompositionValue::Url(s) | CompositionValue::Path(s) => {
                s.len() as u64
            }
            CompositionValue::Reference(target) => reference(target),
            CompositionValue::List(items) => {
                stack
                    .try_reserve(items.len())
                    .map_err(|_| fail(codes::LIMIT, None))?;
                stack.extend(items);
                items.len() as u64
            }
            CompositionValue::Record(fields) => {
                stack
                    .try_reserve(fields.len())
                    .map_err(|_| fail(codes::LIMIT, None))?;
                let mut bytes = 0_u64;
                for (name, value) in fields {
                    bytes = bytes
                        .checked_add(name.len() as u64 + 1)
                        .ok_or_else(|| fail(codes::LIMIT, None))?;
                    if let Some(value) = value {
                        stack.push(value);
                    }
                }
                bytes
            }
            CompositionValue::Variant { tag, payload } => {
                stack.push(payload);
                tag.len() as u64
            }
            CompositionValue::Null | CompositionValue::Bool(_) => 0,
        };
        work = work
            .checked_add(extra)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| fail(codes::LIMIT, None))?;
        if work > profile::MAX_ITEMS {
            return Err(fail(codes::LIMIT, None));
        }
    }
    Ok(work)
}

/// Accounts for wrapper depth and all nominal owner text before cloning a type.
fn type_clone_work(mut ty: &T) -> u64 {
    let mut work = 1;
    loop {
        match ty {
            T::List(inner) | T::Ref(inner) | T::Nullable(inner) => {
                work += 1;
                ty = inner;
            }
            T::Nominal(owner) => return work + symbol_work(owner),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => return work + (identity.len() + version.len() + name.len()) as u64,
            _ => return work,
        }
    }
}

/// Charges copied contract members, choices and defaults independently of later materialization.
fn body_clone_work(body: &B, cancel: &CancellationToken) -> Result<u64, E> {
    let mut work = 1_u64;
    match body {
        B::Record(fields) => {
            for field in fields {
                work = work
                    .checked_add(field.name.len() as u64 + type_clone_work(&field.ty) + 1)
                    .ok_or_else(|| fail(codes::LIMIT, None))?;
                for value in field
                    .default
                    .iter()
                    .chain(field.restrictions.choices.iter().flatten())
                {
                    work = work
                        .checked_add(value_clone_work(value, cancel, |never| match *never {})?)
                        .ok_or_else(|| fail(codes::LIMIT, None))?;
                }
                for bound in [&field.restrictions.minimum, &field.restrictions.maximum]
                    .into_iter()
                    .flatten()
                {
                    work = work
                        .checked_add(bound.coefficient().len() as u64)
                        .ok_or_else(|| fail(codes::LIMIT, None))?;
                }
            }
        }
        B::Variant(alternatives) => {
            for alternative in alternatives {
                work = work
                    .checked_add(
                        alternative.tag.len() as u64 + type_clone_work(&alternative.ty) + 1,
                    )
                    .ok_or_else(|| fail(codes::LIMIT, None))?;
            }
        }
    }
    Ok(work)
}

/// Adds contextual variant syntax without reserving `variant` as a binding identifier.
fn parse_declaration(source: &crate::CapturedProjectSource, tokens: &[Token]) -> Result<Root, E> {
    let public = matches!(&tokens[0].kind,TokenKind::Identifier(s) if s==crate::language::graph_names::PUBLIC);
    let start = usize::from(public);
    let variant = matches!(tokens.get(start).map(|t|&t.kind),Some(TokenKind::Identifier(s)) if s==profile::VARIANT);
    let span = ByteSpan::new(tokens[0].span.start(), tokens[tokens.len() - 1].span.end())
        .map_err(|_| fail(codes::INVALID_SOURCE, None))?;
    let location = SourceLocation::new(source.digest(), span);
    if variant || matches!(tokens.get(start).map(|t| &t.kind), Some(TokenKind::Record)) {
        let Some(Token {
            kind: TokenKind::Identifier(name),
            ..
        }) = tokens.get(start + 1)
        else {
            return Err(fail(codes::INVALID_SOURCE, Some(location)));
        };
        if !is_upper_name(name)
            || is_protected_name(name)
            || !matches!(
                tokens.get(start + 2).map(|t| &t.kind),
                Some(TokenKind::OpenBrace)
            )
            || !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::CloseBrace))
        {
            return Err(fail(codes::INVALID_SOURCE, Some(location)));
        }
        let body = &tokens[start + 3..tokens.len() - 1];
        if variant && body.iter().any(|t| matches!(t.kind, TokenKind::Equals)) {
            return Err(fail(codes::INVALID_SOURCE, Some(location)));
        }
        let fields = parsed_fields(body)?;
        if variant && fields.is_empty() {
            return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
        }
        return Ok(Root {
            owner: ModuleSymbolIdentity::new(
                LogicalModuleIdentity::new(V1_SOURCE_PROFILE, source.module_id()),
                name,
            ),
            public,
            location,
            ty: None,
            fields,
            tokens: body.to_vec(),
            variant,
        });
    }
    let eq = tokens
        .iter()
        .position(|t| matches!(t.kind, TokenKind::Equals))
        .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(location)))?;
    if eq <= start + 1 || eq + 1 >= tokens.len() {
        return Err(fail(codes::INVALID_SOURCE, Some(location)));
    }
    let TokenKind::Identifier(name) = &tokens[eq - 1].kind else {
        return Err(fail(codes::INVALID_SOURCE, Some(location)));
    };
    if !is_snake_name(name) || is_protected_name(name) {
        return Err(fail(codes::INVALID_SOURCE, Some(location)));
    }
    let ty = parse_complete_type(&tokens[start..eq - 1])
        .ok_or_else(|| fail(codes::INVALID_SOURCE, Some(location)))?;
    Ok(Root {
        owner: ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(V1_SOURCE_PROFILE, source.module_id()),
            name,
        ),
        public,
        location,
        ty: Some(ty),
        fields: Vec::new(),
        tokens: tokens[eq + 1..].to_vec(),
        variant: false,
    })
}

/// Parses complete source member types without requiring old-profile value terminators.
fn parsed_fields(tokens: &[Token]) -> Result<Vec<(String, TypeExpr)>, E> {
    let mut result = Vec::new();
    let mut names = BTreeSet::new();
    for member in split(tokens)? {
        let end = member
            .iter()
            .position(|t| matches!(t.kind, TokenKind::Equals))
            .unwrap_or(member.len());
        if end < 2 {
            return Err(fail(codes::INVALID_SOURCE, None));
        }
        let TokenKind::Identifier(name) = &member[end - 1].kind else {
            return Err(fail(codes::INVALID_SOURCE, None));
        };
        if !is_snake_name(name) || is_protected_name(name) || !names.insert(name.clone()) {
            return Err(fail(codes::DUPLICATE_MEMBER, None));
        }
        let ty = parse_complete_type(&member[..end - 1])
            .ok_or_else(|| fail(codes::INVALID_SOURCE, None))?;
        if end < member.len() && end + 1 == member.len() {
            return Err(fail(codes::INVALID_SOURCE, None));
        }
        result.push((name.clone(), ty));
    }
    Ok(result)
}

/// Splits top-level members only, permitting one final comma and rejecting empty interior entries.
fn split(tokens: &[Token]) -> Result<Vec<&[Token]>, E> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, t) in tokens.iter().enumerate() {
        match t.kind {
            TokenKind::OpenBrace
            | TokenKind::OpenBracket
            | TokenKind::OpenParen
            | TokenKind::Less => depth += 1,
            TokenKind::CloseBrace
            | TokenKind::CloseBracket
            | TokenKind::CloseParen
            | TokenKind::Greater => {
                if depth == 0 {
                    return Err(fail(codes::INVALID_SOURCE, None));
                }
                depth -= 1;
            }
            _ => {}
        }
        if depth == 0 && matches!(t.kind, TokenKind::Comma) {
            if start == i {
                return Err(fail(codes::INVALID_SOURCE, None));
            }
            result.push(&tokens[start..i]);
            start = i + 1;
        }
    }
    if depth != 0 {
        return Err(fail(codes::INVALID_SOURCE, None));
    }
    if start < tokens.len() {
        result.push(&tokens[start..]);
    }
    Ok(result)
}

/// Converts parser-produced closed values while making identity references unrepresentable.
fn closed(v: V) -> Result<ClosedValue, E> {
    Ok(match v {
        V::Number(n) => CompositionValue::Number(n),
        V::String(s) => CompositionValue::String(s),
        V::Bool(b) => CompositionValue::Bool(b),
        V::Url(s) => CompositionValue::Url(s),
        V::Path(s) => CompositionValue::Path(s),
        V::Null => CompositionValue::Null,
        V::Reference(_) => return Err(fail(codes::INCOMPATIBLE_VALUE, None)),
        V::List(vs) => {
            CompositionValue::List(vs.into_iter().map(closed).collect::<Result<_, _>>()?)
        }
        V::Record(fs) => CompositionValue::Record(
            fs.into_iter()
                .map(|(n, v)| Ok((n, v.map(closed).transpose()?)))
                .collect::<Result<_, E>>()?,
        ),
        V::Variant { tag, payload } => CompositionValue::Variant {
            tag,
            payload: Box::new(closed(*payload)?),
        },
    })
}

/// Retains type/reference-type occurrence dependencies without embedding referenced targets.
fn type_edges(
    from: &ModuleSymbolIdentity,
    ty: &T,
    reference: bool,
    location: SourceLocation,
    edges: &mut Vec<ProjectProvenance>,
) {
    match ty {
        T::Nominal(to) => edges.push(ProjectProvenance {
            from: from.clone(),
            to: to.clone(),
            kind: if reference {
                Edge::ReferenceType
            } else {
                Edge::Type
            },
            location,
        }),
        T::Ref(t) => type_edges(from, t, true, location, edges),
        T::List(t) | T::Nullable(t) => type_edges(from, t, reference, location, edges),
        _ => {}
    }
}
/// Maps shared semantic failures without exposing private contract text.
fn contract(e: &CompositionError, location: Option<SourceLocation>) -> E {
    fail(
        match e {
            CompositionError::Cancelled => codes::CANCELLED,
            CompositionError::Limit
            | CompositionError::Allocation
            | CompositionError::InvalidLimits => codes::LIMIT,
            CompositionError::PrivateType => codes::PRIVATE_EXPOSURE,
            CompositionError::EmbeddedCycle => codes::EMBEDDED_CYCLE,
            CompositionError::InvalidRestrictions | CompositionError::DuplicateChoice => {
                codes::RESTRICTION_VIOLATION
            }
            _ => codes::INCOMPATIBLE_VALUE,
        },
        location,
    )
}
/// Creates one non-authoritative original-byte diagnostic.
fn fail(code: &'static str, location: Option<SourceLocation>) -> E {
    E { code, location }
}
