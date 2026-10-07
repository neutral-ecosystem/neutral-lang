// SPDX-License-Identifier: Apache-2.0

//! Successor-only graph retention. Syntax is shared; no old-profile allocating owners are constructed.

use super::{CapturedCompositionProject, E, codes, fail, reserve, retain};
use crate::frontend::{GraphSyntaxErrorKind, scan_graph_source_bounded};
use neutral_core::{CancellationToken, SourceLocation, allocation::text};

/// Private complete topology; SCC traversal validates bounds without retaining a second graph projection.
pub(super) struct Graph {
    /// Canonical validated imports, with exact source-local aliases.
    edges: Vec<Import>,
}
impl Graph {
    /// Borrows canonical complete imports without allocation.
    pub(super) fn edges(&self) -> &[Import] {
        &self.edges
    }
}
/// One retained import; all strings are built through checked fallible reservations.
pub(super) struct Import {
    /// Logical importing module, not a host path.
    from: String,
    /// Exact captured target module.
    target: String,
    /// Source-local import qualifier.
    alias: String,
}
impl Import {
    /// Returns the logical importing module.
    pub(super) fn from(&self) -> &str {
        &self.from
    }
    /// Returns the logical target module.
    pub(super) fn target(&self) -> &str {
        &self.target
    }
    /// Returns the local qualifier without making it semantic identity.
    pub(super) fn alias(&self) -> &str {
        &self.alias
    }
}

/// Constructs bounded topology from captured bytes with fallible graph and worklist retention.
pub(super) fn build(
    captured: &CapturedCompositionProject,
    cancel: &CancellationToken,
) -> Result<Graph, E> {
    let limits = captured.limits().values();
    let mut work = captured.composition_limits().work;
    let syntax = scan(captured, &mut work, cancel)?;
    let (mut edges, positions) = imports(captured, &syntax, &mut work, cancel)?;
    scc(
        captured.sources().len(),
        &positions,
        limits.scc_units,
        &mut work,
        cancel,
    )?;
    charge(&mut work, edges.len() as u64, cancel)?;
    edges.sort_unstable_by(|a, b| {
        (&a.from, &a.target, &a.alias).cmp(&(&b.from, &b.target, &b.alias))
    });
    charge(&mut work, 1, cancel)?;
    Ok(Graph { edges })
}

/// Scans all units before edge validation, preserving syntax-first diagnostic precedence and count bounds.
fn scan(
    captured: &CapturedCompositionProject,
    work: &mut u64,
    cancel: &CancellationToken,
) -> Result<Vec<crate::frontend::GraphSourceSyntax>, E> {
    let limits = captured.limits().values();
    let mut result = Vec::new();
    let mut first = None;
    let mut errors = 0_u64;
    let mut edge_count = 0_u64;
    for source in captured.sources() {
        charge(work, source.bytes().len() as u64 + 1, cancel)?;
        let syntax = scan_graph_source_bounded(
            source.bytes(),
            source.module_id(),
            limits.imports_per_module,
            usize::try_from(captured.composition_limits().json.string_bytes())
                .unwrap_or(usize::MAX),
            Some(cancel),
        )
        .map_err(|error| {
            fail(
                if cancel.is_cancelled() {
                    codes::CANCELLED
                } else if error.kind == GraphSyntaxErrorKind::LimitExceeded {
                    codes::LIMIT
                } else {
                    codes::INVALID_SOURCE
                },
                Some(SourceLocation::new(source.digest(), error.span)),
            )
        });
        match syntax {
            Ok(syntax) => {
                edge_count = edge_count
                    .checked_add(syntax.imports.len() as u64)
                    .ok_or_else(|| fail(codes::LIMIT, None))?;
                if edge_count > limits.import_edges {
                    record_error(
                        &mut first,
                        &mut errors,
                        limits.diagnostics,
                        fail(
                            codes::LIMIT,
                            Some(SourceLocation::new(source.digest(), syntax.header_span)),
                        ),
                    )?;
                }
                reserve(&mut result, 1)?;
                result.push(syntax);
            }
            Err(error) => {
                if error.code == codes::CANCELLED {
                    return Err(error);
                }
                record_error(&mut first, &mut errors, limits.diagnostics, error)?;
            }
        }
    }
    if let Some(error) = first {
        return Err(error);
    }
    Ok(result)
}

/// Unpublished logical imports plus numeric edges for the bounded SCC traversal.
type Assembly = (Vec<Import>, Vec<(usize, usize)>);

/// Checks every edge in source order before returning complete topology; failed aliases still participate in diagnostics.
fn imports(
    captured: &CapturedCompositionProject,
    syntaxes: &[crate::frontend::GraphSourceSyntax],
    work: &mut u64,
    cancel: &CancellationToken,
) -> Result<Assembly, E> {
    let limits = captured.limits().values();
    let mut edges = Vec::new();
    let mut positions = Vec::new();
    let mut first = None;
    let mut errors = 0_u64;
    for (from, (source, syntax)) in captured.sources().iter().zip(syntaxes).enumerate() {
        let own_name = source.module_id().rsplit("::").next().unwrap_or("");
        let mut aliases = Vec::new();
        reserve(&mut aliases, syntax.vocabulary_aliases.len())?;
        aliases.extend(syntax.vocabulary_aliases.iter().map(String::as_str));
        let mut targets: Vec<&str> = Vec::new();
        reserve(&mut targets, syntax.imports.len())?;
        for import in &syntax.imports {
            charge(
                work,
                aliases.len() as u64 + targets.len() as u64 + 1,
                cancel,
            )?;
            let location = Some(SourceLocation::new(source.digest(), import.span));
            let duplicate = targets.contains(&import.target.as_str());
            if !duplicate {
                targets.push(&import.target);
            }
            let collision = import.alias == own_name || aliases.contains(&import.alias.as_str());
            if !duplicate && import.target != source.module_id() && !collision {
                reserve(&mut aliases, 1)?;
                aliases.push(import.alias.as_str());
            }
            let to = captured
                .sources()
                .binary_search_by(|s| s.module_id().cmp(&import.target));
            if duplicate || import.target == source.module_id() || collision || to.is_err() {
                record_error(
                    &mut first,
                    &mut errors,
                    limits.diagnostics,
                    fail(codes::INVALID_SOURCE, location),
                )?;
                continue;
            }
            reserve(&mut edges, 1)?;
            reserve(&mut positions, 1)?;
            edges.push(Import {
                from: retain(text(source.module_id()))?,
                target: retain(text(&import.target))?,
                alias: retain(text(&import.alias))?,
            });
            positions.push((from, to.map_err(|_| fail(codes::INVALID_SOURCE, location))?));
        }
    }
    if let Some(error) = first {
        return Err(error);
    }
    Ok((edges, positions))
}

/// Retains only the first canonical diagnostic while bounding the complete failing set without allocation.
fn record_error(first: &mut Option<E>, count: &mut u64, limit: u64, error: E) -> Result<(), E> {
    *count = count
        .checked_add(1)
        .ok_or_else(|| fail(codes::LIMIT, None))?;
    if *count > limit {
        return Err(fail(codes::LIMIT, None));
    }
    if first.is_none() {
        *first = Some(error);
    }
    Ok(())
}

/// Charges traversal before retention and checks cancellation at every graph boundary.
fn charge(work: &mut u64, count: u64, cancel: &CancellationToken) -> Result<(), E> {
    if cancel.is_cancelled() {
        return Err(fail(codes::CANCELLED, None));
    }
    *work = work
        .checked_sub(count)
        .ok_or_else(|| fail(codes::LIMIT, None))?;
    Ok(())
}

/// Validates SCC sizes using iterative Kosaraju passes; loops never recurse on the Rust stack.
fn scc(
    count: usize,
    edges: &[(usize, usize)],
    limit: u64,
    work: &mut u64,
    cancel: &CancellationToken,
) -> Result<(), E> {
    charge(work, count as u64, cancel)?;
    let mut forward: Vec<Vec<usize>> = Vec::new();
    let mut reverse: Vec<Vec<usize>> = Vec::new();
    reserve(&mut forward, count)?;
    reserve(&mut reverse, count)?;
    forward.resize_with(count, Vec::new);
    reverse.resize_with(count, Vec::new);
    for &(from, to) in edges {
        charge(work, 1, cancel)?;
        reserve(&mut forward[from], 1)?;
        reserve(&mut reverse[to], 1)?;
        forward[from].push(to);
        reverse[to].push(from);
    }
    let mut seen = Vec::new();
    reserve(&mut seen, count)?;
    seen.resize(count, false);
    let mut finishing = Vec::new();
    let mut frames = Vec::new();
    reserve(&mut finishing, count)?;
    reserve(&mut frames, count)?;
    for root in 0..count {
        charge(work, 1, cancel)?;
        if seen[root] {
            continue;
        }
        seen[root] = true;
        frames.push((root, 0));
        while let Some((node, child)) = frames.last_mut() {
            charge(work, 1, cancel)?;
            if let Some(&next) = forward[*node].get(*child) {
                *child += 1;
                if !seen[next] {
                    seen[next] = true;
                    frames.push((next, 0));
                }
            } else {
                finishing.push(*node);
                frames.pop();
            }
        }
    }
    seen.fill(false);
    let mut stack = Vec::new();
    reserve(&mut stack, count)?;
    for &root in finishing.iter().rev() {
        charge(work, 1, cancel)?;
        if seen[root] {
            continue;
        }
        seen[root] = true;
        stack.push(root);
        let mut size = 0_u64;
        while let Some(node) = stack.pop() {
            charge(work, 1, cancel)?;
            size += 1;
            if size > limit {
                return Err(fail(codes::LIMIT, None));
            }
            for &next in &reverse[node] {
                charge(work, 1, cancel)?;
                if !seen[next] {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/project_cache/composition_graph.rs"]
mod tests;
