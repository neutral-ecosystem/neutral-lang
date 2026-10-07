<!-- SPDX-License-Identifier: Apache-2.0 -->

# Reviewed fuzz seeds

Tracked seeds bootstrap coverage-guided campaigns at current public schemas.
They are immutable examples, not mutable campaign corpora or conformance
oracles. Mutable discoveries remain ignored under `fuzz/corpus/`.
`composition-source.neu` contains both source records and a tagged variant with
nested defaults, nullable references, ordinary reuse and heterogeneous values.
Successor source/wire/probe harnesses compile it explicitly; it grants no host
execution or acquisition authority.
`composition-diamond.neu` and `vocabulary/diamond-*.json` form a public typed
reference graph over an exact four-bundle dependency diamond. They seed bounded
transitive mutation; they do not modify frozen portable fixtures or identity vectors.
