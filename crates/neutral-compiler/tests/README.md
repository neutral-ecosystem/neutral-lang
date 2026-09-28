<!-- SPDX-License-Identifier: Apache-2.0 -->

# Compiler tests

This directory owns private frontend, semantic, and compiler-boundary unit
modules. Cross-package behavior remains the responsibility of
`neutral-test-suite`.

`module_graph/` tests the pure captured-source graph builder, import syntax,
diagnostic order, exact graph limits, cancellation, and iterative SCC ordering.
