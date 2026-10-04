<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete project fixtures

This contract-gate corpus owns complete public/private/disconnected project
requests and reviewed public/private view and malformed-IR mutation descriptors.
The request schema reuses the frozen capture envelope; descriptors refer only
to adjacent reviewed fixtures, never host acquisition. Expected outcomes are
pinned in the [conformance manifest](../../../conformance/manifest.toml).

Crate-owned copies under `neutral-test-suite/tests/project_ir` remain executable
after this portable plan is archived. Run
`cargo test --package neutral-test-suite project_ir` to exercise their reader,
value/default, source/provenance, bounds, cancellation, and view-closure checks.
The encoded descriptor also pins complete-project wire mutation classifications.
Owning codec tests exercise truncation, closed tuples, bounds, and companion
forgeries; compiler-free probe tests verify executable inspection and redaction.
