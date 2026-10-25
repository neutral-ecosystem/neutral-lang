<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-probe

`neutral-probe` is the reader-only inspection library and standalone probe
binary for complete Neutral expression-project artifacts (`NEUP3`). It depends
on the public `neutral-core`, `neutral-encoding`, and `neutral-reader` contracts;
its tests use public `neutral-ir` constructors. It does not link the compiler or
resolve source files.

The standalone command validates the complete artifact before exposing counts,
typed logical/interface identities, exact semantic profiles and features, and
public root names. Source IDs, spans, private declaration names, and raw private
values are omitted. Without `--root`, it lists every public declaration and
template owner. Repeated `--root module::name` selects public owners, and roots
are validated before output is written. Historical and unknown magic fail
closed without a decoder fallback. Artifact reads obey the hard byte cap.

```console
cargo run --package neutral-probe -- --json --root example::api target/project.nir
```

The library's `inspect_expression_encoded` is the typed consumer API. It
requires exact public entry contracts and features; the standalone catalogue
does not invent an entry contract. `render_standalone_expression` returns a
fallible, cancellation-aware JSON document without exposing partial output.

Verify the reader-only library and binary boundary with:

```sh
cargo test --package neutral-probe
```
