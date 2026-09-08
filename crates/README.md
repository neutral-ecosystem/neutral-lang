<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral workspace crates

This directory owns Neutral's production libraries and binaries plus dedicated
verification/support packages. Each crate documents its ecosystem boundary in
its own README, inherits workspace package metadata, keeps test bodies under
its `tests/` tree, and may depend only on the graph allowed by `xtask`.
