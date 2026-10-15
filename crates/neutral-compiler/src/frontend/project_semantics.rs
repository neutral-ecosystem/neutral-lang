// SPDX-License-Identifier: Apache-2.0

//! Coordinate effect-free semantic analysis of captured logical modules.
//!
//! Shared declaration state and diagnostics live here; private syntax, resolution
//! and interface modules own parsing, cross-module checks and public projection.
//! Lowering and cache entry points compose those phases without acquiring source
//! input. The composition pipeline has separate request state and failure handling.

mod syntax;
use syntax::{is_requirement_or_import, parse_complete_type_fallible, parse_roots};
mod interface;
use interface::{build_public_interface, public_type};
mod resolution;
use resolution::{
    cycle_root, exposed_private_reference, parse_name, parse_reference, resolve_name, resolve_type,
    resolve_value,
};

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

#[path = "project_cache.rs"]
mod cache;
mod composition;
pub use cache::{ProjectCacheLimits, ProjectCacheStats, ProjectCompilationCache};
pub use composition::codes as composition_diagnostics;
pub use composition::{
    CompositionCompilationCache, CompositionCompileFailure, ExpressionCompileFailure,
    compile_composition_project, compile_expression_project,
};

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

impl neutral_core::allocation::TryClone for TypeExpr {
    /// Copies bounded syntax types and every recursive wrapper fallibly for successor cache retention.
    fn try_clone(&self) -> Result<Self, neutral_core::allocation::AllocationError> {
        Ok(match self {
            Self::Num => Self::Num,
            Self::String => Self::String,
            Self::Bool => Self::Bool,
            Self::Url => Self::Url,
            Self::Path => Self::Path,
            Self::Nominal(alias, name, span) => {
                Self::Nominal(alias.try_clone()?, name.try_clone()?, *span)
            }
            Self::List(inner) => Self::List(inner.try_clone()?),
            Self::Ref(inner) => Self::Ref(inner.try_clone()?),
            Self::Nullable(inner) => Self::Nullable(inner.try_clone()?),
        })
    }
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
pub fn analyze_project_semantics(
    captured: &CapturedProject,
    graph: &ModuleGraph,
    cancellation: &CancellationToken,
) -> Result<ProjectSemanticModel, ProjectSemanticFailure> {
    analyze_project_semantics_with_parser(captured, graph, cancellation, &mut |source| {
        parse_roots(source.module_id(), source.digest(), source.bytes())
    })
    .map(|resolved| resolved.model)
}

/// Validated semantic conclusions and their request-local parsed declaration scope.
struct ResolvedProject {
    /// Public conclusions exposed without leaking private syntax.
    model: ProjectSemanticModel,
    /// Exact roots passed directly to contextual lowering, not reparsed.
    roots: BTreeMap<(String, String), Root>,
}

/// Resolves all project semantics while allowing only the unit parsing phase to be reused.
///
/// Bind the graph to the exact captured units before resolving anything. Collect
/// the complete declaration scope before following dependencies, so source order
/// cannot determine whether a name exists. Visibility, public type closure, and
/// dependency-cycle checks run on every request, even when the supplied internal
/// parser returns cached roots. Parsed syntax is not a cached semantic conclusion.
#[expect(
    clippy::too_many_lines,
    reason = "explicit fail-closed project phases remain visible in one orchestration boundary"
)]
fn analyze_project_semantics_with_parser(
    captured: &CapturedProject,
    graph: &ModuleGraph,
    cancellation: &CancellationToken,
    parse: &mut impl FnMut(&crate::CapturedProjectSource) -> Result<Vec<Root>, ProjectSemanticFailure>,
) -> Result<ResolvedProject, ProjectSemanticFailure> {
    let (first_module_id, first_digest) = match captured.sources().first() {
        Some(source) => (source.module_id(), source.digest()),
        None => ("", SourceContentDigest::from_bytes(&[])),
    };
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
        return Err(single(diagnostic(
            diagnostics::GRAPH_MISMATCH,
            first_module_id,
            first_digest,
            zero_span(),
        )));
    }
    if cancellation.is_cancelled() {
        return Err(single(diagnostic(
            diagnostics::CANCELLED,
            first_module_id,
            first_digest,
            zero_span(),
        )));
    }
    let vocabularies = validate_project_vocabularies(captured).map_err(|_| {
        single(diagnostic(
            diagnostics::INVALID_VOCABULARY,
            first_module_id,
            first_digest,
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
        let declarations = parse(source)?;
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
            single(diagnostic(
                diagnostics::LIMIT_EXCEEDED,
                first_module_id,
                first_digest,
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
        .values()
        .map(|root| root.symbol.clone())
        .collect::<Vec<_>>();
    let model = ProjectSemanticModel {
        symbols: Arc::from(symbols),
        dependencies: Arc::from(edges),
        value_order: Arc::from(value_order),
        public_interface: Arc::new(public_interface),
        locations: Arc::from(locations),
    };
    Ok(ResolvedProject { model, roots })
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
