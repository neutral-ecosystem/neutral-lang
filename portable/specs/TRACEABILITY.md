<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 implementation traceability

Status: accepted release-train baseline

This index prevents the v1 delta from becoming a parser-only or
consumer-specific feature set. Each requirement group has one portable contract,
stage owner, and required public evidence. Exact requirement IDs and case paths
are added to the manifest when their stage activates.

| Requirement group | Governing contract | Stage owner | Required evidence |
| --- | --- | --- | --- |
| Profile compatibility | [SOURCE](contracts/SOURCE.md) | 00/01 | v0.1 regression and explicit-profile matrix |
| Capture and modules | [PROJECT](contracts/PROJECT.md), [SOURCE](contracts/SOURCE.md) | 01 | complete request, no-I/O, graph/SCC, limits, negative diagnostics |
| Public semantics | [SOURCE](contracts/SOURCE.md) | 02 | visibility, closure, cross-module reuse/ref, semantic-cycle corpus |
| Vocabulary/location values | [VOCABULARY](contracts/VOCABULARY.md) | 02 | exact-lock and inert-value corpus |
| Project IR and reader | [PROJECT](contracts/PROJECT.md) | 02 | validated IR, public view, independent reader probe |
| Identity/artifacts | [PROJECT](contracts/PROJECT.md) | 03 | canonical transcript and SHA-256 vectors |
| Authoring | [AUTHORING](contracts/AUTHORING.md) | 03 | catalogue/overlay/projection/no-op Editor probe |
| Release assurance | [conformance manifest](../conformance/manifest.toml) | 04/05 | deterministic, hostile-input, limits, Flow-boundary, retained CI evidence |

The `v1.0.0` release is blocked if a v1 contract rule lacks an active fixture
family, expected outcome, owner, or public-boundary evidence.
