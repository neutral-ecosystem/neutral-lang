<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 delta requirements

Status: accepted portable baseline

## Baseline and scope

v1 inherits v0.1 without copying it. `neu "0.1"` remains frozen; a v1 compiler
must not reinterpret it. v1 adds only project composition and language tooling
infrastructure. It assigns no authoring-client UX, consumer execution, command, provider,
secret, authorization, acquisition, or runtime-effect meaning.

## Required outcomes

1. Compile one complete bounded captured project into validated project-level
   Neutral IR without external I/O after capture.
2. Support one source unit per logical module, explicit aliased imports,
   deterministic graph processing, and import SCCs.
3. Make declarations private by default; allow only public, transitively
   reachable types in public signatures and public targets for exposed refs.
4. Support multiple exactly locked, data-only vocabularies and inert `url` and
   `path` scalar values.
5. Publish validated-reader views separately from complete project IR; roots
   select views and never affect capture or logical equality.
6. Separate captured-closure, logical-project, derivation, and artifact
   identities under bounded canonicalization and versioned SHA-256 transcripts.
7. Publish a separately versioned, data-only authoring profile capable of
   dynamic catalogue discovery and deterministic source projection.
8. Before `v0.8.0`, complete a reviewed, newly versioned vocabulary schema for
   composite field types, tagged variants, cross-vocabulary public types, closed
   defaults, origin distinctions, and bounded declarative restrictions,
   preserving old schemas and identity profiles.
9. Before `v0.9.0`, expose project-neutral typed operation/input/result/resource,
   condition/output connection, capability/mapping-description, and report data
   through independent reader and authoring boundaries with precise attribution,
   typed selectors, explicit entry discovery, self-contained contract facts, and
   tested reader compatibility, without core execution.

## Requirement groups

| Group | Mandatory behavior | Contract |
| --- | --- | --- |
| Base and compatibility | Preserve v0.1; select profiles explicitly; map each v0 extension to a v1 delta. | [Source](contracts/SOURCE.md) |
| Capture and limits | Host-completed request; complete supplied closure; exact locks; no resolver callback/I/O; independent bounds. | [Project](contracts/PROJECT.md) |
| Modules and visibility | Logical modules; aliases; SCCs; no wildcard/relative imports or re-exports; private-by-default public closure. | [Source](contracts/SOURCE.md) |
| Semantics and IR | Cross-module reuse/ref validation; complete project IR; reader; source maps/provenance; separate views. | [Project](contracts/PROJECT.md) |
| Vocabulary | Exact, aliased, data-only semantic locks; public vocabulary type rules; inert location values. | [Vocabulary](contracts/VOCABULARY.md) |
| Identity and artifacts | Canonical logical form; identity chain; deterministic outputs; no host-mapping or root influence on meaning. | [Project](contracts/PROJECT.md) |
| Authoring | Exact authoring profile; static catalogue plus project overlay; closed editable model; compiler authority. | [Authoring](contracts/AUTHORING.md) |
| Vocabulary composition | Reviewed composite fields, semantic defaults/restrictions, independent reader validation, compatibility, and identity vectors. | [Consumer readiness](contracts/CONSUMER-READINESS.md) |
| Consumer data boundary | Project-neutral data conventions, capability/report envelopes, safe source attribution, and external interpretation boundary. | [Consumer readiness](contracts/CONSUMER-READINESS.md) |
| Conformance | Named fixtures/oracles, negative diagnostics, vectors, and independent reader/authoring/consumer-boundary probes. | [Conformance](../conformance/README.md) |

The full stable identifier list, every diagnostic code, and exact fixture bytes
are release assets. They must be added before the stage that activates them and
all must be present before `v1.0.0`; see [the release plan](../PLAN.md).

## Stage 2 frozen capture identifiers

- `V1-CAP-001`: closed, versioned `CapturedProjectRequest` and processing
  controls.
- `V1-CAP-002`: exact profile/module header agreement and unique logical source
  and module identities.
- `V1-CAP-003`: complete supplied source-set capture, including disconnected
  units.
- `V1-CAP-004`: exact vocabulary-lock coverage and host-mapping separation.
- `V1-CAP-005`: independent non-zero structural limits with exact/one-over
  behavior.
- `V1-CAP-006`: immutable capture with no resolver, callback, or external I/O.

Their normative envelope and failure catalogue are in the
[captured project request contract](contracts/CAPTURE-REQUEST.md). Stage 2.1
freezes these requirements and fixtures without claiming their implementation.

## Stage 3 frozen module-graph identifiers

- `V1-MOD-001`: exact v1 profile and qualified module/import grammar.
- `V1-MOD-002`: one captured source unit per logical module, with exact
  request/header agreement.
- `V1-MOD-003`: required import aliases in the shared local alias namespace.
- `V1-MOD-004`: deterministic bounded graph construction, canonical ordering,
  and source-mapped graph diagnostics.
- `V1-MOD-005`: valid import SCCs with bounded membership and deterministic
  dependency-first condensation.
- `V1-MOD-006`: missing, self, duplicate, wildcard, relative, implicit, and
  re-export import failures without acquisition.

The normative grammar, graph behavior, limits, and `NEU-MOD` diagnostic
catalogue are in the [module graph contract](contracts/MODULE-GRAPH.md).
The registered semantic-cycle fixture also reserves Stage 4 `V1-XMOD-004`;
Stage 3 must accept its import SCC, not claim semantic evaluation.

## Scheduled `v0.8.0` and `v0.9.0` requirement identifiers

These are scheduled release requirements, not claims of frozen schemas or
implementation. [Consumer readiness](contracts/CONSUMER-READINESS.md) owns their
scope; schema versions, diagnostic registries, exact fixtures/oracles, and
identity vectors must be reviewed before activation.

- `V1-VOC-005`: bounded source-aligned list/nullable/typed-reference field composition.
- `V1-VOC-006`: closed semantic defaults and finite-choice/numeric/length restrictions.
- `V1-VOC-007`: explicit schema compatibility, exact locks, complete IR contract facts,
  public closure, and independent reader revalidation.
- `V1-VOC-008`: reviewed identity forms/vectors without reinterpreting frozen profiles.
- `V1-VOC-009`: closed tagged variants and heterogeneous typed collections.
- `V1-VOC-010`: explicit cross-vocabulary public types and bounded exact transitive locks.
- `V1-VOC-011`: required/omitted/null/default distinctions and safe value-origin facts.
- `V1-CONS-001`: portable operation/input/result/resource and dependency/output data.
- `V1-CONS-002`: structured condition/deferred-input/symbolic-handle representation,
  with evaluation and execution explicitly outside core.
- `V1-CONS-003`: versioned capability/mapping-description/report boundary and external
  plan-context identity separation; no adapter implementation or secret payloads.
- `V1-CONS-004`: reader-only public interpretive closure and safe diagnostic attribution.
- `V1-CONS-005`: generic authoring support for composition, restrictions, and connections.
- `V1-CONS-006`: bounded field/element source attribution and typed related diagnostics.
- `V1-CONS-007`: stable typed member selectors without runtime evaluation.
- `V1-CONS-008`: explicit typed public entry discovery and post-compilation selection.
- `V1-CONS-009`: self-contained compiler-independent contract discovery from artifacts/views.
- `V1-CONS-010`: exact producer/reader feature negotiation and compatibility matrix.
