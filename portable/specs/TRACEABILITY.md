<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 implementation traceability

Status: accepted release-train baseline

This index prevents the v1 delta from becoming a parser-only or
consumer-specific feature set. Each requirement group has one portable contract,
stage owner, and required public evidence. Exact requirement IDs and case paths
are added to the manifest when their stage activates.

| Requirement group | Governing contract | Stage owner | Required evidence |
| --- | --- | --- | --- |
| V1-BASE-001, V1-BASE-002 — profile compatibility | [SOURCE](contracts/SOURCE.md) | 00/01 | Stage 1 positive/negative/boundary/lookalike matrix and inherited v0.1 regression |
| V1-BASE-003 — diagnostic and limit foundation | [SOURCE](contracts/SOURCE.md), [PROJECT](contracts/PROJECT.md) | 01 | public profile family, shared limit constants, reader/CLI catalogue agreement |
| V1-BASE-004 — no-I/O dependency boundary | [PROJECT](contracts/PROJECT.md) | 01 | workspace dependency audit and captured-input compiler tests |
| V1-BASE-005 — v1 exclusions | [SOURCE](contracts/SOURCE.md), [accepted decisions](decisions/README.md) | 01 | function/effect/acquisition/product negative fixtures |
| V1-CAP-001 — request and controls | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md) | 01 | closed request fixture schema and version/profile failures |
| V1-CAP-002 — source/module agreement | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md), [SOURCE](contracts/SOURCE.md) | 01 | duplicate identity and profile/module mismatch oracles |
| V1-CAP-003 — complete supplied set | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md), [PROJECT](contracts/PROJECT.md) | 01 | complete and disconnected-unit capture oracles |
| V1-CAP-004 — locks and host mappings | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md), [VOCABULARY](contracts/VOCABULARY.md) | 01 | exact/missing/extra/conflicting lock and host mapping oracles |
| V1-CAP-005 — independent limits | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md) | 01 | exact/one-over capture limit evidence |
| V1-CAP-006 — immutable no-I/O capture | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md), [PROJECT](contracts/PROJECT.md) | 01 | dependency audit and no-resolver public-boundary evidence |
| Module graph and SCCs | [PROJECT](contracts/PROJECT.md), [SOURCE](contracts/SOURCE.md) | 01 | Stage 3 graph/SCC limits and negative diagnostics |
| Public semantics | [SOURCE](contracts/SOURCE.md) | 02 | visibility, closure, cross-module reuse/ref, semantic-cycle corpus |
| Vocabulary/location values | [VOCABULARY](contracts/VOCABULARY.md) | 02 | exact-lock and inert-value corpus |
| Project IR and reader | [PROJECT](contracts/PROJECT.md) | 02 | validated IR, public view, independent reader probe |
| Identity/artifacts | [PROJECT](contracts/PROJECT.md) | 03 | canonical transcript and SHA-256 vectors |
| Authoring | [AUTHORING](contracts/AUTHORING.md) | 03 | catalogue/overlay/projection/no-op Editor probe |
| Release assurance | [conformance manifest](../conformance/manifest.toml) | 04/05 | deterministic, hostile-input, limits, Flow-boundary, retained CI evidence |

The `v1.0.0` release is blocked if a v1 contract rule lacks an active fixture
family, expected outcome, owner, or public-boundary evidence.
