// SPDX-License-Identifier: Apache-2.0

//! Smoke the actual operations so benchmark coverage cannot decay to old-only labels.

/// Exercises current compilation, independent consumers, identity layers and real cache changes.
#[test]
fn expression_benchmark_exercises_current_pipeline() {
    let mut phases = Vec::new();
    super::run(1, |name, iterations, _elapsed| {
        assert!(iterations > 0);
        phases.push(name.to_owned());
    });
    assert_eq!(
        phases,
        [
            "expression-capture",
            "expression-compile",
            "expression-reader",
            "expression-encode",
            "expression-decode",
            "expression-identities",
            "expression-view",
            "expression-probe",
            "expression-cache-warm",
            "expression-cache-changed",
            "expression-growth",
            "expression-concurrent",
        ]
    );
}
