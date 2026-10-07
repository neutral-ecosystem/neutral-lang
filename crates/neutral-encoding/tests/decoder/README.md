<!-- SPDX-License-Identifier: Apache-2.0 -->

# Restricted CBOR boundaries

These crate-local tests protect the untrusted lexical decoding layer before
logical artifact validation: integer widths, truncation, strings, duplicate
maps, limits, cancellation, and checked offsets. They complement full artifact
tests without changing the accepted wire schema.

Nested array/map/text/byte inputs exercise each fallible lexical reservation with
request-local synthetic faults and mid-parse cancellation. No partial lexical tree
is returned, and a fresh unchanged parse must still succeed. This does not replace
full schema/reader validation or establish system-wide out-of-memory behavior.

Run `cargo test --package neutral-encoding decoder::tests` or
`cargo xtask test all` for the complete repository suite.
