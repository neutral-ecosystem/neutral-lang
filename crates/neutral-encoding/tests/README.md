<!-- SPDX-License-Identifier: Apache-2.0 -->

# Encoding tests

`expression/` checks frozen `/3` tuple vectors, complete round-trips, hostile
input, limits, cancellation, and atomic allocation failures. `decoder/` checks
the shared restricted-CBOR parser. `current/` checks public decoder limits,
safe error contracts, and literal rejection of historical project magic.
