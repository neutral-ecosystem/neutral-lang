<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-reader

`neutral-reader` validates and exposes immutable views of current Neutral
expression projects. `ValidatedExpressionProject` checks the `/3` model,
companions, semantic profiles, dependencies, canonical identity, and producer
resource bounds before publishing authority. It is independent of compiler
private parser state and decoded projects enter through the current encoding
reader.

The expression reader provides bounded public entry selection, type/dependency
inspection, and redacted public projection. Consumer policy is explicit and
data-only: `ExpressionConsumerBoundary` validates public report contracts,
registered associations and supplied bindings before pure semantic
evaluation. It does not schedule work, authorize effects or invoke external
backends. See [current expression consumers](src/consumer/README.md).

The shared deterministic language-profile catalogue is supplied by
`neutral-core`. Composition inspection APIs are separate validated catalogue
and resolved-scope readers; they expose contracts, closed-value materialization,
reference-type facts and public binding edges. They do not accept complete
captured composition projects or derive a second whole-project authority.
See [composition catalogue inspection](src/composition/README.md).

Run the crate tests with:

```sh
cargo test -p neutral-reader
```
