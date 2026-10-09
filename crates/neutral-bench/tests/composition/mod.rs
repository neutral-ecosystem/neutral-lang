// SPDX-License-Identifier: Apache-2.0

//! Smoke the actual operations so benchmark coverage cannot decay to old-only labels.

/// Exercises current compilation, independent consumers, identity layers and real cache changes.
#[test]
fn composition_benchmark_exercises_current_pipeline() {
    let mut phases = Vec::new();
    super::run(1, |name, iterations, _elapsed| {
        assert!(iterations > 0);
        phases.push(name.to_owned());
    });
    assert_eq!(
        phases,
        [
            "composition-capture",
            "composition-graph",
            "composition-compile",
            "composition-reader",
            "composition-encode",
            "composition-decode",
            "composition-view",
            "composition-identities",
            "composition-cache-warm",
            "composition-cache-changed",
            "composition-growth",
            "composition-concurrent",
        ]
    );
}
