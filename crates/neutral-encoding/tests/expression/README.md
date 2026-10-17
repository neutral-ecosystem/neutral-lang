<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression transport tests

These owning-crate tests compare complete static, residual and template wire
literals, every frozen node/operator tuple, malformed tokens and independent wire
limits. They also sweep observed fallible reservations and cancellation and check
that unchanged valid bytes recover after failure.

`qualification.rs` adds hand-encoded node/operator comparisons, all 45 producer
and caller positive controls, literal retained-count boundaries, and inclusive
work/storage thresholds on both sides of the trust boundary. The file-backed
`neutral-test-suite` expression pipeline exercises 37 positive retention controls
with multiple modules, a diamond vocabulary closure and multi-axis templates.
Compiler expansion visitation/depth and evaluator output controls belong to their
execution phase: a decoder must not invent filtered-away compilation history.

The runtime-owned `fixtures/` literals are exact copies of the frozen complete
expression oracle bytes, separate from the optional portable specification tree.
