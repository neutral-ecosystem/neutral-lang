<!-- SPDX-License-Identifier: Apache-2.0 -->

# Restricted CBOR boundaries

These crate-local tests protect the untrusted lexical decoding layer before
logical artifact validation: integer widths, truncation, strings, duplicate
maps, limits, cancellation, and checked offsets. They complement full artifact
tests without changing the accepted wire schema.

Run `cargo test --package neutral-encoding decoder::tests` or
`cargo xtask test all` for the complete repository suite.
