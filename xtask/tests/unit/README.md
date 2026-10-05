<!-- SPDX-License-Identifier: Apache-2.0 -->

# Automation unit tests

This private module verifies command parsing, manifests, dependency boundaries,
traceability, documentation generation, and safe result handling.
`git_hygiene.rs` covers generated-state ignores, intentional fuzz/release inputs,
and force-added ignored files using an isolated temporary Git index.
