<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 delta requirements

Status: accepted portable baseline

## Baseline and scope

v1 inherits v0.1 without copying it. `neu "0.1"` remains frozen; a v1 compiler
must not reinterpret it. v1 adds only project composition and language tooling
infrastructure. It assigns no Editor UX, Flow execution, command, provider,
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
| Conformance | Named fixtures/oracles, negative diagnostics, vectors, and Reader/Editor/Flow-boundary probes. | [Conformance](../conformance/README.md) |

The full stable identifier list, every diagnostic code, and exact fixture bytes
are release assets. They must be added before the stage that activates them and
all must be present before `v1.0.0`; see [the release plan](../PLAN.md).
