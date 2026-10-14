<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression transport tests

These owning-crate tests compare complete static, residual and template wire
literals, every frozen node/operator tuple, malformed tokens and independent wire
limits. They also sweep observed fallible reservations and cancellation and check
that unchanged valid bytes recover after failure.

The runtime-owned `fixtures/` literals are exact copies of the frozen complete
expression oracle bytes, separate from the optional portable specification tree.
