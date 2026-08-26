<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-suite

`neutral-test-suite` owns cross-package smoke, integration, system, and
conformance tests for Neutral.

It is non-published and has no production role. It combines public package
boundaries with the normative fixture/oracle corpus to verify complete vertical
slices, while package-local unit tests remain next to their implementation.

Its active Stage 2 tests cover the minimal compiler-to-reader-to-probe path,
frozen oracles, formatting invariance, repeated/concurrent determinism,
malformed input, limits, and bounded mutation smoke.
