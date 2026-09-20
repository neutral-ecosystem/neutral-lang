<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 accepted decisions

Status: accepted portable baseline

The v1 delta is closed on these decisions:

1. v0.1 remains an independently selectable frozen profile.
2. One source unit owns one logical qualified module; paths are host concerns.
3. Imports use mandatory aliases; alias namespace is shared with vocabularies.
4. Import cycles use SCC processing; only semantic cycles are rejected.
5. Visibility is private-by-default; public signatures close over public types.
6. Roots are consumer views, excluded from capture/IR equality and project
   meaning.
7. Capture receives a complete host-neutral request and never performs I/O.
8. Multiple vocabulary locks are exact, data-only, and semantically stable.
9. `url`/`path` are inert values, never acquisition instructions.
10. Captured closure, logical project, derivation, and artifact identities are
    separate.
11. Authoring is a separately versioned data-only bridge; Flow remains external
    and owns mapper/execution behavior.

No decision here permits functions, control flow, mutation, secrets, packages,
re-exports, runtime effects, executable vocabularies, or Editor-specific source
semantics.
