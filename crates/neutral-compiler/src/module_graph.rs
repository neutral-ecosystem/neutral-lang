// SPDX-License-Identifier: Apache-2.0

//! Deterministic, bounded module graph construction from immutable captured sources.

use crate::{
    CapturedProject, CapturedProjectSource,
    frontend::{GraphImport, GraphSyntaxErrorKind, scan_graph_source},
};
use neutral_core::{ByteSpan, CancellationToken, SourceContentDigest, SourceLocation};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Stable diagnostic codes for the v1 module graph contract.
pub mod diagnostics {
    /// Invalid module/import syntax or placement.
    pub const INVALID_SYNTAX: &str = "NEU-MOD-001";
    /// Imported module absent from the complete captured source set.
    pub const MISSING_IMPORT: &str = "NEU-MOD-002";
    /// A module directly imports itself.
    pub const SELF_IMPORT: &str = "NEU-MOD-003";
    /// The same target is imported more than once in a module.
    pub const DUPLICATE_IMPORT: &str = "NEU-MOD-004";
    /// A local import alias collides with another occupied alias.
    pub const ALIAS_COLLISION: &str = "NEU-MOD-005";
    /// A forbidden import form attempted acquisition or re-export syntax.
    pub const FORBIDDEN_IMPORT: &str = "NEU-MOD-006";
    /// A graph count, SCC size, or diagnostic bound was exceeded.
    pub const LIMIT_EXCEEDED: &str = "NEU-MOD-007";
    /// Cooperative cancellation stopped graph construction.
    pub const CANCELLED: &str = "NEU-MOD-008";
}

/// One source-accounted module in canonical module-ID order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphModule {
    /// Exact logical module ID.
    module_id: Arc<str>,
    /// Exact inert source ID.
    source_id: Arc<str>,
    /// Exact captured source-byte identity.
    source_digest: SourceContentDigest,
    /// Module-header span in original source bytes.
    header_span: ByteSpan,
}

impl GraphModule {
    /// Returns the exact logical module ID.
    #[must_use]
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Returns the exact source ID for source-map construction.
    #[must_use]
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Returns a typed original-byte location for the module header.
    #[must_use]
    pub const fn source_location(&self) -> SourceLocation {
        SourceLocation::new(self.source_digest, self.header_span)
    }

    /// Returns the original-byte module-header span.
    #[must_use]
    pub const fn header_span(&self) -> ByteSpan {
        self.header_span
    }
}

/// One directed, source-accounted import edge in canonical order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEdge {
    /// Importing module ID.
    from: Arc<str>,
    /// Exact target module ID from the complete captured set.
    target: Arc<str>,
    /// Required local alias.
    alias: Arc<str>,
    /// Exact source ID of the importing unit.
    source_id: Arc<str>,
    /// Exact captured source-byte identity of the importing unit.
    source_digest: SourceContentDigest,
    /// Import statement span in original source bytes.
    span: ByteSpan,
}

impl GraphEdge {
    /// Returns the importing module ID.
    #[must_use]
    pub fn from(&self) -> &str {
        &self.from
    }

    /// Returns the target module ID.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the local import alias.
    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Returns the source ID of the unit containing this import.
    #[must_use]
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Returns a typed original-byte location for the complete import.
    #[must_use]
    pub const fn source_location(&self) -> SourceLocation {
        SourceLocation::new(self.source_digest, self.span)
    }

    /// Returns the original-byte import span.
    #[must_use]
    pub const fn span(&self) -> ByteSpan {
        self.span
    }
}

/// One canonically ordered strongly connected component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphComponent {
    /// Member module IDs in lexical order.
    modules: Arc<[Arc<str>]>,
}

impl GraphComponent {
    /// Returns every member module ID in canonical order.
    #[must_use]
    pub fn modules(&self) -> &[Arc<str>] {
        &self.modules
    }
}

/// Complete immutable graph and dependency-first SCC condensation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleGraph {
    /// All modules in canonical ID order, including disconnected units.
    modules: Arc<[GraphModule]>,
    /// All import edges in canonical source/target/alias order.
    edges: Arc<[GraphEdge]>,
    /// SCCs in dependency-first order.
    components: Arc<[GraphComponent]>,
}

impl ModuleGraph {
    /// Returns every captured module, including disconnected members.
    #[must_use]
    pub fn modules(&self) -> &[GraphModule] {
        &self.modules
    }

    /// Returns all validated logical import edges.
    #[must_use]
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// Returns dependency-first strongly connected components.
    #[must_use]
    pub fn components(&self) -> &[GraphComponent] {
        &self.components
    }
}

/// One stable, source-accounted module graph diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleGraphDiagnostic {
    /// Stable NEU-MOD diagnostic code.
    code: &'static str,
    /// Exact logical module ID, if one is available.
    module_id: Arc<str>,
    /// Exact source ID, if one is available.
    source_id: Arc<str>,
    /// Exact captured source-byte identity, absent for graph-wide failures.
    source_digest: Option<SourceContentDigest>,
    /// Original-byte span in the captured source.
    span: ByteSpan,
    /// Safe logical target ID, when applicable.
    target: Arc<str>,
    /// Safe local alias, when applicable.
    alias: Arc<str>,
}

impl ModuleGraphDiagnostic {
    /// Returns the stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Returns the logical module in which the failure occurred.
    #[must_use]
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Returns the exact source ID for source-map consumers.
    #[must_use]
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Returns a typed original-byte location, or none for a graph-wide failure.
    #[must_use]
    pub fn source_location(&self) -> Option<SourceLocation> {
        self.source_digest
            .map(|digest| SourceLocation::new(digest, self.span))
    }

    /// Returns the original-byte failure span.
    #[must_use]
    pub const fn span(&self) -> ByteSpan {
        self.span
    }

    /// Returns the safe target module ID, if present.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns the safe local alias, if present.
    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }
}

/// Complete bounded failure without any published partial graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleGraphFailure {
    /// Canonically ordered diagnostics.
    diagnostics: Arc<[ModuleGraphDiagnostic]>,
}

impl ModuleGraphFailure {
    /// Returns all retained diagnostics in canonical order.
    #[must_use]
    pub fn diagnostics(&self) -> &[ModuleGraphDiagnostic] {
        &self.diagnostics
    }
}

/// One parsed source awaiting cross-module validation.
struct PendingSource<'a> {
    /// Canonically ordered captured source identity.
    source: GraphModule,
    /// Captured source retained for exact diagnostic accounting.
    captured_source: &'a CapturedProjectSource,
    /// Source-order imports with exact spans.
    imports: Vec<GraphImport>,
    /// Captured vocabulary aliases occupying the same namespace.
    vocabulary_aliases: Vec<String>,
}

/// Constructs an immutable import graph using only the complete captured project.
///
/// # Errors
///
/// Returns bounded ordered diagnostics and publishes no graph when parsing,
/// closure validation, a resource bound, or cancellation fails.
pub fn build_module_graph(
    captured: &CapturedProject,
    cancellation: &CancellationToken,
) -> Result<ModuleGraph, ModuleGraphFailure> {
    let limits = captured.limits().values();
    let pending = scan_sources(captured, cancellation)?;
    let positions = pending
        .iter()
        .enumerate()
        .map(|(index, member)| (member.source.module_id().to_owned(), index))
        .collect::<BTreeMap<_, _>>();
    let edges = validate_edges(&pending, &positions, limits.diagnostics, cancellation)?;
    let components = condense(&pending, &edges, &positions, limits.scc_units, cancellation)
        .map_err(|code| single_failure(graph_wide_diagnostic(code)))?;
    if cancellation.is_cancelled() {
        return Err(single_failure(graph_wide_diagnostic(
            diagnostics::CANCELLED,
        )));
    }
    Ok(ModuleGraph {
        modules: Arc::from(
            pending
                .into_iter()
                .map(|member| member.source)
                .collect::<Vec<_>>(),
        ),
        edges: Arc::from(edges),
        components: Arc::from(components),
    })
}

/// Parses every captured source and enforces import/edge counts before edge allocation.
fn scan_sources<'a>(
    captured: &'a CapturedProject,
    cancellation: &CancellationToken,
) -> Result<Vec<PendingSource<'a>>, ModuleGraphFailure> {
    let limits = captured.limits().values();
    let mut pending = Vec::with_capacity(captured.sources().len());
    let mut errors = Vec::new();
    let mut edge_count = 0_u64;
    for source in captured.sources() {
        if cancellation.is_cancelled() {
            return Err(single_failure(diagnostic(
                diagnostics::CANCELLED,
                source,
                zero_span(),
                "",
                "",
            )));
        }
        match scan_graph_source(
            source.bytes(),
            source.module_id(),
            limits.imports_per_module,
        ) {
            Ok(syntax) => {
                let count = u64::try_from(syntax.imports.len()).unwrap_or(u64::MAX);
                let over_limit = match edge_count.checked_add(count) {
                    Some(total) => {
                        edge_count = total;
                        total > limits.import_edges
                    }
                    None => true,
                };
                if over_limit {
                    errors.push(diagnostic(
                        diagnostics::LIMIT_EXCEEDED,
                        source,
                        syntax.header_span,
                        "",
                        "",
                    ));
                }
                pending.push(PendingSource {
                    source: GraphModule {
                        module_id: Arc::from(source.module_id()),
                        source_id: Arc::from(source.source_id()),
                        source_digest: source.digest(),
                        header_span: syntax.header_span,
                    },
                    captured_source: source,
                    imports: syntax.imports,
                    vocabulary_aliases: syntax.vocabulary_aliases,
                });
            }
            Err(error) => {
                let code = match error.kind {
                    GraphSyntaxErrorKind::InvalidSyntax => diagnostics::INVALID_SYNTAX,
                    GraphSyntaxErrorKind::ForbiddenImport => diagnostics::FORBIDDEN_IMPORT,
                    GraphSyntaxErrorKind::AliasCollision => diagnostics::ALIAS_COLLISION,
                    GraphSyntaxErrorKind::LimitExceeded => diagnostics::LIMIT_EXCEEDED,
                };
                errors.push(diagnostic(code, source, error.span, "", ""));
            }
        }
    }
    if !errors.is_empty() {
        return Err(failure(errors, limits.diagnostics));
    }
    Ok(pending)
}

/// Validates all local aliases and graph targets, then canonically orders edges.
fn validate_edges(
    pending: &[PendingSource<'_>],
    positions: &BTreeMap<String, usize>,
    diagnostics_limit: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<GraphEdge>, ModuleGraphFailure> {
    let mut edges = Vec::new();
    let mut errors = Vec::new();
    for member in pending {
        let source = member.captured_source;
        let mut aliases = member
            .vocabulary_aliases
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut targets = BTreeSet::new();
        let own_name = member.source.module_id().rsplit("::").next().unwrap_or("");
        for import in &member.imports {
            if cancellation.is_cancelled() {
                return Err(single_failure(diagnostic(
                    diagnostics::CANCELLED,
                    source,
                    import.span,
                    "",
                    "",
                )));
            }
            let code = if !targets.insert(import.target.clone()) {
                Some(diagnostics::DUPLICATE_IMPORT)
            } else if import.target == member.source.module_id() {
                Some(diagnostics::SELF_IMPORT)
            } else if import.alias == own_name || !aliases.insert(import.alias.clone()) {
                Some(diagnostics::ALIAS_COLLISION)
            } else if !positions.contains_key(&import.target) {
                Some(diagnostics::MISSING_IMPORT)
            } else {
                None
            };
            if let Some(code) = code {
                errors.push(diagnostic(
                    code,
                    source,
                    import.span,
                    &import.target,
                    &import.alias,
                ));
            } else {
                edges.push(GraphEdge {
                    from: Arc::clone(&member.source.module_id),
                    target: Arc::from(import.target.as_str()),
                    alias: Arc::from(import.alias.as_str()),
                    source_id: Arc::clone(&member.source.source_id),
                    source_digest: member.source.source_digest,
                    span: import.span,
                });
            }
        }
    }
    if !errors.is_empty() {
        return Err(failure(errors, diagnostics_limit));
    }
    edges.sort_by(|left, right| {
        (left.from(), left.target(), left.alias(), left.span.start()).cmp(&(
            right.from(),
            right.target(),
            right.alias(),
            right.span.start(),
        ))
    });
    Ok(edges)
}

/// Returns a locationless diagnostic for a graph-wide resource or cancellation failure.
fn graph_wide_diagnostic(code: &'static str) -> ModuleGraphDiagnostic {
    ModuleGraphDiagnostic {
        code,
        module_id: Arc::from(""),
        source_id: Arc::from(""),
        source_digest: None,
        span: zero_span(),
        target: Arc::from(""),
        alias: Arc::from(""),
    }
}

/// Constructs one source-accounted diagnostic using only bounded logical names.
fn diagnostic(
    code: &'static str,
    source: &CapturedProjectSource,
    span: ByteSpan,
    target: &str,
    alias: &str,
) -> ModuleGraphDiagnostic {
    ModuleGraphDiagnostic {
        code,
        module_id: Arc::from(source.module_id()),
        source_id: Arc::from(source.source_id()),
        source_digest: Some(source.digest()),
        span,
        target: Arc::from(target),
        alias: Arc::from(alias),
    }
}

/// Returns a canonical bounded failure, replacing overflow with one limit error.
fn failure(mut errors: Vec<ModuleGraphDiagnostic>, limit: u64) -> ModuleGraphFailure {
    errors.sort_by(|left, right| {
        (
            left.module_id(),
            left.span.start(),
            left.code(),
            left.target(),
            left.alias(),
        )
            .cmp(&(
                right.module_id(),
                right.span.start(),
                right.code(),
                right.target(),
                right.alias(),
            ))
    });
    if u64::try_from(errors.len()).unwrap_or(u64::MAX) > limit {
        return single_failure(graph_wide_diagnostic(diagnostics::LIMIT_EXCEEDED));
    }
    ModuleGraphFailure {
        diagnostics: Arc::from(errors),
    }
}

/// Wraps one terminal cancellation or resource failure.
fn single_failure(error: ModuleGraphDiagnostic) -> ModuleGraphFailure {
    ModuleGraphFailure {
        diagnostics: Arc::from(vec![error]),
    }
}

/// Returns the zero-width span used when a graph-wide failure has no source.
fn zero_span() -> ByteSpan {
    ByteSpan::new(0, 0).expect("zero-width span is valid")
}

/// Computes iterative SCCs and a stable dependency-first condensation order.
fn condense(
    modules: &[PendingSource<'_>],
    edges: &[GraphEdge],
    positions: &BTreeMap<String, usize>,
    scc_limit: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<GraphComponent>, &'static str> {
    let groups =
        strongly_connected_groups(modules.len(), edges, positions, scc_limit, cancellation)?;
    order_components(modules, edges, positions, &groups, cancellation)
}

/// Finds strongly connected groups with iterative depth-first traversals.
///
/// This is the two-pass Kosaraju traversal: finish nodes in the forward graph,
/// then traverse the reverse graph in reverse finishing order. Explicit stacks
/// bound call-stack use even for a long import chain. Sorting neighbors and group
/// members makes later condensation independent of input enumeration order.
fn strongly_connected_groups(
    count: usize,
    edges: &[GraphEdge],
    positions: &BTreeMap<String, usize>,
    scc_limit: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<Vec<usize>>, &'static str> {
    let mut forward = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for edge in edges {
        let from = positions[edge.from()];
        let target = positions[edge.target()];
        forward[from].push(target);
        reverse[target].push(from);
    }
    for neighbors in &mut forward {
        neighbors.sort_unstable();
    }
    for neighbors in &mut reverse {
        neighbors.sort_unstable();
    }
    let mut visited = vec![false; count];
    let mut finishing = Vec::with_capacity(count);
    for root in 0..count {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some(&(node, child)) = stack.last() {
            if cancellation.is_cancelled() {
                return Err(diagnostics::CANCELLED);
            }
            if child < forward[node].len() {
                stack.last_mut().expect("active DFS frame").1 += 1;
                let next = forward[node][child];
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                finishing.push(node);
                stack.pop();
            }
        }
    }
    visited.fill(false);
    // Reverse finishing order makes each reverse traversal stay within one SCC.
    let mut groups = Vec::new();
    for &root in finishing.iter().rev() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![root];
        let mut group = Vec::new();
        while let Some(node) = stack.pop() {
            if cancellation.is_cancelled() {
                return Err(diagnostics::CANCELLED);
            }
            group.push(node);
            if u64::try_from(group.len()).unwrap_or(u64::MAX) > scc_limit {
                return Err(diagnostics::LIMIT_EXCEEDED);
            }
            for &next in &reverse[node] {
                if !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        group.sort_unstable();
        groups.push(group);
    }
    Ok(groups)
}

/// Topologically orders SCCs by dependency and lexical component identity.
fn order_components(
    modules: &[PendingSource<'_>],
    edges: &[GraphEdge],
    positions: &BTreeMap<String, usize>,
    groups: &[Vec<usize>],
    cancellation: &CancellationToken,
) -> Result<Vec<GraphComponent>, &'static str> {
    let count = modules.len();
    let mut component_of = vec![0_usize; count];
    for (index, group) in groups.iter().enumerate() {
        for &member in group {
            component_of[member] = index;
        }
    }
    let mut dependencies = vec![BTreeSet::new(); groups.len()];
    let mut dependents = vec![BTreeSet::new(); groups.len()];
    for edge in edges {
        let from = component_of[positions[edge.from()]];
        let target = component_of[positions[edge.target()]];
        if from != target && dependencies[from].insert(target) {
            dependents[target].insert(from);
        }
    }
    let mut remaining = dependencies.iter().map(BTreeSet::len).collect::<Vec<_>>();
    let mut ready = BTreeSet::new();
    for (index, group) in groups.iter().enumerate() {
        if remaining[index] == 0 {
            ready.insert((modules[group[0]].source.module_id().to_owned(), index));
        }
    }
    let mut ordered = Vec::with_capacity(groups.len());
    while let Some((_, index)) = ready.pop_first() {
        if cancellation.is_cancelled() {
            return Err(diagnostics::CANCELLED);
        }
        ordered.push(GraphComponent {
            modules: Arc::from(
                groups[index]
                    .iter()
                    .map(|&member| Arc::clone(&modules[member].source.module_id))
                    .collect::<Vec<_>>(),
            ),
        });
        for &dependent in &dependents[index] {
            remaining[dependent] -= 1;
            if remaining[dependent] == 0 {
                ready.insert((
                    modules[groups[dependent][0]].source.module_id().to_owned(),
                    dependent,
                ));
            }
        }
    }
    Ok(ordered)
}

#[cfg(test)]
#[path = "../tests/module_graph/mod.rs"]
mod tests;
