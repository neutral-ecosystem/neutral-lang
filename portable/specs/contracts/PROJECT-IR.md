<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete project IR, reader, and view contract

Status: accepted Stage 6 contract, encoded integration, and reader boundary

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
ceiling. `ProjectLimits::artifact_bytes` retains the captured output-byte cap
independently of materialized-value work. Encoding and decoding enforce that
producer cap as well as independent caller and hard wire limits. Source IDs
also obey the reader's text bound. No in-process node count substitutes for an
encoded byte count.

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
the private binding's identity or source. Transport options cannot affect
complete IR.

## Complete project transport

`NIR-PROJECT-CBOR/1` starts with the eight bytes `NEUPR\r\n\x1a`, followed by
exactly one restricted-CBOR array with twelve positions in this order:

| Position | Content |
| --- | --- |
| 0 | Exact complete IR schema string |
| 1 | Modules: `[module_identity, import_targets]` |
| 2 | Declarations: `[symbol, public, signature, value_or_null, defaults]` |
| 3 | Vocabulary records: `[identity, version, name, public, fields]` |
| 4 | Vocabulary catalogues: `[identity, version, public_type_names]` |
| 5 | Declared public-interface fingerprint, exactly 32 bytes |
| 6 | Sources: `[module, source_id, digest_bytes, byte_length]` |
| 7 | Source maps: `[symbol, location]` |
| 8 | Provenance: `[from_symbol, to_symbol, edge_kind, location]` |
| 9 | Limits: `[modules, declarations, import_edges, nodes, text_bytes, artifact_bytes]` |
| 10 | Facts: `[source_units, source_bytes, vocabulary_units, vocabulary_bytes, declarations, import_edges, value_nodes]` |
| 11 | Vocabulary sources: `[identity, version, digest_bytes, byte_length]` |

Every collection is a definite array; every tuple has exact cardinality.
Module identities are `[profile, module_name]`, symbols are
`[module_identity, declaration_name]`, locations are
`[source_digest_bytes, span_start, span_end]`, and fields are `[name, type_or_value]`.
Signatures are `[false, binding_type]` or `[true, record_fields]`.
Scalar types are one-element tuples tagged `num`, `string`, `bool`, `url`, or
`path`; wrappers are `[List|Ref|nullable, inner_type]`; nominal types are
`[nominal, symbol]` or `[vocabulary, identity, version, type_name]`.
Values are `[null]`, `[num, negative, coefficient, signed_scale]`,
`[string|url|path|bool, scalar]`, `[Ref, symbol]`, `[List, values]`, or
`[record, fields]`. Edge ordinals are 0 type, 1 reference-type, 2 value,
3 reference. Unknown tags/ordinals, duplicate identities/fields, unordered
canonical collections, extra/missing positions, and trailing bytes fail closed.

The shared restricted-CBOR parser enforces bytes, strings, container items,
depth, traversal, and cancellation before allocation. Equivalent integer-width
encodings are accepted; the writer emits shortest widths. Export signatures
and public edges are recomputed from complete declarations and provenance,
then independently validated against the declared fingerprint and full IR.
No duplicated export index can override complete content. This transport does
not claim an integrity signature or a Stage 7 captured/logical/artifact identity.
The legacy document `NIR-CBOR/0.1` format remains unchanged and distinct.

`neutral-probe [--json] [--root module::declaration ...] <artifact>` recognizes
both formats without compiler linkage. Project inspection validates the complete
artifact before selection, exposes complete counts plus the public view, and
never prints private declaration identities, source IDs, spans, or provenance.
Omitted roots select all public exports; the library's explicit empty selection
produces an empty view. Text logs use category prefixes; JSON stdout contains
only an indented inspection document. Any failure exits unsuccessfully and
emits no partial summary. Host file reads are bounded before decoding.

The single-file compiler CLI still rejects the unavailable complete v1 profile;
the library and probe integration do not silently enable incomplete acquisition.
