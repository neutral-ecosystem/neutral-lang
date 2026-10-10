<!-- SPDX-License-Identifier: Apache-2.0 -->

# Trusted document encoder

This private directory projects independently validated documents into the
existing framed-CBOR artifact. `mod.rs` orders the projections; `logical.rs`
owns meaning, `companions.rs` owns source/provenance/derivation records,
`envelope.rs` owns producer metadata and integrity, and `frame.rs` assembles
the fixed directory and sections. `fields.rs` shares CBOR emission helpers.

`capabilities.rs` performs bounded capability inspection shared with the decoder.
Hostile reconstruction stays separate in `../decode.rs` and `../decoder.rs`;
encoding helpers must not replace independent decoder validation. Public callers
continue to use `neutral_encoding::encode`.
