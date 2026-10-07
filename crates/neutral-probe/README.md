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

The active probe implements deterministic logical/identity metadata, source-map,
derivation, resource, binding, nominal-record, exact vocabulary-contract,
qualified-schema, final-value, and provenance summaries plus one consumer-owned
diagnostic mapped through the public reader source map. Its traversal remains
generic and contains no fixture-specific interpretation. The standalone binary
accepts one encoded artifact path, validates it under the hard decoder limits,
and emits the same shared-renderer `[info]` observations as the in-process
library, or one bounded `[error]`. With `--json`, stdout is a JSON document
without log prefixes so it can be parsed by other tools.
The binary also detects complete `NIR-PROJECT-CBOR/1` and successor
`NIR-PROJECT-CBOR/2` artifacts. It validates all
modules, private declarations, and companions before exposing complete counts
and a redacted public view. No private identities, source IDs, spans, or raw
provenance appear in that projection. An omitted selection includes all public
exports; `--root` can be repeated to select a smaller public dependency closure.
Selection never changes complete IR. Artifact file reads obey the hard byte cap.
Project summaries also expose the frozen identity profile and typed complete
logical digest, computed through the validated reader without compiler linkage.
This digest includes private/disconnected meaning but publishes no transcript or
private provenance; it is distinct from the public-interface fingerprint and
remains unchanged for omitted, empty, or explicit public-root selections.

```console
cargo run --package neutral-probe -- path/to/artifact.nir
```

For a formatted JSON inspection view of the decoded artifact, run:

```console
cargo run --package neutral-probe -- --json path/to/artifact.nir
```

For a complete project artifact produced by `neutral_encoding::project`, run:

```console
cargo run --package neutral-probe -- --json --root example::api target/project.nir
```

The single-file compiler CLI does not yet produce complete-project artifacts.
The library's `inspect_project_encoded` accepts explicit consumer limits and
optional selections; `Some(&[])` yields an empty view, while `None` selects all
public exports. Its typed failure contains either a bounded decode error or a
reader view or identity-construction error and no partial summary.

`composition::inspect_composition_encoded` provides the same explicit consumer
limits and root selection for successor artifacts. Its public projection exposes
both-origin record/variant contracts, defaults/restrictions, reference paths and
targets, and safe occurrence facts. All alternative types and transitive
vocabulary dependencies are retained, not just the selected payload. Source IDs,
spans and private implementation provenance are redacted; canonical public
vocabulary default owners remain without captured-byte spans. Complete logical
identity is independent of selection. The same `--json`/`--root` commands above
work for these library-produced artifacts without compiler linkage.

The JSON view is a readable, versioned projection of the probe summary. Its
field names, value shapes, and text labels are shared through
`neutral_probe::inspection_schema`, so consumers and both renderers use one
definition. Successor contract/value text lists are inspection presentation,
not a JSON IR transport. The `.nir` file remains the canonical binary NIR-CBOR artifact;
JSON whitespace does not change its encoding or identity.

Verify the reader-only library and binary boundary with:

```sh
cargo test --package neutral-probe
```

Use `cargo xtask test all` for complete repository validation, including doctests.
