<!-- SPDX-License-Identifier: Apache-2.0 -->

# 03 — Identity and authoring bridge

Status: accepted operational plan

## Stages 7–8

### Stage 7 — `v0.7.0 -> v0.8.0`

Implement and independently review the identity chain: captured closure,
logical project, derivation, and artifact. Canonical logical form uses stable
module/type/module-symbol identities rather than graph-local labels. Host paths,
aliases, source evidence, roots, and capture order do not change logical
project identity. Literal transcript and SHA-256 vectors are release assets.

The [frozen transcript contract](../specs/contracts/PROJECT-IDENTITY.md) and
[contract and integration review](evidence/stage7-contract-core.md) cover all four
bounded identity layers, validated-reader facts, reader-only probe reporting,
and independently reproduced literal/adversarial vectors.

Stage 7 also owns the deferred root-to-project-digest invariance and actual
incremental/cache-versus-clean equivalence assertions from Stage 6. These are
verified against actual caller-owned syntax cache execution: changed units,
fresh vocabulary/controls/source facts, stale-entry rejection, failed-run
recovery, and concurrent schedules. Immutable replay is not labelled caching;
persistent or final-artifact caches are not implemented.

Before `v0.8.0`, complete the newly scheduled vocabulary composition extension
gate in the [implementation checklist](07-IMPLEMENTATION-CHECKLIST.md).
[V1-VOC-005..011](../specs/contracts/CONSUMER-READINESS.md) add a reviewed,
versioned bundle schema for lists, nullable values, typed references, closed
defaults, bounded declarative restrictions, tagged variants, cross-vocabulary
dependencies, and omission/default origin rules. Carry all new facts through IR,
reader validation, view closure, and new identity vectors. Preserve old bundle
schemas, v0 behavior, and frozen identity profiles. Existing Stage 7 evidence
does not claim this extension is implemented.

### Stage 8 — `v0.8.0 -> v0.9.0`

Implement the separately versioned, data-only authoring profile: exact profile
discovery; static core/vocabulary descriptor catalogue; revision-bound project
overlay; closed editable graph model; deterministic source projection and
formatting; and authoring diagnostics. Connections are the only visual form of
ordinary reuse and identity-reference edges. The compiler, not the Editor,
remains the source authority.

Before `v0.9.0`, complete
[V1-CONS-001..010](../specs/contracts/CONSUMER-READINESS.md): project-neutral
operation/input/result/resource data examples, explicit prerequisite/output
connections, structured deferred conditions and symbolic handles, and versioned
capability/mapping-description/report interfaces. Extend authoring discovery and
projection for the composite vocabulary schema. Add precise public source
attribution, typed member selectors, explicit entry
discovery, self-contained contract facts, and producer/reader compatibility
negotiation. Scheduling, mapping execution,
condition evaluation, credentials, authorization, and backend behavior remain
external responsibilities; no executor or new computation syntax is introduced.

## Exit evidence

A generic Editor probe must build its palette from discovery, edit a bounded
project model, project source, compile it, reopen it, and no-op round trip
without a handwritten construct table or a private parser/AST dependency.

A separate standalone reader-only probe must inspect the consumer examples,
preserve public dependency closure, and exercise literal supported/unsupported
capability reports with safe source attribution. Prove presentation-only edits
preserve semantic identity, while semantic constraints/defaults change it.
Register these as reviewed fixture families, not completed evidence, until
schema freeze, implementation, hostile/limit testing, and integration pass.
