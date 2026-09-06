<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-suite

`neutral-test-suite` owns cross-package smoke, integration, system, and
conformance tests for Neutral.

It is non-published and has no production role. It combines public package
boundaries with the normative fixture/oracle corpus to verify complete vertical
slices, while package-local unit tests remain next to their implementation.

Its active tests cover the completed Stage 2–8 vertical paths and current
Stage 9 hardening:
source/scalar/record/list/reuse/reference/vocabulary compilation, typed reader
and probe traversal, external encoding and hostile decoding, frozen oracles,
determinism, structural limits, deterministic fuzz-style cases, the reference
formatter, host CLI, standalone probe, documentation, and traceability.
Formatter evidence spans the complete positive source corpus and checks exact
canonical layout, idempotence, logical/provenance preservation, deterministic
nonsemantic comment placement, and separation from source/artifact identity.
