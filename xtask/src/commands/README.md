<!-- SPDX-License-Identifier: Apache-2.0 -->

# Commands

This directory owns task composition and command-specific operations. It supports the developer and CI automation layer, not production language behavior.

Modules: `development.rs`, `documentation.rs`, `testing.rs`, `analysis.rs`,
`quality.rs`, `distribution.rs`, and `portable.rs`. `test_execution.rs` owns
configured Nextest/Cargo backend selection, exact test inventory counts, and
runner arguments shared by coverage and mutation. `cargo xtask test all` is the
primary test entry point; it includes Cargo doctests because Nextest runs only
test binaries. Fuzz campaigns and benchmarks retain their dedicated runners.
Command routing remains in [the crate entry point](../lib.rs); configuration
and production contracts remain separate. All commands use the shared runtime
reporter and preserve the same error and color controls.
