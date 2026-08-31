<!-- SPDX-License-Identifier: Apache-2.0 -->

# Negative immutable-value reuse fixtures

These fixtures freeze Stage 5.1 rejection boundaries for unresolved names,
wrong declaration kinds, direct and indirect dependency cycles, and invariant
container types. The traversal fixture is executed with a captured low node
limit by the test suite. Rejected reuse never produces logical IR.
