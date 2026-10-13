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

`inspect_public_template_retention` shares the same preflight for ordered parameter
types/defaults and the body, including template metadata storage. It intersects
expression and expansion controls; semantic consumers still check scopes, exact
types, visibility and application-time phase requirements.

Inspection allocates nothing and confers no nominal typing, visibility, binding,
companion or identity authority. Run `cargo nextest run --config-file
config/nextest.toml --package neutral-ir --test expression -E 'test(inspection)'`.
