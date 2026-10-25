<!-- SPDX-License-Identifier: Apache-2.0 -->

# Reviewed fuzz seeds

Tracked seeds bootstrap coverage-guided campaigns at current public schemas.
They are immutable examples, not mutable campaign corpora or conformance
oracles. `composition-source.neu` is the current `neu "1.0"` expression source;
valid wire mutations are generated at runtime by the harness from that source.
Mutable discoveries remain ignored under `fuzz/corpus/`.
