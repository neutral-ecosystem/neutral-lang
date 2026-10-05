<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 contracts

Status: accepted portable baseline

These contracts are one coherent v1 delta. They do not recreate the frozen v0
rules for values, records, lists, source literals, or v0 vocabulary encoding.

- [SOURCE.md](SOURCE.md): module syntax, imports, visibility, and cross-module
  resolution.
- [PROJECT.md](PROJECT.md): host-neutral capture, IR, identities, reader, and
  views.
- [CAPTURE-REQUEST.md](CAPTURE-REQUEST.md): exact Stage 2 request envelope,
  controls, limits, identities, host boundary, and fail-closed outcomes.
- [MODULE-GRAPH.md](MODULE-GRAPH.md): Stage 3 module grammar, import edges,
  aliases, SCC ordering, limits, and graph diagnostics.
- [PUBLIC-SEMANTICS.md](PUBLIC-SEMANTICS.md): Stage 4 visibility, public type
  closure, qualified resolution, semantic cycles, and identity edges.
- [PROJECT-IR.md](PROJECT-IR.md): Stage 6 complete typed project publication,
  companions, resources, independent reader validation, and public view closure.
- [PROJECT-IDENTITY.md](PROJECT-IDENTITY.md): bounded canonical logical form,
  domain-separated captured/logical/derivation/artifact transcripts and vectors.
- [VOCABULARY.md](VOCABULARY.md): exact data-only vocabulary locks and authoring
  metadata boundary.
- [AUTHORING.md](AUTHORING.md): the separately versioned Editor bridge.
- [CONSUMER-READINESS.md](CONSUMER-READINESS.md): scheduled `v0.8.0` composite
  vocabulary schema and `v0.9.0` project-neutral reader/authoring data boundaries;
  schema and fixture review must precede activation.
- [v1-checklist.md](v1-checklist.md): implementation and conformance completion
  checklist, kept separate from design acceptance.
