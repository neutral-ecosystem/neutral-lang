// SPDX-License-Identifier: Apache-2.0

//! Smoke the actual operations so benchmark coverage cannot decay to old-only labels.
//!
//! Runs the shared pipeline at one measured iteration per ordinary phase and
//! asserts its complete ordered phase inventory. This catches disconnected labels,
//! skipped operations and broken setup through the same assertions as the real
//! harness. Growth/concurrency keep their own bounded counts even in this smoke run.
//! No timing threshold is asserted: scheduler noise should not turn a correctness
//! smoke test into a performance gate or substitute for controlled measurements.

/// Exercises current compilation, independent consumers, identity layers and real cache changes.
#[test]
fn expression_benchmark_exercises_current_pipeline() {
    let mut phases = Vec::new();
    super::run(1, |name, iterations, _elapsed| {
        // Check that a reported phase did actual work without treating wall time as a stable oracle.
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
