<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete project IR, reader, and view contract

Status: accepted Stage 6 contract and core boundary

This refines [PROJECT](PROJECT.md), [PUBLIC-SEMANTICS](PUBLIC-SEMANTICS.md),
[VOCABULARY](VOCABULARY.md), and the inherited closed-default/value rules.
Package release numbers are not schema versions.

## Complete publication — V1-IR-001, V1-IR-002

`neutral.project-ir/1` retains every supplied module, including disconnected and
private-only units. Modules and declarations are in exact logical identity
order. Imports retain logical targets, not acquisition hints or aliases.
Every binding has a resolved type and a fully materialized contextual value;
every record retains canonical field types and all validated closed defaults,
including unused defaults. Ordinary reuse is materialized in dependency-first
order. `Ref<T>` retains only its typed module-symbol identity and never embeds
or evaluates its target. Cycles in ordinary reuse or embedded record types fail;
reference and import SCCs are not evaluation cycles.

Unknown, duplicate, missing required, incompatible, and non-closed default
fields fail. Closed defaults cannot read bindings or contain references.
Lists remain ordered and invariant; nullable values require explicit nullable
types. Numbers use the existing exact normalization contract, never floats.
`url` and `path` remain distinct inert decoded text.

Complete locked vocabulary schemas retain identity, exact semantic revision,
visibility, and canonical fields. Source aliases and authoring metadata are
not logical IR. Public vocabularies include the interpretation schemas needed
by their public nominal types, not only a name catalogue.

## Companions, derivation, and resources

Sources retain logical source/module IDs, exact original-byte digests and byte
lengths. A declaration source-map entry covers every complete root, and typed
provenance preserves every distinct dependency occurrence and original span.
They remain separate from logical values and public views. Exact vocabulary
content digests/byte counts and explicit processing limits are derivation facts.
They are excluded from `ProjectIr::logical_eq`, together with source evidence
and resource accounting. Stage 7 owns canonical logical/captured/derivation
identity transcripts; this gate does not publish provisional project hashes.

Resource facts include supplied source/vocabulary counts and bytes, complete
declarations, canonical import edges, and all retained value/default nodes.
The reader recomputes facts and rejects discrepancies. Caller bounds intersect
producer bounds: neither can relax the other. Lowering checks a deterministic
aggregate work/text retention budget derived from the request's output bound
before copying reused/default values. Value/type recursion has the shared hard
ceiling. This is an in-process IR boundary, not a claim about an encoded wire
byte count; later encoding must separately enforce the captured output-byte cap.

## Independent reader — V1-IR-003, V1-API-001

`compile_project` consumes only frozen captured input and explicit cancellation.
Its typed `Result` is governed by `neutral.project-result/1`; failures retain
bounded graph/semantic diagnostics or lowering classification/location and no
authoritative partial project. Stable lowering codes are `NEU-PIR-001`
(incompatible value/default), `NEU-PIR-002` (limit), and `NEU-PIR-003`
(cancellation). The schema is available independently of package versioning.

`ValidatedProject::from_ir` accepts complete data, independent caller limits,
and cancellation. It uses no parser, source text, compiler-private model, or
resolver. It checks schema/profile, module/import and declaration identities,
resolved types, contextual values/defaults, vocabulary schemas, source-map and
provenance coverage, type edges and non-reference semantic cycles, resource
facts, and the exact redacted export index/fingerprint before publication.
Reader failures are typed schema/limit/module/declaration/public-interface/
companion/resource/cancellation classifications. No failure exposes a view.
Source digests account for supplied evidence; a reader without source bytes
cannot prove that an arbitrary producer faithfully compiled those bytes.

## Post-compilation views — V1-VIEW-001, V1-VIEW-002

`neutral.project-view/1` supplies only selected public module-symbol roots.
Roots must be distinct existing public declarations. An empty selection yields
an empty view, not a smaller compiled project. Input order is normalized.
Selection never alters capture, complete IR, or logical equality.

The view contains selected public roots and their public type/value/reference
dependency closure, including reference targets inherited through private
value reuse and transitive locked vocabulary interpretation schemas. It
contains materialized public values and signatures, not private identities,
source IDs, source maps, private defaults, or raw provenance. Public data may
contain values intentionally reused from private bindings; this does not expose
the private binding's identity or source. Format/encoding options belong to
later transport adapters and cannot affect complete IR.

Standalone reader-only project probing and hostile encoded project decoding
remain the subsequent Stage 6 integration/validation gates. The single-file
CLI still rejects the unavailable complete v1 profile; this library boundary
does not silently activate an incomplete command path.
