// SPDX-License-Identifier: Apache-2.0

//! Exact SCC boundaries, iterative traversal and independent successor graph work controls.

use super::*;

/// A valid source cycle accepts its exact SCC size, but one-over never publishes topology.
#[test]
fn security_successor_graph_scc_size_boundary_is_exact() {
    for limit in [2, 3, 4] {
        let result = scc(
            3,
            &[(0, 1), (1, 2), (2, 0)],
            limit,
            &mut 1000,
            &CancellationToken::new(),
        );
        if limit < 3 {
            assert_eq!(result.unwrap_err().code, codes::LIMIT);
        } else {
            result.unwrap();
        }
    }
    // Disconnected and singleton components do not inherit the largest component's count.
    scc(
        5,
        &[(0, 1), (1, 2), (2, 0), (3, 4)],
        3,
        &mut 1000,
        &CancellationToken::new(),
    )
    .unwrap();
}

/// Traversal work is checked independently from retained graph size, before each expansion.
#[test]
fn security_successor_graph_work_boundary_and_cancellation_are_exact() {
    let edges = [(0, 1), (1, 2), (0, 2)];
    let mut remaining = 1000;
    scc(3, &edges, 1, &mut remaining, &CancellationToken::new()).unwrap();
    let exact = 1000 - remaining;
    for budget in [exact - 1, exact, exact + 1] {
        let mut remaining = budget;
        let result = scc(3, &edges, 1, &mut remaining, &CancellationToken::new());
        if budget < exact {
            assert_eq!(result.unwrap_err().code, codes::LIMIT);
        } else {
            result.unwrap();
        }
    }
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        scc(3, &edges, 1, &mut 1000, &cancel).unwrap_err().code,
        codes::CANCELLED
    );
}

/// Long acyclic import chains use reserved iterative worklists, never recursive DFS stack frames.
#[test]
fn security_successor_graph_long_chain_uses_iterative_traversal() {
    let count = 10_000;
    let edges = (0..count - 1).map(|n| (n, n + 1)).collect::<Vec<_>>();
    scc(count, &edges, 1, &mut 1_000_000, &CancellationToken::new()).unwrap();
}
