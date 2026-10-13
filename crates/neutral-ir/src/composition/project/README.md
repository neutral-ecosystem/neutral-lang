<!-- SPDX-License-Identifier: Apache-2.0 -->

# Successor project structure

This module owns raw complete composition-project data and bounded retained-data
inspection. Logical declarations and vocabulary contracts stay separate from
source maps, provenance, origins, limits and accounting. Inspection bounds raw
structures before recursive consumers or clones; it is not semantic validation.
The compiler produces these contracts, the independent reader validates them,
and the codec transports only independently validated complete artifacts.

[`mod.rs`](mod.rs) owns signatures, complete envelopes, origin paths and named
policy/resource tuples. `values`/`from_values` preserve frozen tuple order;
`intersect` takes the smaller bound independently for each dimension, without
validating positivity or producer claims.

[`inspection.rs`](inspection.rs) performs bounded borrowed traversal before
recursive copying. `inspect_composition_bodies` handles nominal bodies without
fabricating a project; its work projection supports aggregate caller accounting.
Neither inspection function resolves names, grants visibility or checks identities.

Run `cargo xtask test all` for producer/reader coverage. Focused raw preflight tests
are in [the IR expression tests](../../../tests/unit/expression/README.md).
