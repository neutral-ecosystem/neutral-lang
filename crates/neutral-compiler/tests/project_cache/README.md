<!-- SPDX-License-Identifier: Apache-2.0 -->

# Incremental project execution

These tests exercise the caller-owned compiler syntax cache, including stale
bytes, digest-collision simulation, bounded retention, failed runs, and source
accounting. Cross-package identity and reader equivalence is verified in the
project identity conformance suite. Cached syntax never bypasses fresh project
semantics, lowering, acceptance limits, or reader validation.
`composition.rs` exercises the separate successor cache's private digest/byte,
module and feature guards. Cross-package clean/cached IR and wire comparisons,
changed vocabulary defaults and request isolation live in the composition
contract suite; these are actual parser hits, not captured replay.
The private pipeline observer injects cancellation and processing errors at ten
phase boundaries, including final publication. Every failure preserves the prior
successful generation. This is not global allocator-failure testing.

Retention-specific checkpoints cover warm syntax copies, module-key ownership,
pending syntax retention and ordered insertion. Failures preserve the prior cache
generation and its parser hits. Original-byte retention is tested exactly at its
acceptance bound and one below/above; declining retention cannot change complete IR.

`composition_graph.rs` checks the private successor graph's exact SCC/work bounds,
pre-cancellation and iterative traversal of long import chains. The graph shares
syntax with the existing module scanner but does not retain old-profile public owners.
