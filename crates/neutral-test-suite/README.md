<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-suite

`neutral-test-suite` owns cross-package smoke, integration, system, and
conformance tests for Neutral.

It is non-published and has no production role. It combines public package
boundaries with the normative fixture/oracle corpus to verify complete vertical
slices, while package-local unit tests remain next to their implementation.

Its active tests cover the minimal compiler-to-reader-to-probe path and Stage 3
Slice 3.1 identifier/comment boundaries: frozen oracles, formatting and comment
invariance, repeated/concurrent determinism, malformed input, limits, future
syntax exclusion, and bounded mutation smoke.
