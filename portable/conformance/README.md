<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 conformance

Status: accepted release-train baseline

[manifest.toml](manifest.toml) is the activation inventory for the inherited
v0 suite and v1 delta suites. Each active v1 case must name exact bytes and an
expected result; no planned inventory entry is passing evidence.

The v1.0.0 gate requires the full inherited v0 corpus, all v1 fixture/oracle
families, reviewed canonical identity vectors, and independent Reader, generic
authoring, and project-neutral consumer-boundary probes. See [the plan](../PLAN.md) and
[testing rules](../development/04-TESTING-CONFORMANCE.md).

The scheduled [consumer-readiness requirements](../specs/contracts/CONSUMER-READINESS.md)
add composite vocabulary fixtures before `v0.8.0` and consumer-data/authoring
fixtures before `v0.9.0`. They are not active manifest entries or passing evidence
until their schemas, literal outcomes, and identity vectors have been reviewed.
The `composition-catalogue` suite covers standalone catalogue acceptance/errors
and legacy migration. The separate required `composition-contract` suite covers
the explicit successor capture/compiler/codec/reader/probe/identity pipeline.
Its 26 request cases, five baseline partitions, twelve identity variations and
eight rejection vectors retain their reviewed byte pins. Production comparisons,
independent interface framing and hostile/fault/boundary/clean-cache suites are
documented in the [extension evidence](../development/evidence/stage7-contract-core.md#composition-identity-and-registered-corpus-closure-08-10-2026).
Frozen contract prose records its original design checkpoint; current activation
authority is the manifest, review and freeze record. No source-header, fallback
or compiler CLI selection of the successor pipeline is implied.
Core acceptance and consumer interpretation failures must have separate oracles;
no backend execution or external product implementation is required.

The `authoring-foundation` suite registers 26 pinned requests/oracles under
[AUTHORING/1](../specs/contracts/AUTHORING.md). Its status is `frozen`, not
`required`: foundation contract/fixture completion does not establish bridge
implementation, public production comparisons, or the generic Editor probe.
Existing composition pins remain historical checkpoints and are not regenerated
when later-stage registry entries are appended. Authoring inputs have their own
byte pins verified through the active freeze's fixture-corpus section.

The `consumer-boundary` suite freezes 32 literal requests/oracles under
[Consumer/1](../specs/contracts/CONSUMER-PROFILE.md), with separately checked
contract/input/oracle byte pins. Selectors, entry discovery, attribution/privacy,
closure, compatibility, limits and cancellation have literal expected outcomes.
This is not production validation; graph cases are registered separately below
and public protocol implementation remains pending. Status is `frozen`, not `required`.

The `consumer-graphs` suite registers 31 literal cases, 13 file-backed source
inputs and one captured vocabulary under the [example convention](../specs/contracts/EXAMPLE-CONSUMER.md).
Core-positive inputs compile and independently read; four core-negative families
have literal NEU-COMP outcomes. Consumer report projections, capability/alternative
contexts, attribution/privacy and independent exact/one-over controls are pinned,
but their adapter/comparison is not implemented. This suite is `frozen` only.
The expression/evaluator/expansion semantic contracts are separately frozen and
byte-pinned together with successor compatibility and vectors; production activation remains pending.

The frozen `expression-composition-review` suite registers
[68 approved requests](../specs/fixtures/expressions/cases.json), 42 `.neu`
files, one vocabulary bundle and independent arithmetic/NHT/CBOR/matrix vectors
under the [frozen successor compatibility](../specs/contracts/EXPRESSION-PROFILE.md).
Written approval is recorded in the freeze manifest; candidate-named snapshot
IDs and embedded labels remain unchanged to preserve accepted bytes. The standalone
[oracle check](oracles/expressions/README.md) verifies data consistency only;
production comparisons and activation remain pending. No fixture was regenerated.
