<!-- SPDX-License-Identifier: Apache-2.0 -->

# Borrowed expression preflight

[`mod.rs`](mod.rs) checks all branches, types, lexical-reference shapes and selector
storage under explicit hard/caller bounds. [`payload.rs`](payload.rs) shares the
same budget for nested materialized values; skipped branches are never skipped
by inspection.

`inspect_expression_shape` returns isolated tree counts.
`inspect_expression_retention` also returns charged work and owned storage.
`inspect_expression_value_retention` checks a materialized body without inventing
a residual node; its residual shape counts are empty. Callers inspecting several
bodies must debit returned costs from their remaining request budget.

Inspection allocates nothing and confers no nominal typing, visibility, binding,
companion or identity authority. Run `cargo nextest run --config-file
config/nextest.toml --package neutral-ir --test expression -E 'test(inspection)'`.
