<!-- SPDX-License-Identifier: Apache-2.0 -->

# Reviewed fuzz seeds

Tracked seeds bootstrap coverage-guided campaigns at current public schemas.
They are immutable examples, not mutable campaign corpora or conformance
oracles. `composition-source.neu` is the current `neu "1.0"` expression source;
valid wire mutations are generated at runtime by the harness from that source.
Mutable discoveries remain ignored under `fuzz/corpus/`.

`expression_support::SOURCE_SEEDS` explicitly registers the runtime-owned sources:

| Seed | Paths reached |
| --- | --- |
| `composition-source.neu` | Materialized scalar baseline |
| `expression-consumer.neu` | Deferred input/output, typed selectors, skipped/selected demand and ordinary prerequisites |
| `expression-expansion.neu` | Definition-scoped templates, defaults, finite matrices, filters and exclusions |
| `expression-guards.neu` | Nested `if`/`match`/short-circuit guards and repeated output sites |

Source/formatter targets preserve the two required header lines while mutating a
selected source body. IR/probe targets generate a valid complete artifact for
each registered source once, then select a seed and mutate from a machine-word
byte offset anywhere in the frame. `cargo xtask fuzz smoke` executes the same
support code without libFuzzer, verifies every registered seed compiles and
independently decodes, and checks literal consumer outcomes. Seeds never import
portable-plan files or acquire vocabulary/host inputs.
