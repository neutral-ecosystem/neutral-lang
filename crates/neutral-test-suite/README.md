<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-suite

`neutral-test-suite` owns cross-package smoke, integration, system, and
conformance tests for Neutral.

It is non-published and has no production role. It combines public package
boundaries with the normative fixture/oracle corpus to verify complete vertical
slices, while package-local unit tests remain next to their implementation.

Its active tests cover the minimal compiler-to-reader-to-probe path, all
Stage 3 slices, and Stage 4 Slices 4.1–4.2: identifier/comment boundaries,
exact numbers, bounded strings, Booleans, nullable scalars, explicit null,
nominal records, contextual field validation, recursive record values, closed
defaults, omission materialization, field provenance, typed reader traversal,
safe probe rendering, frozen oracles, determinism, malformed input, limits,
future syntax exclusion, and bounded mutation smoke.
