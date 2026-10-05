// SPDX-License-Identifier: Apache-2.0

//! Complete contextual project lowering over frozen captured inputs.
//!
//! Semantic resolution establishes legal names, types, and dependency order;
//! lowering materializes values under those types. Ordinary reuse copies immutable
//! meaning, whereas `ref` retains a typed identity edge. Only after all defaults
//! and bindings validate are source/processing companions assembled for publication.

use super::{
    Arc, BTreeMap, BTreeSet, CancellationToken, CapturedProject, LogicalModuleIdentity,
    ModuleContext, ModuleGraph, ModuleSymbolIdentity, NameOccurrence, ProjectDependencyKind,
    ProjectPublicEdgeKind, ProjectPublicField, ProjectPublicType, ProjectSemanticFailure,
    ProjectSemanticModel, ProjectSymbolKind, ProjectVocabularySet, Root, SourceLocation, Token,
    TokenKind, V1_SOURCE_PROFILE, analyze_project_semantics_with_parser, key, parse_name,
    parse_reference, parse_roots, public_type, resolve_name, validate_project_vocabularies,
};
use neutral_ir::{
    ExactNumber,
    project::{
        PROJECT_IR_SCHEMA, PROJECT_MAX_DEPTH, PROJECT_RESULT_SCHEMA, ProjectDeclaration, ProjectIr,
        ProjectLimits, ProjectModule, ProjectProvenance, ProjectResourceFacts, ProjectSource,
        ProjectSourceMap, ProjectValue, ProjectVocabularyRecord, ProjectVocabularySource,
    },
    project_interface::ProjectPublicSignature,
};
use neutral_vocabulary::ProjectVocabularyType;

/// Stable complete-project lowering failure categories.
pub mod codes {
    /// A contextual value, default, or reference has an incompatible type.
    pub const INVALID_VALUE: &str = "NEU-PIR-001";
    /// Explicit publication bounds were exceeded.
    pub const LIMIT: &str = "NEU-PIR-002";
    /// Cancellation prevented authoritative publication.
    pub const CANCELLED: &str = "NEU-PIR-003";
}

/// Versioned, non-authoritative failure; no variant contains partial project IR.
#[derive(Clone, Debug)]
pub enum ProjectCompileFailure {
    /// Captured import graph failed validation.
    Graph(crate::ModuleGraphFailure),
    /// Complete declaration semantics failed validation.
    Semantics(ProjectSemanticFailure),
    /// Contextual lowering failed at an original source occurrence.
    Lowering {
        /// Stable fail-closed classification.
        code: &'static str,
        /// Original source occurrence when available.
        location: Option<SourceLocation>,
    },
}

impl ProjectCompileFailure {
    /// Returns the result envelope schema, independent of package versioning.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        PROJECT_RESULT_SCHEMA
    }
}

/// Compiles every supplied module to complete, fully typed project data.
///
/// Private and disconnected units participate just like public ones. This entry
/// point has no root-selection argument: consumers derive views after complete
/// compilation and independent reader validation, never by pruning input here.
///
/// # Errors
/// Returns no authoritative partial artifact on graph, semantic, value, limit,
/// or cancellation failure. Performs no acquisition or ambient host I/O.
pub fn compile_project(
    captured: &CapturedProject,
    cancellation: &CancellationToken,
) -> Result<Arc<ProjectIr>, ProjectCompileFailure> {
    compile_project_with_parser(captured, cancellation, &mut |source| {
        parse_roots(source.module_id(), source.digest(), source.bytes())
    })
}

/// Runs fresh graph, semantic, and contextual phases over an explicit unit parser.
///
/// The parser hook is private and is the only cache seam. Recompute all later
/// phases under current inputs and controls; a cache hit must not preserve old
/// vocabulary visibility, dependency conclusions, provenance, or resource facts.
pub(super) fn compile_project_with_parser(
    captured: &CapturedProject,
    cancellation: &CancellationToken,
    parse: &mut impl FnMut(&crate::CapturedProjectSource) -> Result<Vec<Root>, ProjectSemanticFailure>,
) -> Result<Arc<ProjectIr>, ProjectCompileFailure> {
    let graph = captured
        .module_graph(cancellation)
        .map_err(ProjectCompileFailure::Graph)?;
    let resolved = analyze_project_semantics_with_parser(captured, &graph, cancellation, parse)
        .map_err(ProjectCompileFailure::Semantics)?;
    let model = resolved.model;
    let roots = resolved.roots;
    let vocabularies =
        validate_project_vocabularies(captured).map_err(|_| fail(codes::INVALID_VALUE, None))?;
    let mut modules = BTreeMap::new();
    for source in captured.sources() {
        modules.insert(
            source.module_id().to_owned(),
            ModuleContext {
                digest: source.digest(),
                aliases: graph
                    .edges()
                    .iter()
                    .filter(|edge| edge.from() == source.module_id())
                    .map(|edge| (edge.alias().to_owned(), edge.target().to_owned()))
                    .collect(),
            },
        );
    }
    let controls = captured.limits().values();
    let limits = ProjectLimits {
        modules: controls.source_units,
        declarations: controls.declarations,
        import_edges: controls.import_edges,
        nodes: controls.output_bytes,
        text_bytes: controls.total_source_bytes.max(controls.output_bytes),
        artifact_bytes: controls.output_bytes,
    };
    let mut lowering = Lowering {
        roots: &roots,
        modules: &modules,
        vocabularies: &vocabularies,
        values: BTreeMap::new(),
        defaults: BTreeMap::new(),
        cancellation,
        remaining: controls.output_bytes,
        numeric_limit: controls.source_bytes_per_unit,
    };
    // Validate defaults even if no binding uses them; unused declarations are
    // still part of the complete project and cannot shelter invalid values.
    for root in roots
        .values()
        .filter(|root| root.symbol.kind == ProjectSymbolKind::Record)
    {
        lowering.record_defaults(root.symbol.identity(), 0)?;
    }
    // Ordinary value dependencies have already been ordered by semantics. A
    // reused value must be materialized before a dependent reuse reads it.
    for identity in model.value_order() {
        let root = &roots[&key(identity)];
        let ty = public_type(
            root.declared_type
                .as_ref()
                .ok_or_else(|| fail(codes::INVALID_VALUE, Some(root.symbol.source_location())))?,
            identity.module().module_name(),
            &modules,
            &vocabularies,
        );
        let value = lowering.value(&root.value, &ty, identity.module().module_name(), false, 0)?;
        lowering.values.insert(identity.clone(), value);
    }
    let declarations = lower_declarations(&mut lowering);
    if cancellation.is_cancelled() {
        return Err(fail(codes::CANCELLED, None));
    }
    assemble_project(
        captured,
        &graph,
        &model,
        declarations,
        &vocabularies,
        limits,
    )
}

/// Collects fully materialized declarations in stable identity order.
fn lower_declarations(lowering: &mut Lowering<'_>) -> Vec<ProjectDeclaration> {
    lowering
        .roots
        .values()
        .map(|root| {
            let identity = root.symbol.identity().clone();
            let signature = if let Some(ty) = &root.declared_type {
                ProjectPublicSignature::Binding(public_type(
                    ty,
                    identity.module().module_name(),
                    lowering.modules,
                    lowering.vocabularies,
                ))
            } else {
                let mut fields = root
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        ProjectPublicField::new(
                            name,
                            public_type(
                                ty,
                                identity.module().module_name(),
                                lowering.modules,
                                lowering.vocabularies,
                            ),
                        )
                    })
                    .collect::<Vec<_>>();
                fields.sort_by(|left, right| left.name().cmp(right.name()));
                ProjectPublicSignature::Record(fields)
            };
            ProjectDeclaration {
                public: root.symbol.is_public(),
                value: lowering.values.remove(&identity),
                defaults: lowering.defaults.remove(&identity).unwrap_or_default(),
                identity,
                signature,
            }
        })
        .collect::<Vec<_>>()
}

/// Removes non-semantic import aliases while retaining every supplied module.
fn lower_modules(graph: &ModuleGraph) -> Vec<ProjectModule> {
    graph
        .modules()
        .iter()
        .map(|module| {
            let imports = graph
                .edges()
                .iter()
                .filter(|edge| edge.from() == module.module_id())
                .map(|edge| edge.target().to_owned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            ProjectModule {
                identity: LogicalModuleIdentity::new(V1_SOURCE_PROFILE, module.module_id()),
                imports,
            }
        })
        .collect::<Vec<_>>()
}

/// Retains exact source occurrences independently of semantic edge identity.
fn lower_provenance(model: &ProjectSemanticModel) -> Vec<ProjectProvenance> {
    let mut provenance = model
        .dependencies()
        .iter()
        .map(|edge| ProjectProvenance {
            from: edge.from().clone(),
            to: edge.to().clone(),
            kind: match edge.kind() {
                ProjectDependencyKind::Type => ProjectPublicEdgeKind::Type,
                ProjectDependencyKind::ReferenceType => ProjectPublicEdgeKind::ReferenceType,
                ProjectDependencyKind::Value => ProjectPublicEdgeKind::Value,
                ProjectDependencyKind::Reference => ProjectPublicEdgeKind::Reference,
            },
            location: edge.source_location(),
        })
        .collect::<Vec<_>>();
    provenance.sort_by(|left, right| {
        (&left.from, left.kind, &left.to, left.location).cmp(&(
            &right.from,
            right.kind,
            &right.to,
            right.location,
        ))
    });
    provenance.dedup();
    provenance
}

/// Assembles complete immutable publication data after all contextual checks.
///
/// This is the publication boundary, not another parser. Meaning and companions
/// come from the same accepted capture so source locations and resource facts
/// cannot accidentally describe a prior successful request.
fn assemble_project(
    captured: &CapturedProject,
    graph: &ModuleGraph,
    model: &ProjectSemanticModel,
    declarations: Vec<ProjectDeclaration>,
    vocabularies: &ProjectVocabularySet,
    limits: ProjectLimits,
) -> Result<Arc<ProjectIr>, ProjectCompileFailure> {
    let value_nodes = declarations
        .iter()
        .flat_map(|decl| {
            decl.value
                .iter()
                .chain(decl.defaults.iter().map(|(_, value)| value))
        })
        .try_fold(0_u64, |sum, value| sum.checked_add(nodes(value)))
        .ok_or_else(|| fail(codes::LIMIT, None))?;
    let project_modules = lower_modules(graph);
    let import_edges = project_modules
        .iter()
        .map(|module| module.imports.len() as u64)
        .sum();
    let sources = captured
        .sources()
        .iter()
        .map(|source| ProjectSource {
            module: source.module_id().to_owned(),
            source_id: source.source_id().to_owned(),
            digest: source.digest(),
            byte_len: source.bytes().len() as u64,
        })
        .collect();
    let source_maps = model
        .symbols()
        .iter()
        .map(|symbol| ProjectSourceMap {
            declaration: symbol.identity().clone(),
            location: symbol.source_location(),
        })
        .collect();
    let provenance = lower_provenance(model);
    let mut ir = ProjectIr {
        schema: PROJECT_IR_SCHEMA.to_owned(),
        modules: project_modules,
        declarations,
        vocabulary_records: vocabulary_records(vocabularies),
        public_interface: model.public_interface().as_ref().clone(),
        sources,
        source_maps,
        provenance,
        limits,
        resources: ProjectResourceFacts {
            source_units: captured.resource_facts().source_units(),
            source_bytes: captured.resource_facts().total_source_bytes(),
            vocabulary_units: captured.resource_facts().vocabulary_units(),
            vocabulary_bytes: captured.resource_facts().total_vocabulary_bytes(),
            declarations: model.symbols().len() as u64,
            import_edges,
            value_nodes,
        },
        vocabulary_sources: captured
            .vocabularies()
            .iter()
            .map(|vocabulary| ProjectVocabularySource {
                identity: vocabulary.identity().to_owned(),
                version: vocabulary.lock().version().to_owned(),
                digest: neutral_core::VocabularyContentDigest::from_bytes(vocabulary.bytes()),
                byte_len: vocabulary.bytes().len() as u64,
            })
            .collect(),
    };
    ir.public_interface = ir
        .recompute_public_interface()
        .map_err(|_| fail(codes::LIMIT, None))?;
    if !ir.within_schema_limits(limits) {
        return Err(fail(codes::LIMIT, None));
    }
    Ok(Arc::new(ir))
}

/// Explicit context and aggregate work budget for contextual values and defaults.
struct Lowering<'a> {
    /// Complete declaration set.
    roots: &'a BTreeMap<(String, String), Root>,
    /// Canonical imported name resolution.
    modules: &'a BTreeMap<String, ModuleContext>,
    /// Canonical locked vocabulary contracts.
    vocabularies: &'a ProjectVocabularySet,
    /// Dependency-first immutable binding values.
    values: BTreeMap<ModuleSymbolIdentity, ProjectValue>,
    /// Memoized validated closed record defaults.
    defaults: BTreeMap<ModuleSymbolIdentity, Vec<(String, ProjectValue)>>,
    /// Explicit cancellation control.
    cancellation: &'a CancellationToken,
    /// Remaining bounded lowering work and textual retention budget.
    remaining: u64,
    /// Source-size-derived independent exact-number bound.
    numeric_limit: u64,
}

impl Lowering<'_> {
    /// Charges work before allocation or recursion, checking cancellation.
    fn charge(&mut self, cost: u64) -> Result<(), ProjectCompileFailure> {
        if self.cancellation.is_cancelled() {
            return Err(fail(codes::CANCELLED, None));
        }
        self.remaining = self
            .remaining
            .checked_sub(cost)
            .ok_or_else(|| fail(codes::LIMIT, None))?;
        Ok(())
    }

    /// Materializes one expected type without accepting unresolved names or syntax.
    ///
    /// Context chooses how a literal is interpreted: record fields, nullable
    /// values, vocabulary defaults, and references must match the already resolved
    /// type. Recursive work spends the shared budget and checks depth rather than
    /// treating each nested value as an independent fresh allowance.
    fn value(
        &mut self,
        tokens: &[Token],
        ty: &ProjectPublicType,
        module: &str,
        closed: bool,
        depth: usize,
    ) -> Result<ProjectValue, ProjectCompileFailure> {
        if depth > PROJECT_MAX_DEPTH {
            return Err(fail(codes::LIMIT, None));
        }
        self.charge(1)?;
        let digest = self.modules[module].digest;
        let invalid = || {
            fail(
                codes::INVALID_VALUE,
                tokens
                    .first()
                    .map(|token| SourceLocation::new(digest, token.span)),
            )
        };
        let ty = if let ProjectPublicType::Nullable(inner) = ty {
            if matches!(
                tokens,
                [Token {
                    kind: TokenKind::Null,
                    ..
                }]
            ) {
                return Ok(ProjectValue::Null);
            }
            inner.as_ref()
        } else {
            ty
        };
        if let Some((occurrence, end)) = parse_name(tokens, 0)
            && end == tokens.len()
        {
            if closed {
                return Err(invalid());
            }
            let target = self.name(&occurrence, module)?;
            let root = &self.roots[&key(&target)];
            let actual = public_type(
                root.declared_type.as_ref().ok_or_else(invalid)?,
                target.module().module_name(),
                self.modules,
                self.vocabularies,
            );
            if actual != *ty {
                return Err(invalid());
            }
            let value = self.values.get(&target).ok_or_else(invalid)?;
            let cost = value_cost(value);
            self.charge(cost)?;
            return Ok(self.values[&target].clone());
        }
        match (ty, tokens) {
            (ProjectPublicType::Ref(inner), _) if !closed => {
                let (occurrence, end) = parse_reference(tokens, 0).ok_or_else(invalid)?;
                if end != tokens.len() {
                    return Err(invalid());
                }
                let target = self.name(&occurrence, module)?;
                let target_root = &self.roots[&key(&target)];
                let actual = public_type(
                    target_root.declared_type.as_ref().ok_or_else(invalid)?,
                    target.module().module_name(),
                    self.modules,
                    self.vocabularies,
                );
                if actual != **inner {
                    return Err(invalid());
                }
                Ok(ProjectValue::Reference(target))
            }
            (ProjectPublicType::List(inner), _) if enclosed(tokens, false) => {
                let mut values = Vec::new();
                for item in split_items(&tokens[1..tokens.len() - 1])? {
                    values.push(self.value(item, inner, module, closed, depth + 1)?);
                }
                Ok(ProjectValue::List(values))
            }
            (ProjectPublicType::Nominal(_) | ProjectPublicType::VocabularyNominal { .. }, _)
                if enclosed(tokens, true) =>
            {
                self.record(tokens, ty, module, closed, depth + 1)
            }
            _ => self.scalar(tokens, ty, module),
        }
    }

    /// Lowers exactly one typed scalar and charges retained decoded text.
    fn scalar(
        &mut self,
        tokens: &[Token],
        ty: &ProjectPublicType,
        module: &str,
    ) -> Result<ProjectValue, ProjectCompileFailure> {
        let digest = self.modules[module].digest;
        let invalid = || {
            fail(
                codes::INVALID_VALUE,
                tokens
                    .first()
                    .map(|token| SourceLocation::new(digest, token.span)),
            )
        };
        match (ty, tokens) {
            (
                ProjectPublicType::Num,
                [
                    Token {
                        kind: TokenKind::Number(spelling),
                        ..
                    },
                ],
            ) => {
                self.charge(spelling.len() as u64)?;
                ExactNumber::from_source(spelling, self.numeric_limit, self.numeric_limit)
                    .map(ProjectValue::Number)
                    .map_err(|_| invalid())
            }
            (
                ProjectPublicType::String | ProjectPublicType::Url | ProjectPublicType::Path,
                [
                    Token {
                        kind: TokenKind::StringLiteral(text),
                        ..
                    },
                ],
            ) => {
                self.charge(text.value.len() as u64)?;
                Ok(match ty {
                    ProjectPublicType::Url => ProjectValue::Url(text.value.clone()),
                    ProjectPublicType::Path => ProjectValue::Path(text.value.clone()),
                    _ => ProjectValue::String(text.value.clone()),
                })
            }
            (
                ProjectPublicType::Bool,
                [
                    Token {
                        kind: TokenKind::True,
                        ..
                    },
                ],
            ) => Ok(ProjectValue::Bool(true)),
            (
                ProjectPublicType::Bool,
                [
                    Token {
                        kind: TokenKind::False,
                        ..
                    },
                ],
            ) => Ok(ProjectValue::Bool(false)),
            _ => Err(invalid()),
        }
    }

    /// Resolves one already parsed name under explicit module/import visibility.
    fn name(
        &self,
        occurrence: &NameOccurrence,
        module: &str,
    ) -> Result<ModuleSymbolIdentity, ProjectCompileFailure> {
        let mut errors = Vec::new();
        let target = resolve_name(
            occurrence,
            &(module.to_owned(), String::new()),
            &self.modules[module],
            self.roots,
            &mut errors,
        )
        .ok_or_else(|| {
            fail(
                codes::INVALID_VALUE,
                Some(SourceLocation::new(
                    self.modules[module].digest,
                    occurrence.span,
                )),
            )
        })?;
        Ok(self.roots[&target].symbol.identity().clone())
    }

    /// Validates every declared default, including defaults unused by bindings.
    fn record_defaults(
        &mut self,
        identity: &ModuleSymbolIdentity,
        depth: usize,
    ) -> Result<(), ProjectCompileFailure> {
        if depth > PROJECT_MAX_DEPTH {
            return Err(fail(codes::LIMIT, None));
        }
        if self.defaults.contains_key(identity) {
            return Ok(());
        }
        let root = &self.roots[&key(identity)];
        let mut defaults = Vec::new();
        for field in split_items(&root.value)? {
            if let Some(eq) = field
                .iter()
                .position(|token| matches!(token.kind, TokenKind::Equals))
            {
                let TokenKind::Identifier(name) = &field[eq - 1].kind else {
                    return Err(fail(codes::INVALID_VALUE, None));
                };
                let ty = root
                    .fields
                    .iter()
                    .find(|(candidate, _)| candidate == name)
                    .map(|(_, ty)| {
                        public_type(
                            ty,
                            identity.module().module_name(),
                            self.modules,
                            self.vocabularies,
                        )
                    })
                    .ok_or_else(|| fail(codes::INVALID_VALUE, None))?;
                defaults.push((
                    name.clone(),
                    self.value(
                        &field[eq + 1..],
                        &ty,
                        identity.module().module_name(),
                        true,
                        depth + 1,
                    )?,
                ));
            }
        }
        defaults.sort_by(|left, right| left.0.cmp(&right.0));
        self.defaults.insert(identity.clone(), defaults);
        Ok(())
    }

    /// Materializes exact contextual field coverage and closed default insertion.
    fn record(
        &mut self,
        tokens: &[Token],
        ty: &ProjectPublicType,
        module: &str,
        closed: bool,
        depth: usize,
    ) -> Result<ProjectValue, ProjectCompileFailure> {
        let (fields, default_owner) = match ty {
            ProjectPublicType::Nominal(identity) => {
                let root = &self.roots[&key(identity)];
                self.charge(root.fields.len() as u64)?;
                let mut fields = root
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            public_type(
                                ty,
                                identity.module().module_name(),
                                self.modules,
                                self.vocabularies,
                            ),
                        )
                    })
                    .collect::<Vec<_>>();
                fields.sort_by(|left, right| left.0.cmp(&right.0));
                self.record_defaults(identity, depth)?;
                (fields, Some(identity))
            }
            ProjectPublicType::VocabularyNominal {
                identity,
                version: _,
                name,
            } => {
                let vocabulary = &self.vocabularies.vocabularies()[identity];
                let record = vocabulary
                    .public_type(name)
                    .ok_or_else(|| fail(codes::INVALID_VALUE, None))?;
                self.charge(record.fields().len() as u64)?;
                (
                    record
                        .fields()
                        .iter()
                        .map(|field| {
                            (
                                field.name().to_owned(),
                                vocabulary_type(
                                    field.ty(),
                                    vocabulary.identity(),
                                    vocabulary.version(),
                                ),
                            )
                        })
                        .collect(),
                    None,
                )
            }
            _ => return Err(fail(codes::INVALID_VALUE, None)),
        };
        let mut supplied = BTreeMap::new();
        for field in split_items(&tokens[1..tokens.len() - 1])? {
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
                return Err(fail(codes::INVALID_VALUE, None));
            };
            if rest.is_empty() || supplied.insert(name.clone(), rest).is_some() {
                return Err(fail(codes::INVALID_VALUE, None));
            }
        }
        let mut values = Vec::new();
        for (name, field_type) in fields {
            let value = if let Some(tokens) = supplied.remove(&name) {
                self.value(tokens, &field_type, module, closed, depth)?
            } else if let Some(owner) = default_owner {
                let default = self.defaults[owner]
                    .iter()
                    .find(|(field, _)| field == &name)
                    .ok_or_else(|| fail(codes::INVALID_VALUE, None))?;
                let cost = value_cost(&default.1);
                self.charge(cost)?;
                self.defaults[owner]
                    .iter()
                    .find(|(field, _)| field == &name)
                    .ok_or_else(|| fail(codes::INVALID_VALUE, None))?
                    .1
                    .clone()
            } else {
                return Err(fail(codes::INVALID_VALUE, None));
            };
            values.push((name, value));
        }
        if !supplied.is_empty() {
            return Err(fail(codes::INVALID_VALUE, None));
        }
        Ok(ProjectValue::Record(values))
    }
}

/// Returns whether token endpoints are the exact requested contextual delimiters.
fn enclosed(tokens: &[Token], record: bool) -> bool {
    if record {
        matches!(
            tokens.first().map(|token| &token.kind),
            Some(TokenKind::OpenBrace)
        ) && matches!(
            tokens.last().map(|token| &token.kind),
            Some(TokenKind::CloseBrace)
        )
    } else {
        matches!(
            tokens.first().map(|token| &token.kind),
            Some(TokenKind::OpenBracket)
        ) && matches!(
            tokens.last().map(|token| &token.kind),
            Some(TokenKind::CloseBracket)
        )
    }
}

/// Splits top-level comma-delimited children without reparsing nested bodies.
fn split_items(tokens: &[Token]) -> Result<Vec<&[Token]>, ProjectCompileFailure> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::OpenBrace
            | TokenKind::OpenBracket
            | TokenKind::OpenParen
            | TokenKind::Less => depth += 1,
            TokenKind::CloseBrace
            | TokenKind::CloseBracket
            | TokenKind::CloseParen
            | TokenKind::Greater => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| fail(codes::INVALID_VALUE, None))?;
            }
            TokenKind::Comma if depth == 0 => {
                if start == index {
                    return Err(fail(codes::INVALID_VALUE, None));
                }
                items.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(fail(codes::INVALID_VALUE, None));
    }
    if start < tokens.len() {
        items.push(&tokens[start..]);
    }
    Ok(items)
}

/// Converts the closed vocabulary field grammar to shared project types.
fn vocabulary_type(ty: &ProjectVocabularyType, identity: &str, version: &str) -> ProjectPublicType {
    match ty {
        ProjectVocabularyType::Num => ProjectPublicType::Num,
        ProjectVocabularyType::String => ProjectPublicType::String,
        ProjectVocabularyType::Bool => ProjectPublicType::Bool,
        ProjectVocabularyType::Url => ProjectPublicType::Url,
        ProjectVocabularyType::Path => ProjectPublicType::Path,
        ProjectVocabularyType::Nominal(name) => ProjectPublicType::VocabularyNominal {
            identity: identity.to_owned(),
            version: version.to_owned(),
            name: name.clone(),
        },
    }
}

/// Retains complete vocabulary interpretation schemas, never source aliases.
fn vocabulary_records(vocabularies: &ProjectVocabularySet) -> Vec<ProjectVocabularyRecord> {
    vocabularies
        .vocabularies()
        .values()
        .flat_map(|vocabulary| {
            vocabulary
                .types()
                .iter()
                .map(|record| ProjectVocabularyRecord {
                    identity: vocabulary.identity().to_owned(),
                    version: vocabulary.version().to_owned(),
                    name: record.name().to_owned(),
                    public: record.is_public(),
                    fields: record
                        .fields()
                        .iter()
                        .map(|field| {
                            (
                                field.name().to_owned(),
                                vocabulary_type(
                                    field.ty(),
                                    vocabulary.identity(),
                                    vocabulary.version(),
                                ),
                            )
                        })
                        .collect(),
                })
        })
        .collect()
}

/// Counts materialized nodes iteratively, including contextual children.
fn nodes(value: &ProjectValue) -> u64 {
    let mut pending = vec![value];
    let mut count = 0;
    while let Some(value) = pending.pop() {
        count += 1;
        match value {
            ProjectValue::List(values) => pending.extend(values),
            ProjectValue::Record(fields) => pending.extend(fields.iter().map(|(_, value)| value)),
            _ => {}
        }
    }
    count
}

/// Computes retained clone work before copying a reused/default subtree.
fn value_cost(value: &ProjectValue) -> u64 {
    let mut pending = vec![value];
    let mut cost = 0_u64;
    while let Some(value) = pending.pop() {
        cost = cost.saturating_add(1);
        match value {
            ProjectValue::Number(number) => {
                cost = cost.saturating_add(number.coefficient().len() as u64);
            }
            ProjectValue::String(text) | ProjectValue::Url(text) | ProjectValue::Path(text) => {
                cost = cost.saturating_add(text.len() as u64);
            }
            ProjectValue::List(values) => pending.extend(values),
            ProjectValue::Record(fields) => pending.extend(fields.iter().map(|(_, value)| value)),
            _ => {}
        }
    }
    cost
}

/// Constructs a non-authoritative lowering failure envelope.
fn fail(code: &'static str, location: Option<SourceLocation>) -> ProjectCompileFailure {
    ProjectCompileFailure::Lowering { code, location }
}
