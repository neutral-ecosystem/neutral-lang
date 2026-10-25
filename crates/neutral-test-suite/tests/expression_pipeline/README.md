<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete expression pipeline

These tests use the current exact project/semantic profiles also selected by the
CLI. Real captured source and vocabulary bytes pass through compilation,
complete artifact encoding and independent reader validation before assertions.
No ambient bindings, execution, backend adapters or acquisition are involved.

- `mod.rs`: shared capture/transport helpers and literal static, expanded and
  guarded dependency/public-view results.
- `consumer.rs`: explicitly supplied input/output generations, complete result
  validation, guarded demand, unavailable/failed reports and vocabulary restriction
  obligations that survive encoding.
- `consumer/generations.rs`: vocabulary-owned prerequisites, exact policy/result
  associations, malformed skipped generations, null/absence/unavailable/failure,
  explicit unavailable attribution and adapter/target/input revision invariance.
- `inspection.rs`: independently decoded probe/entry selection, root-invariant
  identity, complete public-contract parity and exact tuple/feature/missing-contract
  rejection without evaluation.
- `cache.rs`: real clean/warm/changed-unit syntax generations, imported templates,
  matrices, changed vocabulary defaults/restrictions and refreshed source evidence,
  each crossing independent encoding/reading. Consumer tests also check reports
  from cached artifacts without retaining binding context.
- `projection.rs`: literal nested if/match/short-circuit guard paths, repeated
  output-key occurrences, retained vocabulary obligations and private-origin
  redaction through the public accessors after independent decoding.
- `bounds.rs`: inclusive and positive one-over boundaries for 37 artifact-retained
  project, vocabulary, expression and expansion controls, with recovery.
- `fixtures/`: file-backed source and closed captured vocabularies, including
  `consumer.neu` / `consumer-domain.json` for operation/result/prerequisite data; imports and
  diamond bundles in the boundary test remain explicit request inputs.

Run `cargo xtask test integration` for these and other integration tests, or
`cargo xtask test all` for the whole suite. A direct focused run is
`cargo test --package neutral-test-suite expression_pipeline::`.

These tests exercise actual expression-cache equivalence; they do not replace the
registered source corpus, independent vectors, fuzz, mutation or soak campaigns.
