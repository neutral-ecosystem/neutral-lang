<!-- SPDX-License-Identifier: Apache-2.0 -->

# IR tests

Run `cargo xtask test all` for the full workspace and doctests. For this crate,
run `cargo nextest run --config-file config/nextest.toml --package neutral-ir`
and `cargo test --package neutral-ir --doc`.

[`unit`](unit/README.md) contains test bodies. The library's path-based test module
retains access to private implementation where needed; [`expression.rs`](expression.rs)
loads the public raw-expression tests. Production modules contain no test bodies.
Complete source-to-artifact and hostile-reader cases belong to their owning
compiler/reader/encoding or cross-crate test suites, not duplicate implementations here.
