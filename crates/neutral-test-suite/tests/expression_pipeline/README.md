<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete expression pipeline

These tests use explicit successor profiles without activating them as the CLI
default. Real captured source and vocabulary bytes pass through compilation,
complete artifact encoding and independent reader validation before assertions.
No ambient bindings, execution, backend adapters or acquisition are involved.

- `mod.rs`: shared capture/transport helpers and literal static, expanded and
  guarded dependency/public-view results.
- `consumer.rs`: explicitly supplied input/output generations, complete result
  validation, guarded demand, unavailable/failed reports and vocabulary restriction
  obligations that survive encoding.
- `inspection.rs`: independently decoded probe/entry selection, root-invariant
  identity and exact feature rejection without evaluation.
- `projection.rs`: literal nested if/match/short-circuit guard paths, repeated
  output-key occurrences, retained vocabulary obligations and private-origin
  redaction through the public accessors after independent decoding.
- `bounds.rs`: inclusive and positive one-over boundaries for 37 artifact-retained
  project, vocabulary, expression and expansion controls, with recovery.
- `fixtures/`: file-backed source and a closed captured vocabulary; imports and
  diamond bundles in the boundary test remain explicit request inputs.

Run `cargo xtask test integration` for these and other integration tests, or
`cargo xtask test all` for the whole suite. A direct focused run is
`cargo test --package neutral-test-suite expression_pipeline::`.

These tests do not establish actual expression-cache equivalence or replace the
registered source corpus, independent vectors, fuzz, mutation or soak campaigns.
