<!-- SPDX-License-Identifier: Apache-2.0 -->

# Unit test ownership

`mod.rs` keeps shared scalar, vocabulary, type/value, nominal and language
primitive tests. `project_identity_framing.rs` tests the private bounded writer.
`expression_retention_inspection.rs` covers the common composition inspector
used by current expression projects. Expression raw-model and frozen identity
tests live in the top-level test files.

The former complete-document graph equality/accessor tests were removed with
those models; current `/3` identity partitions and frozen bytes are mapped in
the [IR test guide](../README.md). Run `cargo test -p neutral-ir`.
