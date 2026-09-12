<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral workspace crates

This directory owns Neutral's production libraries and binaries plus dedicated
verification/support packages. Each crate documents its ecosystem boundary in
its own README, inherits workspace package metadata, keeps test bodies under
its `tests/` tree, and may depend only on the graph allowed by `xtask`.

Most crates are libraries, so they intentionally do not expose a standalone
command. Verify one library with `cargo test --package <crate-name>`; use
`cargo xtask dev` for the complete repository development gate. The CLI and
probe READMEs document their runnable commands, and the benchmark README
documents the controlled performance command.
