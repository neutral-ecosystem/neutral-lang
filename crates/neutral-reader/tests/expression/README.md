<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression reader qualification

These tests build complete independently validated artifacts without linking the
compiler. Companion ownership, public closure, exact entry selection, guarded
dependency inspection and explicit consumer reports have separate test modules.

`selection.rs` checks positive inclusive/one-over root, closure-edge, work and
output limits, shared selection/projection budgets, exact profile/features,
canonical ordering, private reuse redaction and every observed selection fault.
`consumer_coverage.rs` supplies shared raw fixtures; `dependency_regions.rs`
checks canonical paths and guard conditions across every retained expression form.
`boundary_coverage.rs` and `binding_reports.rs` check explicit policy/envelope
rejection, shared context/evaluation work and byte controls, and every observed
registration, binding-preflight and report allocation/cancellation with recovery.

Run `cargo test --package neutral-reader --lib` for these owning tests.
Cross-crate source → wire → reader cases live in the test suite's
`tests/expression_pipeline/` directory and run with `cargo xtask test integration`.
