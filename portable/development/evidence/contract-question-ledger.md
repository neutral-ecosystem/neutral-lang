<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 contract-freeze question ledger

Status: resolved; approved 2026-08-26

This ledger classifies questions found during contract-freeze review. A blocking
question must be resolved by an accepted governing document and recorded in the
approved freeze manifest. Implementation-only questions may not silently change
frozen behavior. Deferred questions remain outside v0.

## Resolved questions

| ID | Question | Evidence | Approved resolution |
| --- | --- | --- | --- |
| CFQ-001 | Which exact architecture, requirements, choices, syntax, and decision revisions are accepted v0 authority? | The source set was marked `proposed`. | The repository owner accepted the coherent v0 source set and its recorded SHA-256 revisions. |
| CFQ-002 | Is the author-facing syntax guide part of accepted v0 behavior? | `proposed-syntax-guide.md` was editable. | The guide was promoted as the accepted v0 authoring guide. |
| CFQ-003 | What exact identity, fingerprint, digest/transcript, vocabulary-schema, and bundle-encoding versions govern v0? | Versions were unassigned. | The freeze assigns the initial `0.1.0` contract family. Required executable vectors remain Stage 2+ evidence. |
| CFQ-004 | What complete oracle and stable failure contract applies to each initial source fixture? | The initial inventory defines the fixture bytes and required oracle shape. | The inventory is approved; activated Stage 2 cases additionally have complete per-case oracle records. |
| CFQ-005 | Have Gates A–G been reviewed with no unresolved finding? | No approval record existed. | The repository owner approved the v0 baseline and freeze manifest on 2026-08-26. |

## Implementation-only questions

None recorded. New implementation choices must be added here before work begins
if they do not alter a frozen observable contract.

## Deliberately deferred questions

None recorded. Later-version features remain excluded unless a separate accepted
proposal changes the v0 scope.

## Closure rule

This ledger is closed because every recorded question has an approved resolution
reference and the freeze manifest records approval. New blocking questions reopen
the ledger and require a reviewed freeze-manifest update.
