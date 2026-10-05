<!-- SPDX-License-Identifier: Apache-2.0 -->

# Incremental project execution

These tests exercise the caller-owned compiler syntax cache, including stale
bytes, digest-collision simulation, bounded retention, failed runs, and source
accounting. Cross-package identity and reader equivalence is verified in the
project identity conformance suite. Cached syntax never bypasses fresh project
semantics, lowering, acceptance limits, or reader validation.
