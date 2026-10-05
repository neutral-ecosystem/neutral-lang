<!-- SPDX-License-Identifier: Apache-2.0 -->

# Strict vocabulary JSON

These crate-local tests protect exact Unicode escape decoding, malformed syntax,
duplicate keys, and decoded allocation limits before vocabulary schema
validation. They ensure hostile JSON cannot publish an accepted intermediate tree.

Run `cargo test --package neutral-vocabulary json::tests` or
`cargo xtask test all` for the complete repository suite.
