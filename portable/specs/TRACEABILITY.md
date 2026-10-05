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
| V1-MOD-001 — module/import grammar | [MODULE-GRAPH](contracts/MODULE-GRAPH.md), [SOURCE](contracts/SOURCE.md) | 01 | qualified-module and forbidden-form fixtures |
| V1-MOD-002 — one unit per module | [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md), [MODULE-GRAPH](contracts/MODULE-GRAPH.md) | 01 | Stage 2 duplicate-module oracle and Stage 3 graph fixtures |
| V1-MOD-003 — local aliases | [MODULE-GRAPH](contracts/MODULE-GRAPH.md) | 01 | alias-collision and required-alias fixtures |
| V1-MOD-004 — deterministic bounded graph | [MODULE-GRAPH](contracts/MODULE-GRAPH.md) | 01 | graph ordering and edge/per-module exact/one-over oracles |
| V1-MOD-005 — SCC behavior | [MODULE-GRAPH](contracts/MODULE-GRAPH.md) | 01 | valid cycle, disconnected member, and SCC exact/one-over oracles |
| V1-MOD-006 — invalid imports | [MODULE-GRAPH](contracts/MODULE-GRAPH.md) | 01 | missing/self/duplicate/forbidden-form negative oracles |
| V1-VIS-001..003, V1-XMOD-001..004 — public semantics | [PUBLIC-SEMANTICS](contracts/PUBLIC-SEMANTICS.md), [SOURCE](contracts/SOURCE.md), [MODULE-GRAPH](contracts/MODULE-GRAPH.md) | 04 | [Stage 4 fixtures/oracles](../conformance/manifest.toml), [public integration and validation](../development/evidence/stage4-validation.md), reader-only probe; import SCC accepted independently of semantic cycles |
| V1-VOC-001..004, V1-LOC-001..002 — vocabulary/location values | [VOCABULARY](contracts/VOCABULARY.md), [CAPTURE-REQUEST](contracts/CAPTURE-REQUEST.md) | 05 | [Stage 5 fixtures/oracles](../conformance/manifest.toml), inherited Stage 2 missing/extra/conflicting lock oracles, canonical-identity/public-type/inert-value tests, executable/unknown-member rejection, and malformed-location/no-acquisition checks |
| V1-IR-001..003, V1-VIEW-001..002, V1-API-001 — complete project IR, reader, and views | [PROJECT-IR](contracts/PROJECT-IR.md), [PROJECT](contracts/PROJECT.md) | 06 | [pinned complete/private/public/view/malformed/wire fixtures](../conformance/manifest.toml), [encoded integration review](../development/evidence/stage6-integration-validation.md), compiler-free standalone probe, complete round-trips, companion/resource/output bounds, public/vocabulary closure, immutable replay, capture-order/concurrent equivalence, and failure envelopes; complete project hash and actual syntax-cache assertions are verified separately in Stage 7 |
| V1-ID-001..004 — captured/logical/derivation/artifact identity | [PROJECT-IDENTITY](contracts/PROJECT-IDENTITY.md), [PROJECT](contracts/PROJECT.md) | 07 | [six pinned literal transcript/SHA-256 vectors](../conformance/manifest.toml), [contract/core review](../development/evidence/stage7-contract-core.md), capture/order/alias/evidence/root exclusions, complete private meaning, explicit context, and exact byte/node/depth/cancellation tests |
| V1-ID-005 — independent identity reproducibility and integration | [PROJECT-IDENTITY](contracts/PROJECT-IDENTITY.md) | 07 | [independent Python encoder and conformance](../../crates/neutral-test-suite/tests/project_identity/README.md), [actual cache execution tests](../../crates/neutral-compiler/tests/project_cache/README.md), and [integration review](../development/evidence/stage7-contract-core.md); literal/adversarial comparisons, typed reader facts, compiler-free probe/root invariance, exact source/profile/module/byte keys, stale/collision rejection, changed vocabulary/controls, complete source facts, and concurrent clean-versus-cached artifacts |
| Authoring, including presentation-only vocabulary metadata | [AUTHORING](contracts/AUTHORING.md), [VOCABULARY](contracts/VOCABULARY.md) | 08 | separate metadata input, catalogue/overlay/projection evidence, unchanged semantic IR/identity, and no-op Editor probe |
| V1-VOC-005..011 — scheduled composite vocabulary extension | [CONSUMER-READINESS](contracts/CONSUMER-READINESS.md), [composition proposal](contracts/VOCABULARY-COMPOSITION.md) | 07, before v0.8.0 | [standalone catalogue tests](../../crates/neutral-vocabulary/tests/composition.rs) cover schema/default/restriction/presence semantics, variants, dependencies, public closure, budgets and cancellation; old-schema compatibility passes; complete project/identity/wire/fixture activation remains pending; composite fields, defaults/restrictions, tagged variants, cross-bundle exact closure, omission/null/default origins, compiler/IR/wire/reader/probe agreement, public closure, exact/one-over limits, compatibility, and immutable/new identity vectors |
| V1-CONS-001..010 — scheduled consumer data and authoring boundary | [CONSUMER-READINESS](contracts/CONSUMER-READINESS.md), [AUTHORING](contracts/AUTHORING.md) | 08, before v0.9.0 | pending protocol/fixture freeze; generic data transport, precise safe attribution, typed member selectors, explicit entry discovery, self-contained contract facts, producer/reader negotiation, static graph and capability/report cases, external-context identity separation, and authoring round trips |
| Release assurance | [conformance manifest](../conformance/manifest.toml) | 04/05 | deterministic, hostile-input, limits, project-neutral consumer boundary, retained CI evidence |

The `v1.0.0` release is blocked if a v1 contract rule lacks an active fixture
family, expected outcome, owner, or public-boundary evidence.

The active Stage 3 graph family is pinned in the
[conformance manifest](../conformance/manifest.toml) and exercised by the
[core graph](../development/evidence/stage3-2-core-graph.md),
[public integration](../development/evidence/stage3-3-public-integration.md),
and [validation](../development/evidence/stage3-4-validation.md) evidence.
The active Stage 4 public-semantics family is pinned in the same manifest and
reviewed in [Stage 4 integration and validation](../development/evidence/stage4-validation.md).
