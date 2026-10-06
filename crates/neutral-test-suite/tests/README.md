<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package test modules

This directory owns executable smoke, integration, system, conformance,
property, security, deterministic fuzz-style, and hardening evidence.

`module_graph/` validates the captured module/import graph against its pinned corpus
and runs adversarial graph-order, limit, and exclusion checks.

`project_identity/` owns literal transcript/digest vectors and identity-layer
equivalence, exclusion, context, cancellation, and boundary regressions.

`composition_contract/` owns frozen successor-profile input/vector copies and a
test-only independent oracle. It tests byte integrity, literal NHT framing,
identity partitions/rejections and existing catalogue validation, not activation
of future compiler/codec support. Runtime tests never read the portable plan.

`source_pipeline/` adds real `.neu` programs with explicit value and rejection
oracles, complete project artifacts, independent reader/probe results, changed-file
cache checks, and actual CLI/probe executable tests. Run
`cargo test --package neutral-test-suite source_pipeline::` for this corpus.
