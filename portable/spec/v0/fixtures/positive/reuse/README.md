<!-- SPDX-License-Identifier: Apache-2.0 -->

# Positive immutable-value reuse fixtures

These fixtures define accepted Stage 5.1 behavior for ordinary immutable-value
reuse. They cover declaration-order independence, transitive replacement,
nested contextual reuse, and the permitted outer-nullable widening. The
compiler lowers each occurrence to the source binding's final logical value and
retains the dependency only as provenance; identity references remain owned by
Stage 5.2.
