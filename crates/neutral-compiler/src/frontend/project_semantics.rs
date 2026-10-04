// SPDX-License-Identifier: Apache-2.0

//! Effect-free declaration and cross-module semantic analysis.

use super::{Token, TokenKind, lexer};
use crate::{
    CapturedProject, ModuleGraph, ProjectVocabularySet, language::graph_names,
    validate_project_vocabularies,
};
use neutral_core::{
    ByteSpan, CancellationToken, CoreError, SourceContentDigest, SourceLocation,
    profile::V1_SOURCE_PROFILE,
};
use neutral_ir::project_interface::{
    MAX_PROJECT_INTERFACE_TYPE_DEPTH, ProjectInterface, ProjectLocationValue, ProjectPublicEdge,
    ProjectPublicEdgeKind, ProjectPublicExport, ProjectPublicField, ProjectPublicSignature,
    ProjectPublicType, ProjectPublicVocabulary,
};
use neutral_ir::{LogicalModuleIdentity, ModuleSymbolIdentity};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[path = "project_lowering.rs"]
mod lowering;
pub use lowering::codes as project_lowering_diagnostics;
pub use lowering::{ProjectCompileFailure, compile_project};

#[cfg(test)]
#[path = "../../tests/project_semantics/mod.rs"]
mod tests;

/// Stable project-semantic diagnostic codes.
pub mod diagnostics {
    /// An ordinary value or embedded type dependency forms a cycle.
    pub const SEMANTIC_CYCLE: &str = "NEU-XMOD-001";
    /// A public modifier is misplaced or duplicated.
    pub const INVALID_PUBLIC: &str = "NEU-XMOD-002";
    /// A name is absent or is not visible through the required import alias.
    pub const INACCESSIBLE_NAME: &str = "NEU-XMOD-003";
    /// A public signature transitively contains a private nominal type.
    pub const PRIVATE_PUBLIC_TYPE: &str = "NEU-XMOD-004";
    /// A publicly exposed reference targets a private binding.
    pub const PRIVATE_REFERENCE: &str = "NEU-XMOD-005";
    /// An immutable reused value has a different declared type.
    pub const TYPE_MISMATCH: &str = "NEU-XMOD-006";
    /// A reference does not target a compatible immutable binding.
    pub const INVALID_REFERENCE: &str = "NEU-XMOD-007";
    /// The explicit project declaration or diagnostic bound was exceeded.
    pub const LIMIT_EXCEEDED: &str = "NEU-XMOD-008";
    /// Cooperative cancellation prevented semantic publication.
    pub const CANCELLED: &str = "NEU-XMOD-009";
    /// A supplied graph does not belong to the captured source closure.
    pub const GRAPH_MISMATCH: &str = "NEU-XMOD-010";
    /// A project declaration or value violates the inherited source grammar.
    pub const INVALID_SOURCE: &str = "NEU-XMOD-011";
    /// An exact captured vocabulary bundle failed closed-schema validation.
    pub const INVALID_VOCABULARY: &str = "NEU-XMOD-012";
    /// A vocabulary type is absent or not source-authorable/public.
    pub const PRIVATE_VOCABULARY_TYPE: &str = "NEU-XMOD-013";
}

/// Root declaration category in the analyzed project.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSymbolKind {
    /// A nominal record declaration.
    Record,
    /// An immutable typed binding.
    Binding,
}

/// One stable, source-accounted root declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSymbol {
    /// Alias-independent symbol key.
    identity: ModuleSymbolIdentity,
    /// Root declaration category.
    kind: ProjectSymbolKind,
    /// Explicit public modifier state.
    is_public: bool,
    /// Complete declaration location in captured bytes.
    source: SourceLocation,
}

impl ProjectSymbol {
    /// Returns the alias-independent module-symbol identity.
    #[must_use]
    pub const fn identity(&self) -> &ModuleSymbolIdentity {
        &self.identity
    }

    /// Returns whether the root is explicitly public.
    #[must_use]
    pub const fn is_public(&self) -> bool {
        self.is_public
    }

    /// Returns the root declaration category.
    #[must_use]
    pub const fn kind(&self) -> ProjectSymbolKind {
        self.kind
    }

    /// Returns its original captured source location.
    #[must_use]
    pub const fn source_location(&self) -> SourceLocation {
        self.source
    }
}

/// Semantic dependency kind retained independently of source spelling.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectDependencyKind {
    /// Nominal type appearing in a declaration signature or field.
    Type,
    /// Nominal target nested under `Ref<T>`; it does not embed or evaluate.
    ReferenceType,
    /// Ordinary immutable value reuse.
    Value,
    /// Typed identity-only reference to a binding.
    Reference,
}

/// One source-accounted, alias-independent project dependency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDependency {
    /// Stable consumer identity.
    from: ModuleSymbolIdentity,
    /// Stable target identity.
    to: ModuleSymbolIdentity,
    /// Typed semantic edge category.
    kind: ProjectDependencyKind,
    /// Exact source occurrence separately from identity.
    source: SourceLocation,
}

impl ProjectDependency {
    /// Returns the consumer root's stable identity.
    #[must_use]
    pub const fn from(&self) -> &ModuleSymbolIdentity {
        &self.from
    }

    /// Returns the target root's stable identity.
    #[must_use]
    pub const fn to(&self) -> &ModuleSymbolIdentity {
        &self.to
    }

    /// Returns the semantic edge category.
    #[must_use]
    pub const fn kind(&self) -> ProjectDependencyKind {
        self.kind
    }

    /// Returns the original source occurrence separately from identity.
    #[must_use]
    pub const fn source_location(&self) -> SourceLocation {
        self.source
    }
}

/// Project resolution facts, not a validated project or authoritative IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSemanticModel {
    /// Every root in canonical module-symbol order.
    symbols: Arc<[ProjectSymbol]>,
    /// Every source-accounted semantic edge in canonical order.
    dependencies: Arc<[ProjectDependency]>,
    /// Immutable binding identities in dependency-first order.
    value_order: Arc<[ModuleSymbolIdentity]>,
    /// Public-only interface with private source/provenance omitted.
    public_interface: Arc<ProjectInterface>,
    /// Source-authored inert location scalars in canonical binding order.
    locations: Arc<[(ModuleSymbolIdentity, ProjectLocationValue)]>,
}

impl ProjectSemanticModel {
    /// Returns roots in stable module-symbol order.
    #[must_use]
    pub fn symbols(&self) -> &[ProjectSymbol] {
        &self.symbols
    }

    /// Returns source-accounted semantic edges in stable order.
    #[must_use]
    pub fn dependencies(&self) -> &[ProjectDependency] {
        &self.dependencies
    }

    /// Returns the stable dependency-first order for immutable bindings.
    #[must_use]
    pub fn value_order(&self) -> &[ModuleSymbolIdentity] {
        &self.value_order
    }

    /// Returns the public-only interface for independent reader validation.
    #[must_use]
    pub const fn public_interface(&self) -> &Arc<ProjectInterface> {
        &self.public_interface
    }

    /// Returns exact inert location values, distinct from ordinary strings.
    #[must_use]
    pub fn locations(&self) -> &[(ModuleSymbolIdentity, ProjectLocationValue)] {
        &self.locations
    }
}

/// One bounded, deterministic project-semantic failure location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSemanticDiagnostic {
    /// Stable project-semantic failure code.
    code: &'static str,
    /// Exact owning logical module.
    module_id: Arc<str>,
    /// Exact captured source location.
    source: SourceLocation,
}

impl ProjectSemanticDiagnostic {
    /// Returns the stable failure code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Returns the exact logical module ID, without exposing a private name.
    #[must_use]
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Returns the original-byte failure location.
    #[must_use]
    pub const fn source_location(&self) -> SourceLocation {
        self.source
    }
}

/// A failed semantic analysis with no authoritative partial model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSemanticFailure {
    /// Canonically ordered, bounded failure diagnostics.
    diagnostics: Arc<[ProjectSemanticDiagnostic]>,
}

impl ProjectSemanticFailure {
    /// Returns canonically ordered, bounded diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[ProjectSemanticDiagnostic] {
        &self.diagnostics
    }
}

/// One parsed nominal type expression and its source occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TypeExpr {
    /// Exact core numeric type.
    Num,
    /// Exact core string type.
    String,
    /// Exact core Boolean type.
    Bool,
    /// Inert URL text type.
    Url,
    /// Inert path text type.
    Path,
    /// Local or alias-qualified nominal type and its source occurrence.
    Nominal(Option<String>, String, ByteSpan),
    /// Invariant ordered-list type.
    List(Box<Self>),
    /// Invariant typed reference target.
    Ref(Box<Self>),
    /// One outer nullability layer.
    Nullable(Box<Self>),
}

/// Private declaration body used to resolve all roots as a set.
#[derive(Clone)]
struct Root {
    /// Complete public symbol identity and location.
    symbol: ProjectSymbol,
    /// Explicit binding type, absent for records.
    declared_type: Option<TypeExpr>,
    /// Record field types in source order.
    fields: Vec<(String, TypeExpr)>,
    /// Exact binding initializer or record field/default tokens.
    value: Vec<Token>,
    /// Exact typed inert scalar for a direct location binding.
    location: Option<ProjectLocationValue>,
}

/// An exact source occurrence of one resolved name.
struct NameOccurrence {
    /// Explicit import alias, absent for a local name.
    alias: Option<String>,
    /// Exact root name spelling.
    name: String,
    /// Name occurrence in original bytes.
    span: ByteSpan,
}

/// An analyzed source module and its import-alias table.
struct ModuleContext {
    /// Exact source content identity.
    digest: SourceContentDigest,
    /// Explicit local aliases to full imported module IDs.
    aliases: BTreeMap<String, String>,
}

/// Resolves the declaration graph of one captured project.
///
/// # Errors
///
/// Returns no model when declaration parsing, visibility, public type closure,
/// cross-module reuse/reference resolution, dependency cycles, limits, or
/// cancellation fail. A successful resolution model does not by itself make
/// the v1 source profile available or replace later contextual value
/// validation and project-IR lowering.
#[expect(
    clippy::too_many_lines,
    reason = "explicit fail-closed project phases remain visible in one orchestration boundary"
)]
pub fn analyze_project_semantics(
    captured: &CapturedProject,
    graph: &ModuleGraph,
    cancellation: &CancellationToken,
) -> Result<ProjectSemanticModel, ProjectSemanticFailure> {
    if graph.modules().len() != captured.sources().len()
        || graph
            .modules()
            .iter()
            .zip(captured.sources())
            .any(|(module, source)| {
                module.module_id() != source.module_id()
                    || module.source_location().source() != source.digest()
            })
    {
        let source = captured.sources().first();
        return Err(single(diagnostic(
            diagnostics::GRAPH_MISMATCH,
            source.map_or("", |source| source.module_id()),
            source.map_or_else(
                || SourceContentDigest::from_bytes(&[]),
                crate::CapturedProjectSource::digest,
            ),
            zero_span(),
        )));
    }
    if cancellation.is_cancelled() {
        let source = captured.sources().first();
        return Err(single(diagnostic(
            diagnostics::CANCELLED,
            source.map_or("", |source| source.module_id()),
            source.map_or_else(
                || SourceContentDigest::from_bytes(&[]),
                crate::CapturedProjectSource::digest,
            ),
            zero_span(),
        )));
    }
    let vocabularies = validate_project_vocabularies(captured).map_err(|_| {
        let source = captured.sources().first();
        single(diagnostic(
            diagnostics::INVALID_VOCABULARY,
            source.map_or("", |source| source.module_id()),
            source.map_or_else(
                || SourceContentDigest::from_bytes(&[]),
                crate::CapturedProjectSource::digest,
            ),
            zero_span(),
        ))
    })?;
    let mut roots = BTreeMap::new();
    let mut modules = BTreeMap::new();
    let limits = captured.limits().values();
    for source in captured.sources() {
        if cancellation.is_cancelled() {
            return Err(single(diagnostic(
                diagnostics::CANCELLED,
                source.module_id(),
                source.digest(),
                zero_span(),
            )));
        }
        let aliases = graph
            .edges()
            .iter()
            .filter(|edge| edge.from() == source.module_id())
            .map(|edge| (edge.alias().to_owned(), edge.target().to_owned()))
            .collect();
        modules.insert(
            source.module_id().to_owned(),
            ModuleContext {
                digest: source.digest(),
                aliases,
            },
        );
        let declarations = parse_roots(source.module_id(), source.digest(), source.bytes())?;
        for root in declarations {
            let key = key(root.symbol.identity());
            if modules[source.module_id()].aliases.contains_key(&key.1) {
                return Err(single(diagnostic(
                    diagnostics::INACCESSIBLE_NAME,
                    source.module_id(),
                    source.digest(),
                    root.symbol.source.span(),
                )));
            }
            if roots.len() as u64 >= limits.declarations {
                return Err(single(diagnostic(
                    diagnostics::LIMIT_EXCEEDED,
                    source.module_id(),
                    source.digest(),
                    root.symbol.source.span(),
                )));
            }
            if roots.insert(key, root.clone()).is_some() {
                return Err(single(diagnostic(
                    diagnostics::INACCESSIBLE_NAME,
                    source.module_id(),
                    source.digest(),
                    root.symbol.source.span(),
                )));
            }
        }
    }
    let mut edges = Vec::new();
    let mut errors = Vec::new();
    for (owner, root) in &roots {
        if cancellation.is_cancelled() {
            return Err(single(diagnostic(
                diagnostics::CANCELLED,
                &owner.0,
                root.symbol.source.source(),
                root.symbol.source.span(),
            )));
        }
        let module = &modules[&owner.0];
        if let Some(ty) = &root.declared_type {
            resolve_type(
                ty,
                owner,
                root,
                module,
                &roots,
                &vocabularies,
                &mut edges,
                &mut errors,
                false,
            );
        }
        for (_, ty) in &root.fields {
            resolve_type(
                ty,
                owner,
                root,
                module,
                &roots,
                &vocabularies,
                &mut edges,
                &mut errors,
                false,
            );
        }
        if root.symbol.kind == ProjectSymbolKind::Binding {
            resolve_value(
                root,
                owner,
                module,
                &modules,
                &roots,
                &vocabularies,
                &mut edges,
                &mut errors,
            );
        }
    }
    if !errors.is_empty() {
        return Err(failure(errors, limits.diagnostics));
    }
    if let Some(exposed) = exposed_private_reference(&roots, &edges) {
        let root = &roots[&exposed];
        return Err(single(diagnostic(
            diagnostics::PRIVATE_REFERENCE,
            &exposed.0,
            root.symbol.source.source(),
            root.symbol.source.span(),
        )));
    }
    if let Some(cycle) = cycle_root(&roots, &edges) {
        let root = &roots[&cycle];
        return Err(single(diagnostic(
            diagnostics::SEMANTIC_CYCLE,
            &cycle.0,
            root.symbol.source.source(),
            root.symbol.source.span(),
        )));
    }
    let value_order = dependency_first_values(&roots, &edges);
    let public_interface = build_public_interface(&roots, &modules, &vocabularies, &edges)
        .map_err(|_| {
            let source = captured.sources().first();
            single(diagnostic(
                diagnostics::LIMIT_EXCEEDED,
                source.map_or("", |source| source.module_id()),
                source.map_or_else(
                    || SourceContentDigest::from_bytes(&[]),
                    crate::CapturedProjectSource::digest,
                ),
                zero_span(),
            ))
        })?;
    edges.sort_by(|a, b| {
        (a.from(), a.kind(), a.to(), a.source.span().start()).cmp(&(
            b.from(),
            b.kind(),
            b.to(),
            b.source.span().start(),
        ))
    });
    let locations = roots
        .values()
        .filter_map(|root| {
            root.location
                .as_ref()
                .map(|value| (root.symbol.identity.clone(), value.clone()))
        })
        .collect::<Vec<_>>();
    let symbols = roots
        .into_values()
        .map(|root| root.symbol)
        .collect::<Vec<_>>();
    Ok(ProjectSemanticModel {
        symbols: Arc::from(symbols),
        dependencies: Arc::from(edges),
        value_order: Arc::from(value_order),
        public_interface: Arc::new(public_interface),
        locations: Arc::from(locations),
    })
}

/// Orders all immutable roots after their ordinary value dependencies.
fn dependency_first_values(
    roots: &BTreeMap<(String, String), Root>,
    edges: &[ProjectDependency],
) -> Vec<ModuleSymbolIdentity> {
    let mut pending = roots
        .iter()
        .filter(|(_, root)| root.symbol.kind == ProjectSymbolKind::Binding)
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    let mut dependencies = pending
        .iter()
        .cloned()
        .map(|key| (key, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for edge in edges {
        if edge.kind == ProjectDependencyKind::Value {
            dependencies
                .get_mut(&key(edge.from()))
                .expect("value edge owner is a binding")
                .insert(key(edge.to()));
        }
    }
    let mut order = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .find(|node| dependencies[*node].is_disjoint(&pending))
            .cloned()
            .expect("all ordinary value cycles were rejected");
        pending.remove(&ready);
        order.push(roots[&ready].symbol.identity.clone());
    }
    order
}

/// Projects only public signatures and public-to-public identity edges.
fn build_public_interface(
    roots: &BTreeMap<(String, String), Root>,
    modules: &BTreeMap<String, ModuleContext>,
    vocabularies: &ProjectVocabularySet,
    edges: &[ProjectDependency],
) -> Result<ProjectInterface, CoreError> {
    let mut exports = Vec::new();
    let public_keys = roots
        .iter()
        .filter(|(_, root)| root.symbol.is_public)
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    for (owner, root) in roots {
        if !root.symbol.is_public {
            continue;
        }
        let signature = match root.symbol.kind {
            ProjectSymbolKind::Binding => ProjectPublicSignature::Binding(public_type(
                root.declared_type
                    .as_ref()
                    .expect("bindings have a declared type"),
                &owner.0,
                modules,
                vocabularies,
            )),
            ProjectSymbolKind::Record => {
                let mut fields = root
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        ProjectPublicField::new(
                            name,
                            public_type(ty, &owner.0, modules, vocabularies),
                        )
                    })
                    .collect::<Vec<_>>();
                fields.sort_by(|left, right| left.name().cmp(right.name()));
                ProjectPublicSignature::Record(fields)
            }
        };
        exports.push(ProjectPublicExport::new(
            root.symbol.identity.clone(),
            signature,
        ));
    }
    let mut public_edges = edges
        .iter()
        .filter(|edge| {
            public_keys.contains(&key(edge.from())) && public_keys.contains(&key(edge.to()))
        })
        .map(|edge| {
            ProjectPublicEdge::new(
                edge.from().clone(),
                edge.to().clone(),
                match edge.kind() {
                    ProjectDependencyKind::Type => ProjectPublicEdgeKind::Type,
                    ProjectDependencyKind::ReferenceType => ProjectPublicEdgeKind::ReferenceType,
                    ProjectDependencyKind::Value => ProjectPublicEdgeKind::Value,
                    ProjectDependencyKind::Reference => ProjectPublicEdgeKind::Reference,
                },
            )
        })
        .collect::<Vec<_>>();
    public_edges.sort_by(|left, right| {
        (left.from(), left.kind(), left.to()).cmp(&(right.from(), right.kind(), right.to()))
    });
    public_edges.dedup();
    let public_vocabularies = vocabularies
        .vocabularies()
        .values()
        .map(|vocabulary| {
            ProjectPublicVocabulary::new(
                vocabulary.identity(),
                vocabulary.version(),
                vocabulary
                    .types()
                    .iter()
                    .filter(|ty| ty.is_public())
                    .map(|ty| ty.name().to_owned())
                    .collect(),
            )
        })
        .collect();
    ProjectInterface::with_vocabularies(public_vocabularies, exports, public_edges)
}

/// Converts a resolved source type to an alias-independent public type.
fn public_type(
    ty: &TypeExpr,
    module_id: &str,
    modules: &BTreeMap<String, ModuleContext>,
    vocabularies: &ProjectVocabularySet,
) -> ProjectPublicType {
    match ty {
        TypeExpr::Num => ProjectPublicType::Num,
        TypeExpr::String => ProjectPublicType::String,
        TypeExpr::Bool => ProjectPublicType::Bool,
        TypeExpr::Url => ProjectPublicType::Url,
        TypeExpr::Path => ProjectPublicType::Path,
        TypeExpr::Nominal(alias, name, _) => {
            if let Some(vocabulary) = alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(module_id, alias))
            {
                return ProjectPublicType::VocabularyNominal {
                    identity: vocabulary.identity().to_owned(),
                    version: vocabulary.version().to_owned(),
                    name: name.clone(),
                };
            }
            let owner = alias.as_ref().map_or(module_id, |alias| {
                modules[module_id]
                    .aliases
                    .get(alias)
                    .expect("resolved import alias")
                    .as_str()
            });
            ProjectPublicType::Nominal(ModuleSymbolIdentity::new(
                LogicalModuleIdentity::new(V1_SOURCE_PROFILE, owner),
                name,
            ))
        }
        TypeExpr::List(inner) => ProjectPublicType::List(Box::new(public_type(
            inner,
            module_id,
            modules,
            vocabularies,
        ))),
        TypeExpr::Ref(inner) => ProjectPublicType::Ref(Box::new(public_type(
            inner,
            module_id,
            modules,
            vocabularies,
        ))),
        TypeExpr::Nullable(inner) => ProjectPublicType::Nullable(Box::new(public_type(
            inner,
            module_id,
            modules,
            vocabularies,
        ))),
    }
}

/// Parses root declarations with the existing byte-accurate lexer.
fn parse_roots(
    module: &str,
    digest: SourceContentDigest,
    bytes: &[u8],
) -> Result<Vec<Root>, ProjectSemanticFailure> {
    let lexed = lexer::lex(bytes).map_err(|error| {
        single(diagnostic(
            diagnostics::INACCESSIBLE_NAME,
            module,
            digest,
            error.span,
        ))
    })?;
    let mut roots = Vec::new();
    let mut statement = Vec::new();
    let mut depth = 0_u64;
    let mut header_lines = 0_u8;
    for token in lexed.tokens {
        if matches!(
            token.kind,
            TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
        ) && depth == 0
        {
            if !statement.is_empty() {
                if header_lines < 2 {
                    header_lines += 1;
                } else if !is_requirement_or_import(&statement) {
                    roots.push(parse_root(module, digest, &statement)?);
                }
                statement.clear();
            }
            continue;
        }
        if matches!(
            token.kind,
            TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
        ) {
            continue;
        }
        match token.kind {
            TokenKind::OpenBrace | TokenKind::OpenBracket | TokenKind::OpenParen => depth += 1,
            TokenKind::CloseBrace | TokenKind::CloseBracket | TokenKind::CloseParen => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
        statement.push(token);
    }
    if !statement.is_empty() {
        return Err(single(diagnostic(
            diagnostics::INACCESSIBLE_NAME,
            module,
            digest,
            statement[0].span,
        )));
    }
    Ok(roots)
}

/// Identifies already graph-validated vocabulary and import statements.
fn is_requirement_or_import(tokens: &[Token]) -> bool {
    matches!(tokens[0].kind, TokenKind::Use)
        || matches!(&tokens[0].kind, TokenKind::Identifier(word) if word == graph_names::IMPORT)
}

/// Parses one complete declaration, preserving exact source locations.
#[expect(
    clippy::too_many_lines,
    reason = "one declaration parser retains the exact source span and failure boundary"
)]
fn parse_root(
    module: &str,
    digest: SourceContentDigest,
    tokens: &[Token],
) -> Result<Root, ProjectSemanticFailure> {
    let mut start = 0;
    let public =
        matches!(&tokens[0].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC);
    if public {
        start = 1;
    }
    let invalid = || {
        single(diagnostic(
            diagnostics::INVALID_SOURCE,
            module,
            digest,
            tokens[0].span,
        ))
    };
    if start >= tokens.len() {
        return Err(invalid());
    }
    if matches!(&tokens[start].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC) {
        return Err(single(diagnostic(
            diagnostics::INVALID_PUBLIC,
            module,
            digest,
            tokens[start].span,
        )));
    }
    let span = ByteSpan::new(tokens[0].span.start(), tokens[tokens.len() - 1].span.end())
        .expect("ordered declaration span");
    if matches!(tokens[start].kind, TokenKind::Record) {
        let Some(Token {
            kind: TokenKind::Identifier(name),
            ..
        }) = tokens.get(start + 1)
        else {
            return Err(invalid());
        };
        if !matches!(
            tokens.get(start + 2).map(|token| &token.kind),
            Some(TokenKind::OpenBrace)
        ) || !matches!(
            tokens.last().map(|token| &token.kind),
            Some(TokenKind::CloseBrace)
        ) {
            return Err(invalid());
        }
        let fields = parse_fields(&tokens[start + 3..tokens.len() - 1]).ok_or_else(invalid)?;
        return Ok(Root {
            symbol: symbol(
                module,
                digest,
                span,
                name,
                ProjectSymbolKind::Record,
                public,
            ),
            declared_type: None,
            fields,
            value: tokens[start + 3..tokens.len() - 1].to_vec(),
            location: None,
        });
    }
    let Some(eq) = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Equals))
    else {
        return Err(invalid());
    };
    if eq <= start + 1 || eq + 1 >= tokens.len() {
        return Err(invalid());
    }
    let TokenKind::Identifier(name) = &tokens[eq - 1].kind else {
        return Err(invalid());
    };
    let ty = parse_complete_type(&tokens[start..eq - 1]).ok_or_else(invalid)?;
    if !validate_value(&tokens[eq + 1..]) {
        return Err(invalid());
    }
    let location = match (&ty, &tokens[eq + 1..]) {
        (
            TypeExpr::Url,
            [
                Token {
                    kind: TokenKind::StringLiteral(value),
                    ..
                },
            ],
        ) => Some(ProjectLocationValue::Url(value.value.clone())),
        (
            TypeExpr::Path,
            [
                Token {
                    kind: TokenKind::StringLiteral(value),
                    ..
                },
            ],
        ) => Some(ProjectLocationValue::Path(value.value.clone())),
        (TypeExpr::Url | TypeExpr::Path, _) => return Err(invalid()),
        _ => None,
    };
    Ok(Root {
        symbol: symbol(
            module,
            digest,
            span,
            name,
            ProjectSymbolKind::Binding,
            public,
        ),
        declared_type: Some(ty),
        fields: Vec::new(),
        value: tokens[eq + 1..].to_vec(),
        location,
    })
}

/// Builds one stable module-symbol identity from a captured declaration.
fn symbol(
    module: &str,
    digest: SourceContentDigest,
    span: ByteSpan,
    name: &str,
    kind: ProjectSymbolKind,
    is_public: bool,
) -> ProjectSymbol {
    ProjectSymbol {
        identity: ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(V1_SOURCE_PROFILE, module),
            name,
        ),
        kind,
        is_public,
        source: SourceLocation::new(digest, span),
    }
}

/// Parses comma-terminated record fields and their type expressions.
fn parse_fields(tokens: &[Token]) -> Option<Vec<(String, TypeExpr)>> {
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();
    let mut start = 0;
    let mut depth = 0_u64;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Less
            | TokenKind::OpenBrace
            | TokenKind::OpenBracket
            | TokenKind::OpenParen => depth += 1,
            TokenKind::Greater
            | TokenKind::CloseBrace
            | TokenKind::CloseBracket
            | TokenKind::CloseParen => depth = depth.checked_sub(1)?,
            TokenKind::Comma if depth == 0 => {
                let field = &tokens[start..index];
                if field.len() < 2 {
                    return None;
                }
                if matches!(&field[0].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC)
                {
                    return None;
                }
                let before_default = field
                    .iter()
                    .position(|token| matches!(token.kind, TokenKind::Equals))
                    .unwrap_or(field.len());
                let name_index = field[..before_default]
                    .iter()
                    .rposition(|token| matches!(token.kind, TokenKind::Identifier(_)))?;
                if name_index == 0 {
                    return None;
                }
                if name_index + 1 != before_default {
                    return None;
                }
                let TokenKind::Identifier(name) = &field[name_index].kind else {
                    return None;
                };
                if !names.insert(name.clone()) {
                    return None;
                }
                fields.push((name.clone(), parse_complete_type(&field[..name_index])?));
                if before_default < field.len() && !validate_value(&field[before_default + 1..]) {
                    return None;
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    if start != tokens.len() {
        return None;
    }
    Some(fields)
}

/// Parses exactly one type expression from token syntax.
fn parse_complete_type(tokens: &[Token]) -> Option<TypeExpr> {
    let mut index = 0;
    let ty = parse_type(tokens, &mut index, 0)?;
    (index == tokens.len()).then_some(ty)
}

/// Parses one recursive core, nominal, list, ref, or nullable type.
fn parse_type(tokens: &[Token], index: &mut usize, depth: usize) -> Option<TypeExpr> {
    if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        return None;
    }
    let first = tokens.get(*index)?;
    *index += 1;
    let mut ty = match &first.kind {
        TokenKind::Num => TypeExpr::Num,
        TokenKind::StringType => TypeExpr::String,
        TokenKind::BoolType => TypeExpr::Bool,
        TokenKind::Identifier(name) if name == graph_names::URL => TypeExpr::Url,
        TokenKind::Identifier(name) if name == graph_names::PATH => TypeExpr::Path,
        TokenKind::Identifier(name) | TokenKind::ProtectedName(name) => {
            if matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::DoubleColon)
            ) {
                *index += 1;
                let second = tokens.get(*index)?;
                let TokenKind::Identifier(target) = &second.kind else {
                    return None;
                };
                *index += 1;
                TypeExpr::Nominal(Some(name.clone()), target.clone(), second.span)
            } else {
                TypeExpr::Nominal(None, name.clone(), first.span)
            }
        }
        TokenKind::List | TokenKind::RefType => {
            if !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::Less)
            ) {
                return None;
            }
            *index += 1;
            let inner = Box::new(parse_type(tokens, index, depth + 1)?);
            if !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::Greater)
            ) {
                return None;
            }
            *index += 1;
            if matches!(first.kind, TokenKind::List) {
                TypeExpr::List(inner)
            } else {
                TypeExpr::Ref(inner)
            }
        }
        _ => return None,
    };
    if matches!(
        tokens.get(*index).map(|token| &token.kind),
        Some(TokenKind::Question)
    ) {
        *index += 1;
        ty = TypeExpr::Nullable(Box::new(ty));
    }
    Some(ty)
}

/// Checks complete inherited contextual-value syntax, including qualified names.
fn validate_value(tokens: &[Token]) -> bool {
    let mut index = 0;
    parse_value_shape(tokens, &mut index, 0).is_some() && index == tokens.len()
}

/// Consumes one bounded scalar, reuse, reference, record, or list value.
fn parse_value_shape(tokens: &[Token], index: &mut usize, depth: usize) -> Option<()> {
    if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        return None;
    }
    let token = tokens.get(*index)?;
    match token.kind {
        TokenKind::Number(_)
        | TokenKind::StringLiteral(_)
        | TokenKind::True
        | TokenKind::False
        | TokenKind::Null => {
            *index += 1;
        }
        TokenKind::Identifier(_) => {
            let (_, end) = parse_name(tokens, *index)?;
            *index = end;
        }
        TokenKind::RefValue => {
            let (_, end) = parse_reference(tokens, *index)?;
            *index = end;
        }
        TokenKind::OpenBracket => {
            *index += 1;
            while !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::CloseBracket)
            ) {
                parse_value_shape(tokens, index, depth + 1)?;
                if matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Comma)
                ) {
                    *index += 1;
                } else if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::CloseBracket)
                ) {
                    return None;
                }
            }
            *index += 1;
        }
        TokenKind::OpenBrace => {
            *index += 1;
            while !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::CloseBrace)
            ) {
                if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Identifier(_))
                ) {
                    return None;
                }
                *index += 1;
                if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Colon)
                ) {
                    return None;
                }
                *index += 1;
                parse_value_shape(tokens, index, depth + 1)?;
                if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Comma)
                ) {
                    return None;
                }
                *index += 1;
            }
            *index += 1;
        }
        _ => return None,
    }
    Some(())
}

/// Resolves nominal type occurrences and public signature closure.
#[expect(
    clippy::too_many_arguments,
    reason = "resolution requires explicit source, declaration, and result sinks"
)]
fn resolve_type(
    ty: &TypeExpr,
    owner: &(String, String),
    root: &Root,
    module: &ModuleContext,
    roots: &BTreeMap<(String, String), Root>,
    vocabularies: &ProjectVocabularySet,
    edges: &mut Vec<ProjectDependency>,
    errors: &mut Vec<ProjectSemanticDiagnostic>,
    under_ref: bool,
) {
    match ty {
        TypeExpr::Nominal(alias, name, span) => {
            if let Some(vocabulary) = alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(&owner.0, alias))
            {
                if vocabulary.public_type(name).is_none() {
                    errors.push(diagnostic(
                        diagnostics::PRIVATE_VOCABULARY_TYPE,
                        &owner.0,
                        module.digest,
                        *span,
                    ));
                }
                return;
            }
            let occurrence = NameOccurrence {
                alias: alias.clone(),
                name: name.clone(),
                span: *span,
            };
            if let Some(target) = resolve_name(&occurrence, owner, module, roots, errors) {
                let target_root = &roots[&target];
                if target_root.symbol.kind == ProjectSymbolKind::Record {
                    if root.symbol.is_public && !target_root.symbol.is_public {
                        errors.push(diagnostic(
                            diagnostics::PRIVATE_PUBLIC_TYPE,
                            &owner.0,
                            module.digest,
                            *span,
                        ));
                    }
                    let kind = if under_ref {
                        ProjectDependencyKind::ReferenceType
                    } else {
                        ProjectDependencyKind::Type
                    };
                    edges.push(dependency(root, target_root, kind, module.digest, *span));
                } else {
                    errors.push(diagnostic(
                        diagnostics::INACCESSIBLE_NAME,
                        &owner.0,
                        module.digest,
                        *span,
                    ));
                }
            }
        }
        TypeExpr::List(inner) | TypeExpr::Nullable(inner) => {
            resolve_type(
                inner,
                owner,
                root,
                module,
                roots,
                vocabularies,
                edges,
                errors,
                under_ref,
            );
        }
        TypeExpr::Ref(inner) => {
            resolve_type(
                inner,
                owner,
                root,
                module,
                roots,
                vocabularies,
                edges,
                errors,
                true,
            );
        }
        _ => {}
    }
}

/// Resolves every value and reference occurrence within one immutable binding.
#[expect(
    clippy::too_many_lines,
    reason = "value and identity references share one ordered token walk"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "explicit context prevents ambient resolution"
)]
fn resolve_value(
    root: &Root,
    owner: &(String, String),
    module: &ModuleContext,
    modules: &BTreeMap<String, ModuleContext>,
    roots: &BTreeMap<(String, String), Root>,
    vocabularies: &ProjectVocabularySet,
    edges: &mut Vec<ProjectDependency>,
    errors: &mut Vec<ProjectSemanticDiagnostic>,
) {
    let tokens = &root.value;
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens[index].kind, TokenKind::RefValue) {
            let Some((occurrence, end)) = parse_reference(tokens, index) else {
                errors.push(diagnostic(
                    diagnostics::INVALID_REFERENCE,
                    &owner.0,
                    module.digest,
                    tokens[index].span,
                ));
                index += 1;
                continue;
            };
            if let Some(target) = resolve_name(&occurrence, owner, module, roots, errors) {
                let target_root = &roots[&target];
                if target_root.symbol.kind == ProjectSymbolKind::Binding {
                    if root.symbol.is_public && !target_root.symbol.is_public {
                        errors.push(diagnostic(
                            diagnostics::PRIVATE_REFERENCE,
                            &owner.0,
                            module.digest,
                            occurrence.span,
                        ));
                    }
                    if tokens.len() == end && index == 0 {
                        if let Some(TypeExpr::Ref(inner)) = &root.declared_type {
                            if !target_root
                                .declared_type
                                .as_ref()
                                .is_some_and(|target_type| {
                                    same_type(
                                        inner,
                                        &owner.0,
                                        target_type,
                                        &target.0,
                                        modules,
                                        vocabularies,
                                    )
                                })
                            {
                                errors.push(diagnostic(
                                    diagnostics::TYPE_MISMATCH,
                                    &owner.0,
                                    module.digest,
                                    occurrence.span,
                                ));
                            }
                        } else {
                            errors.push(diagnostic(
                                diagnostics::TYPE_MISMATCH,
                                &owner.0,
                                module.digest,
                                occurrence.span,
                            ));
                        }
                    }
                    edges.push(dependency(
                        root,
                        target_root,
                        ProjectDependencyKind::Reference,
                        module.digest,
                        occurrence.span,
                    ));
                } else {
                    errors.push(diagnostic(
                        diagnostics::INVALID_REFERENCE,
                        &owner.0,
                        module.digest,
                        occurrence.span,
                    ));
                }
            }
            index = end;
            continue;
        }
        if let Some((occurrence, end)) = parse_name(tokens, index) {
            if !matches!(
                tokens.get(end).map(|token| &token.kind),
                Some(TokenKind::Colon)
            ) && let Some(target) = resolve_name(&occurrence, owner, module, roots, errors)
            {
                let target_root = &roots[&target];
                if target_root.symbol.kind == ProjectSymbolKind::Binding {
                    if index == 0
                        && end == tokens.len()
                        && !root
                            .declared_type
                            .as_ref()
                            .zip(target_root.declared_type.as_ref())
                            .is_some_and(|(left, right)| {
                                same_type(left, &owner.0, right, &target.0, modules, vocabularies)
                            })
                    {
                        errors.push(diagnostic(
                            diagnostics::TYPE_MISMATCH,
                            &owner.0,
                            module.digest,
                            occurrence.span,
                        ));
                    }
                    edges.push(dependency(
                        root,
                        target_root,
                        ProjectDependencyKind::Value,
                        module.digest,
                        occurrence.span,
                    ));
                } else {
                    errors.push(diagnostic(
                        diagnostics::INACCESSIBLE_NAME,
                        &owner.0,
                        module.digest,
                        occurrence.span,
                    ));
                }
            }
            index = end;
        } else {
            index += 1;
        }
    }
}

/// Compares types by resolved nominal module identity, never alias spelling.
fn same_type(
    left: &TypeExpr,
    left_module: &str,
    right: &TypeExpr,
    right_module: &str,
    modules: &BTreeMap<String, ModuleContext>,
    vocabularies: &ProjectVocabularySet,
) -> bool {
    match (left, right) {
        (TypeExpr::Num, TypeExpr::Num)
        | (TypeExpr::String, TypeExpr::String)
        | (TypeExpr::Bool, TypeExpr::Bool)
        | (TypeExpr::Url, TypeExpr::Url)
        | (TypeExpr::Path, TypeExpr::Path) => true,
        (
            TypeExpr::Nominal(left_alias, left_name, _),
            TypeExpr::Nominal(right_alias, right_name, _),
        ) => {
            let left_vocabulary = left_alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(left_module, alias));
            let right_vocabulary = right_alias
                .as_ref()
                .and_then(|alias| vocabularies.resolve(right_module, alias));
            if left_vocabulary.is_some() || right_vocabulary.is_some() {
                return left_vocabulary
                    .zip(right_vocabulary)
                    .is_some_and(|(left, right)| {
                        left.identity() == right.identity()
                            && left.version() == right.version()
                            && left_name == right_name
                    });
            }
            let left_owner = left_alias
                .as_ref()
                .and_then(|alias| modules[left_module].aliases.get(alias))
                .map_or(left_module, String::as_str);
            let right_owner = right_alias
                .as_ref()
                .and_then(|alias| modules[right_module].aliases.get(alias))
                .map_or(right_module, String::as_str);
            left_owner == right_owner && left_name == right_name
        }
        (TypeExpr::List(left), TypeExpr::List(right))
        | (TypeExpr::Ref(left), TypeExpr::Ref(right))
        | (TypeExpr::Nullable(left), TypeExpr::Nullable(right)) => same_type(
            left,
            left_module,
            right,
            right_module,
            modules,
            vocabularies,
        ),
        _ => false,
    }
}

/// Finds a private reference reachable by ordinary reuse from a public root.
fn exposed_private_reference(
    roots: &BTreeMap<(String, String), Root>,
    edges: &[ProjectDependency],
) -> Option<(String, String)> {
    let mut queue = roots
        .iter()
        .filter(|(_, root)| root.symbol.is_public && root.symbol.kind == ProjectSymbolKind::Binding)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    while let Some(current) = queue.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        for edge in edges.iter().filter(|edge| key(edge.from()) == current) {
            let target = key(edge.to());
            if edge.kind == ProjectDependencyKind::Reference && !roots[&target].symbol.is_public {
                return Some(current);
            }
            if edge.kind == ProjectDependencyKind::Value {
                queue.push(target);
            }
        }
    }
    None
}

/// Parses a `ref(name)` or `ref(alias::name)` target at one token offset.
fn parse_reference(tokens: &[Token], index: usize) -> Option<(NameOccurrence, usize)> {
    if !matches!(tokens.get(index + 1)?.kind, TokenKind::OpenParen) {
        return None;
    }
    let (name, end) = parse_name(tokens, index + 2)?;
    matches!(tokens.get(end)?.kind, TokenKind::CloseParen).then_some((name, end + 1))
}

/// Parses one unqualified or alias-qualified name occurrence.
fn parse_name(tokens: &[Token], index: usize) -> Option<(NameOccurrence, usize)> {
    let first = tokens.get(index)?;
    let TokenKind::Identifier(name) = &first.kind else {
        return None;
    };
    if matches!(
        tokens.get(index + 1).map(|token| &token.kind),
        Some(TokenKind::DoubleColon)
    ) {
        let second = tokens.get(index + 2)?;
        let TokenKind::Identifier(target) = &second.kind else {
            return None;
        };
        Some((
            NameOccurrence {
                alias: Some(name.clone()),
                name: target.clone(),
                span: second.span,
            },
            index + 3,
        ))
    } else {
        Some((
            NameOccurrence {
                alias: None,
                name: name.clone(),
                span: first.span,
            },
            index + 1,
        ))
    }
}

/// Resolves one name using only the local module or one explicit import alias.
fn resolve_name(
    occurrence: &NameOccurrence,
    owner: &(String, String),
    module: &ModuleContext,
    roots: &BTreeMap<(String, String), Root>,
    errors: &mut Vec<ProjectSemanticDiagnostic>,
) -> Option<(String, String)> {
    let target_module = match &occurrence.alias {
        Some(alias) => module.aliases.get(alias).cloned(),
        None => Some(owner.0.clone()),
    };
    let target = target_module.map(|module| (module, occurrence.name.clone()));
    match target {
        Some(ref key)
            if roots
                .get(key)
                .is_some_and(|root| occurrence.alias.is_none() || root.symbol.is_public) =>
        {
            Some(key.clone())
        }
        _ => {
            errors.push(diagnostic(
                diagnostics::INACCESSIBLE_NAME,
                &owner.0,
                module.digest,
                occurrence.span,
            ));
            None
        }
    }
}

/// Constructs an identity edge with separate source provenance.
fn dependency(
    from: &Root,
    to: &Root,
    kind: ProjectDependencyKind,
    digest: SourceContentDigest,
    span: ByteSpan,
) -> ProjectDependency {
    ProjectDependency {
        from: from.symbol.identity.clone(),
        to: to.symbol.identity.clone(),
        kind,
        source: SourceLocation::new(digest, span),
    }
}

/// Returns the least root in an ordinary value or embedded-type dependency cycle.
fn cycle_root(
    roots: &BTreeMap<(String, String), Root>,
    edges: &[ProjectDependency],
) -> Option<(String, String)> {
    let mut remaining = roots.keys().cloned().collect::<BTreeSet<_>>();
    let mut dependencies = roots
        .keys()
        .cloned()
        .map(|key| (key, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for edge in edges {
        if matches!(
            edge.kind,
            ProjectDependencyKind::Type | ProjectDependencyKind::Value
        ) {
            dependencies
                .get_mut(&key(edge.from()))
                .expect("edge owner is collected")
                .insert(key(edge.to()));
        }
    }
    loop {
        let ready = remaining
            .iter()
            .filter(|node| dependencies[*node].is_disjoint(&remaining))
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            break;
        }
        for node in ready {
            remaining.remove(&node);
        }
    }
    remaining.into_iter().next()
}

/// Converts a structured module-symbol identity into its canonical map key.
fn key(identity: &ModuleSymbolIdentity) -> (String, String) {
    (
        identity.module().module_name().to_owned(),
        identity.declaration_name().to_owned(),
    )
}

/// Creates one bounded source diagnostic without leaking private target text.
fn diagnostic(
    code: &'static str,
    module: &str,
    digest: SourceContentDigest,
    span: ByteSpan,
) -> ProjectSemanticDiagnostic {
    ProjectSemanticDiagnostic {
        code,
        module_id: Arc::from(module),
        source: SourceLocation::new(digest, span),
    }
}

/// Returns a single-diagnostic failure.
fn single(diagnostic: ProjectSemanticDiagnostic) -> ProjectSemanticFailure {
    ProjectSemanticFailure {
        diagnostics: Arc::from(vec![diagnostic]),
    }
}

/// Orders failures and enforces the explicit diagnostic ceiling.
fn failure(mut errors: Vec<ProjectSemanticDiagnostic>, limit: u64) -> ProjectSemanticFailure {
    errors.sort_by(|a, b| {
        (a.module_id(), a.source.span().start(), a.code()).cmp(&(
            b.module_id(),
            b.source.span().start(),
            b.code(),
        ))
    });
    if errors.len() as u64 > limit {
        return single(diagnostic(
            diagnostics::LIMIT_EXCEEDED,
            errors[0].module_id(),
            errors[0].source.source(),
            errors[0].source.span(),
        ));
    }
    ProjectSemanticFailure {
        diagnostics: Arc::from(errors),
    }
}

/// Returns the zero-length source insertion span.
fn zero_span() -> ByteSpan {
    ByteSpan::new(0, 0).expect("zero span is valid")
}
