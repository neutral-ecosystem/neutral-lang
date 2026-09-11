<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 threat-model review

Review date: 2026-09-06. Owner: maintainer.

Protected assets are exact source and vocabulary identity, authoritative typed
IR, source maps, provenance, derivation facts, host files, output streams, and
bounded CPU/memory availability. Untrusted boundaries are source bytes,
vocabulary JSON, external framed IR, CLI paths and streams, and probe traversal.

| Threat | Control and evidence | Residual state |
| --- | --- | --- |
| Parser/decoder resource exhaustion | Explicit byte, depth, item, field, declaration, text, number, and traversal ceilings; at/over boundary tests; five clean 900-second coverage-guided campaigns | Pass within configured ceilings |
| Stack exhaustion from recursive input | Frozen nesting limits and iterative dependency traversal | No known reproducer |
| Malformed input yields partial authoritative IR | Result enums separate success/failure; reader validates complete graphs; CLI atomic publication | Pass in active hostile suites |
| Source/vocabulary substitution | Typed SHA-256 digests and exact captured lock validation | Pass |
| Cache poisoning or stale source facts | No semantic caches; immutable request-owned artifacts; concurrent isolation test | Pass |
| Cross-request leakage | No mutable global request state; exact repeated/concurrent outcome tests | Pass |
| Executable vocabulary or plugin injection | Closed inert JSON schema; executable members rejected; no loading/network lookup | Pass |
| Artifact corruption/unknown capability | Fixed frame, integrity digests, closed schemas, compatibility validation | Pass |
| Path disclosure or partial file overwrite | Path-safe diagnostics, synchronized temporary file, atomic commit, overwrite policy | Pass on declared host matrix |
| Cancellation publishes work | Capture and every compiler handoff observe one shared token; decoder and CLI cancellation tests | Pass |

Denial-of-service conclusions remain bounded to configured ceilings and tested
host profiles. They are not a claim about unlimited inputs or every filesystem.
