<!-- SPDX-License-Identifier: Apache-2.0 -->

# Checks

This directory owns read-only repository invariants. It supports the developer and CI automation layer, not production language behavior.

Modules: `repository.rs`, `dependencies.rs`, `traceability.rs`, and
`test_layout.rs`. `git_hygiene.rs` rejects force-added ignored generated files.
Checks fail closed and do not change fixtures, approval records, or Git history.
Command routing remains in [the crate entry point](../lib.rs).
