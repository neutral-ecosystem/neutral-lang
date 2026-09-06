<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-probe

`neutral-probe` is the reader-only inspection library and standalone probe
binary for Neutral artifacts.

Its runtime path depends only on public `neutral-core`, `neutral-encoding`, and
`neutral-reader` contracts. Its test-only artifact builder uses public
`neutral-ir` constructors. Its purpose is to prove that encoded artifacts can be
decoded and inspected without linking `neutral-compiler`; it must not import
private parser/semantic models, capture logic, or a filesystem resolver. It
reports observations, not application-specific meaning.

The active probe implements deterministic binding, nominal-record, exact
vocabulary-contract, and qualified-schema summaries plus one consumer-owned
diagnostic mapped through the public reader source map. Its traversal remains
generic and contains no `Fixture`-specific interpretation. The standalone binary
accepts one encoded artifact path, validates it under the hard decoder limits,
and emits deterministic `[info]` observations or one bounded `[error]`.

```console
cargo run --package neutral-probe -- path/to/artifact.nir
```
