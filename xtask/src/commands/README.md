<!-- SPDX-License-Identifier: Apache-2.0 -->

# Commands

This directory owns task composition and command-specific operations. It supports the developer and CI automation layer, not production language behavior.

Modules: `development.rs`, `documentation.rs`, `testing.rs`, `analysis.rs`,
`quality.rs`, `distribution.rs`, and `portable.rs`. `test_execution.rs` owns
configured nextest/Cargo backend selection and exact test inventory counts.
Command routing remains in [the crate entry point](../lib.rs); configuration
and production contracts remain separate. All commands use the shared runtime
reporter and preserve the same error and color controls.
