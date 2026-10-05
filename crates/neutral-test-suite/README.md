<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-suite

`neutral-test-suite` owns cross-package smoke, integration, system, and
conformance tests for Neutral.

It is non-published and has no production role. It combines public package
boundaries with the normative fixture/oracle corpus to verify complete vertical
slices, while package-local unit tests remain next to their implementation.

Its active tests cover source/scalar/record/list/reuse/reference/vocabulary
compilation paths, typed reader and probe traversal, external
encoding and hostile decoding, frozen oracles, determinism, structural limits,
deterministic fuzz-style cases, the reference formatter, host CLI, standalone
probe, documentation, and traceability.
Formatter evidence spans the complete positive source corpus and checks exact
canonical layout, idempotence, logical/provenance preservation, deterministic
nonsemantic comment placement, and separation from source/artifact identity.
Project identity tests pin complete literal transcripts and SHA-256 digests,
verify capture/logical/derivation/artifact exclusions, exact byte/node boundaries,
cancellation, canonical value equivalence, and artifact kind/format/selection.
Crate-owned vector copies remain runnable when the active portable is archived.

The [file-backed pipeline corpus](tests/source_pipeline/README.md) tests real
`.neu` programs, expected materialized values, rejected phases, source maps,
encoding/reader/public-view boundaries, cache updates and executable results.
It also protects inherited field provenance for reused default-bearing records.

## Command

Run the cross-package suite with:

```sh
cargo test --package neutral-test-suite
```

Use `cargo xtask test all` as the primary test command for the complete repository
test graph and doctests. The focused Cargo command above runs this crate only.
