<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-suite

`neutral-test-suite` owns cross-package smoke, integration, system, and
conformance tests for Neutral.

It is non-published and has no production role. It combines public package
boundaries with the normative fixture/oracle corpus to verify complete vertical
slices, while package-local unit tests remain next to their implementation.

Its active tests cover the minimal compiler-to-reader-to-probe path and Stage 3
Slices 3.1–3.2: identifier/comment boundaries, bounded strings, Booleans, typed
reader traversal, safe probe rendering, frozen oracles, determinism, malformed
input, limits, future syntax exclusion, and bounded mutation smoke.
