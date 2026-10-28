// SPDX-License-Identifier: Apache-2.0

//! Literal reservation costs for retained occurrence and guard paths, independent of search-derived bounds.

use super::*;
use neutral_ir::{LogicalModuleIdentity, project_interface::ProjectPublicType};

/// Supplies a request-local traversal with independent work and retained-byte ceilings.
fn walk<'a>(
    owner: &'a ModuleSymbolIdentity,
    cancel: &'a CancellationToken,
    bytes: u64,
    work: u64,
) -> Walk<'a, 'a> {
    Walk {
        owner,
        root: ExpressionRoot::Binding,
        path: vec![3, 4],
        guards: Vec::new(),
        edges: Vec::new(),
        work,
        bytes,
        edge_limit: 2,
        cancel,
    }
}

/// An emitted occurrence retains its path and every enclosing guard path, charging each reservation.
#[test]
fn emitted_dependency_has_exact_path_and_guard_storage() {
    let owner = ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "m"), "owner");
    let cancel = CancellationToken::new();
    let bytes = std::mem::size_of::<ExpressionDependency<'_>>() as u64
        + std::mem::size_of::<DependencyGuard<'_>>() as u64
        + 32;
    // Two u64 indices per path, two paths; four storage reservations add four work units.
    for (available_bytes, available_work, success) in [
        (bytes, bytes + 4, true),
        (bytes - 1, bytes + 4, false),
        (bytes, bytes + 3, false),
    ] {
        let mut state = walk(&owner, &cancel, available_bytes, available_work);
        state.guards.push(DependencyGuard {
            node_path: vec![1, 2],
            condition: GuardCondition::Boolean(true),
        });
        let result = state.emit(BindingKey::Input(&owner));
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(state.bytes, 0);
            assert_eq!(state.work, 0);
            assert_eq!(state.edges[0].node_path, [3, 4]);
            assert_eq!(state.edges[0].guards[0].node_path, [1, 2]);
        } else {
            assert_eq!(result, Err(E::Limit));
            assert_eq!(state.edges, []);
        }
    }
}

/// A nested guard stores its condition path separately from the emitted branch occurrence.
#[test]
fn nested_guard_has_exact_temporary_and_retained_storage() {
    let owner = ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "m"), "owner");
    let cancel = CancellationToken::new();
    let input = Expression {
        ty: ProjectPublicType::Bool,
        node: N::Input(owner.clone()),
    };
    // Three paths of three u64s, one temporary child index and two guard wrappers.
    let bytes = std::mem::size_of::<ExpressionDependency<'_>>() as u64
        + 2 * std::mem::size_of::<DependencyGuard<'_>>() as u64
        + 80;
    // Six storage reservations and one tree visit.
    for (available_bytes, available_work, success) in [
        (bytes, bytes + 7, true),
        (bytes - 1, bytes + 7, false),
        (bytes, bytes + 6, false),
    ] {
        let mut state = walk(&owner, &cancel, available_bytes, available_work);
        let result = state.guarded(&input, 1, 0, GuardCondition::Boolean(false));
        assert_eq!(result.is_ok(), success);
        assert_eq!(state.path, [3, 4]);
        assert_eq!(state.guards, []);
        if success {
            assert_eq!(state.bytes, 0);
            assert_eq!(state.work, 0);
            assert_eq!(state.edges[0].node_path, [3, 4, 1]);
            assert_eq!(state.edges[0].guards[0].node_path, [3, 4, 0]);
        } else {
            assert_eq!(result, Err(E::Limit));
            assert_eq!(state.edges, []);
        }
    }
}
