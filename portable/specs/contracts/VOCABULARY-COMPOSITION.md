<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vocabulary composition extension

Status: standalone catalogue boundary implemented; project activation and identity/wire freeze pending

This document specifies the catalogue contract slice for V1-VOC-005..011.
The separate `neutral_vocabulary::composition` API validates this bundle shape;
the compiler does not yet accept it as a project input. It is not proof that
the promotion gate is complete. Existing
[project vocabulary](VOCABULARY.md), project IR, wire, and identity contracts
remain unchanged. [Variants](VARIANTS.md) supplies the shared source/vocabulary
variant rules. The extension has no executor, acquisition, or callback channel.

## Explicit selection and compatibility

The catalogue API selects bundle encoding `1.0` (the existing strict JSON representation) with
**new logical schema `2.0`**. Schema `2.0` has the exact required feature set
`["vocabulary-composition-v2"]`; it covers all forms below, not only the forms
used in a particular bundle. Locks must match encoding, schema, feature set,
identity, exact semantic revision, and captured-byte digest. Package SemVer
never chooses these contracts. Unknown schema/features reject, not downgrade.

The old schema `1.0` keeps its exact member sets, scalar/local nominal types,
source-required lock cover, and lack of semantic defaults/restrictions. A `2.0`
bundle cannot be disguised as `1.0` by changing a feature list. v0 behavior and
all accepted literal vectors remain immutable.

The proposed new project IR schema, transport and identity profile are separate
contracts, not edits to their existing versions. Their exact layouts and literal
vectors must be frozen before this proposal is activated. Projects using these
forms require an advertised composition-capable producer and reader; source
variants additionally require the advertised tagged-variant capability. Merely
recognizing a source header or JSON schema is not implementation availability.

## Closed envelope and type definitions

The exact envelope members are `format`, `encoding_version`, `schema_version`,
`identity`, `version`, `required_features`, `dependencies`, and `types`.
`format` remains `neutral-vocabulary-bundle`. Identity/name/revision grammar,
strict UTF-8, duplicate-member rejection, inert text, and numeric JSON-token
prohibition are inherited. Every object below rejects extra/missing members
unless a member is explicitly optional. Arrays cannot contain duplicate logical
entries even if their JSON representations differ.

Each dependency has exactly `identity` and `version`. Each record type has
exactly `kind: "record"`, `name`, `public`, and `fields`. Each variant type has
exactly `kind: "variant"`, `name`, `public`, and `alternatives`, as specified by
the variant proposal. Public is Boolean; type names are unique across kinds.
Record field names and variant tags obey existing non-protected snake-name
rules. No field or alternative can add executable or presentation metadata.

Type expressions use the following exact object shapes. Strings in this table
are literals or values, not an embedded type-expression language:

| Kind | Exact members | Meaning |
| --- | --- | --- |
| `num`, `string`, `bool`, `url`, `path` | `kind` | Existing distinct scalars |
| `nominal` | `kind`, `name` | Record or variant in this bundle |
| `external` | `kind`, `identity`, `version`, `name` | Public type in a declared, exactly captured dependency |
| `list` | `kind`, `element` | Invariant ordered list of the nested type |
| `nullable` | `kind`, `inner` | One nullable layer; nested nullable is rejected |
| `ref` | `kind`, `target` | Invariant reference to a non-null nominal/external owner |

List/nullable may compose at different layers. Reference targets are nominal,
never scalar/list/nullable or a source-local alias. Only source-declared types
can name source module-symbol owners; a captured vocabulary cannot depend on a
project's source declarations. Embedded recursion is rejected across records,
variants, lists, and nullable wrappers. A reference edge does not embed its
target; recursive reference types still cannot legalize a source value cycle.

## Field presence and closed defaults

Each field has exactly `name`, `type`, `presence`, and `restrictions`; only a
`defaulted` field additionally has the required `default` member.

| Presence | Omitted initializer | Explicit `null` |
| --- | --- | --- |
| `required` | Rejected even if nullable | Accepted only for a nullable type |
| `optional` | Retained as absent, not null | Accepted only for a nullable type |
| `defaulted` | Materialize the complete validated default | Never replaced by a default; accepted only for a nullable type |

Absence is a record-field state, not an assignable scalar, list element, variant
payload, reference target, or expression. No new source optional-field syntax
is introduced by this vocabulary contract. Source-authored record defaults keep
their existing behavior. New IR must carry optional absence explicitly; old IR
cannot represent it and must not be extended with an ambiguous sentinel.

Closed default objects are contextually checked against the declared type:

| Value | Exact JSON representation |
| --- | --- |
| Number | `{ "kind": "num", "value": "3" }`, using a bounded Neutral numeric literal |
| String, URL, path | `{ "kind": "string"/"url"/"path", "value": "…" }` |
| Boolean | `{ "kind": "bool", "value": true }` |
| Null | `{ "kind": "null" }` |
| List | `{ "kind": "list", "items": [closed values] }` |
| Record | `{ "kind": "record", "fields": [{ "name": "…", "value": closed value }] }` |
| Variant | `{ "kind": "variant", "tag": "…", "payload": closed value }` |

Record/variant owner comes from the expected nominal type, not an unchecked
string embedded in a value. Missing nested fields use the same presence rules;
unknown/duplicate fields reject. Defaults may never introduce reference targets,
source names, aliases, expressions, or deferred inputs. Null under a nullable
reference type is closed because it introduces no reference edge. Materialize
defaults with bounded work, detecting expansion cycles before allocation. An
invalid default rejects the bundle even when no source binding uses it.

## Restrictions

`restrictions` is a possibly empty object. Allowed optional members are
`choices`, `minimum`, `maximum`, `min_length`, and `max_length`. No other
validator language is allowed. Variant alternative types carry no restriction
object; restrictions on record payloads come from their field contracts.

- `choices` is a nonempty list of closed **non-null scalar** values of one
  exact scalar type. Duplicate normalized numbers/text/Booleans reject. Scalars
  of different types never compare equal. Lists/records/refs/variants reject.
- `minimum` and `maximum` are numeric literal strings, applicable only to `num`
  (or its outer nullable form). Comparison is exact, inclusive, and bounded;
  no floating-point conversion or exponent-sized zero expansion occurs.
- Length bounds are canonical unsigned-decimal **strings** (including `"0"`),
  checked within `u64`, and apply only to strings/URLs/paths or lists, possibly
  outer-nullable. Text length counts decoded Unicode scalar values, not UTF-8
  bytes, graphemes, or normalized text. List length counts immediate elements.
- Bounds apply to non-null present values. Null requires nullable typing and
  is exempt from scalar/length restrictions; absence requires optional presence.
- Reversed bounds, an empty choice set, incompatible restrictions, or a choice
  outside the same bounds reject the bundle. All choices must satisfy the bounds.
  Defaults and supplied/reused values satisfy the same complete restrictions.

Finite choices sort by typed canonical value for identity. There is no regex,
coercion, URL parsing, path normalization, uniqueness policy, network lookup,
authorization, or executable dispatch. Resource limits remain independent of
semantic length restrictions: permitting long text does not enlarge a budget.

## Exact captured dependency closure

Collect direct source requirements, then traverse declared bundle dependencies
without acquisition. Supplied locks/bundles must equal that whole closure.
Diamond reuse of the same canonical identity/revision is allowed; conflicting
revisions, duplicate supplied locks, missing/extra bundles, self-dependencies,
and dependency cycles reject. An unused declared dependency rejects: it must
be named by at least one `external` type anywhere in that bundle, including
private definitions. Source aliases do not participate in this graph.

An external type must match a declared dependency and an explicitly public
target. Public closure follows every field/alternative, even optional fields,
unused defaults, and unselected variant alternatives. A `2.0` bundle may depend
on a captured `1.0` leaf bundle; that leaf has no dependency member or new forms.
Capture must validate the complete dependency set before publishing any project.
Closure does not authorize a consumer to open locations or obtain credentials.

## IR, provenance, identity, and validation

Independent readers require the complete resolved type catalogue, dependency
revisions, presence, restrictions, materialized defaults, variants, and selected
values. Public views retain all interpretive dependencies and exclude private
implementation provenance. New origins distinguish supplied, explicit-null,
omitted-optional, and defaulted occurrences; reuse links preserve originating
facts. Safe attribution can be unavailable/redacted, never invented.

Logical identity includes field presence policies, semantic defaults (even
unused), restrictions, the complete nominal catalogue and exact dependency
revisions, variant alternative contracts, selected tags, and final typed values.
Absent and null differ. An explicitly supplied value equal to a materialized
default has the same value meaning; supplied/defaulted occurrence evidence stays
in companions. Capture and derivation bind exact bytes and source evidence.
Aliases, roots, presentation, capture order, and host paths remain non-semantic.

Independent budgets must cover per/total bundles, dependency edges and closure
depth, types, fields, alternatives, choices, composed-type depth, default/value
depth/nodes/expansion, text bytes, numeric digits/scale/comparison work, traversal,
diagnostics, and output bytes. Existing JSON depth intersects the hard 64-layer
ceiling; there is no unbounded fallback. Every independent counter needs exact
and one-over tests. Allocation/arithmetic failure, cancellation, malformed
input, unsupported profile, and invalid contract must publish no partial data.

## Remaining freeze work

### Standalone supplied-value boundary

The catalogue API also accepts closed, already captured values under an explicitly
selected public vocabulary nominal type. It uses the same type/default/restriction
validator as bundle defaults; it does not parse source or activate a project profile.
Required fields reject omission, optional omission remains absent, and defaulted
omission materializes the validated default. Explicit null never requests a default.
Unrecognized/duplicate fields, tags, payload types and restriction violations reject
the whole request. A private root type is not an available public value contract.
Reference-bearing non-null values remain unavailable at this closed-value boundary;
it must not invent a target or silently discard reference dependencies.

Successful values expose an immutable canonical value and separately ordered
occurrence facts: `supplied`, `explicit-null`, `omitted-optional`, or `defaulted`.
Paths consist only of validated field names, list indices and variant payload steps.
Defaulted provenance propagates to nested materialized children. No source bytes,
host paths, fabricated spans, or acquisition locations are attached. These are
occurrence classifications, not the future project's source-map companion.
Supplying the same materialized value produces equal meaning but different origin
facts. Origin facts must never become part of a logical-value transcript.

The independent `value_nodes` budget counts materialization visits (including
nullable wrapper checks and absent field states) cumulatively per catalogue/default
validation or supplied-value request. It derives from the existing total-node policy
and intersects the 1,000,000 hard ceiling. Existing per-value string-byte, numeric
digit/scale, list-item and field budgets apply equally to caller-supplied values.
Work/depth/cancellation remain independent. Bounded result/origin reservations
report `NEU-COM-018` on allocation failure; invalid supplied values use
`NEU-COM-017`, distinct from invalid bundle defaults (`NEU-COM-016`).
This does not complete the whole-pipeline allocation-fault review.

Public reader APIs may enumerate reference **type** dependencies in canonical
field/tag paths, including nullable/list wrappers and unselected alternatives.
Those facts are not captured binding/reference-value edges or an execution order.
The per-definition reader traversal accepts nonzero independent visit, reference
count and depth limits, intersected with catalogue hard ceilings. Its paths label
record fields, variant alternatives, list-element types and nullable inner types.
Every alternative is inspected, whether selected by a value or not. References
are emitted with their exact resolved nominal target; traversal does not expand
the referenced target or turn a reference cycle into recursive embedded work.
Public consumers can enumerate all public definitions to obtain the full catalogue
reference-type dependency set. Cancellation/limits publish no partial enumeration.

The [complete example bundle](../decisions/composition/bundle.json) makes these
member sets concrete. It is a proposed input, not an active conformance case.
Before project activation, finish:

1. Complete compiler diagnostic precedence/recovery, precise source-attribution
   fallback rules and allocation-fault review. Catalogue budgets and safe
   boundary diagnostic codes are specified below and implemented separately.
2. Complete project IR/wire version selectors/layouts and new transcript tags,
   exact independent positive/adversarial digest vectors, and migration matrix.
3. Captured project envelopes, exact lock digests, literal rejection/oracle
   files, boundary fixtures, and manifest activation with runtime execution copies.

The Stage 7 checklist remains unchecked until those artifacts, implementation,
public integration, and full validation exist. Compatibility tests in the
vocabulary crate protect the old schema while this design is completed.

## Implemented catalogue boundary

The shared raw structures live in `neutral_ir::composition`; construction alone
does not establish validity. `validate_composition_closure` takes only borrowed
captured bytes, exact locks, canonical source requirements, explicit limits and
a cancellation token. It publishes an immutable `ValidatedComposition` only
after complete schema, dependency, public/embedded closure, restriction and
default validation. This is not a compiled project or encoded artifact.

`neutral_reader::composition::CompositionCatalogue` exposes public type,
canonical revision/profile and dependency facts from that validated catalogue.
Private and absent type lookup share one safe error; debug output includes
public counts only. This API has no compiler dependency or invented source
provenance, and does not claim complete project/wire/probe integration.

`CompositionLimits::from_vocabulary` derives defaults from existing caller
policy rather than duplicating a release-specific policy. Every independent
field is nonzero and can be narrowed separately:

| Budget | Default source | Additional hard ceiling |
| --- | --- | --- |
| bundles, total_types | vocabulary type count | 1,000,000 |
| captured_bytes | per-bundle byte policy | 67,108,864 total bytes |
| dependencies_per_bundle, choices_per_field | JSON array-item policy | 1,000,000 |
| dependency_edges, total_fields, total_alternatives, total_choices, work | JSON total-node policy | 1,000,000 |
| alternatives_per_type | field count policy | 1,000,000 |
| dependency_depth, type_depth, value_depth | nesting policy | 64 layers |
| value_nodes | JSON total-node policy | 1,000,000 materialization visits |
| JSON object members | object-member policy | 8 members, the largest closed envelope |
| JSON nodes/array items | existing per-bundle policy | 1,000,000 |

Aggregate work charges captured bytes, recursive traversal/default expansion,
exact comparison coefficient lengths and conservative ordered-key/choice
inspection. Decimal exponents never allocate corresponding zero padding.
Restrictions do not enlarge any resource budget. JSON decoding observes
cancellation inside node and string traversal; final checks precede publication.

The catalogue boundary exposes stable `NEU-COM` codes with structured nested
vocabulary classifications and no captured text/host path:

| Code | Classification |
| --- | --- |
| `NEU-COM-001` | Strict vocabulary/JSON/envelope/lock failure |
| `NEU-COM-002` | Zero caller budget |
| `NEU-COM-003` | Resource/hard limit exceeded |
| `NEU-COM-004` | Cancelled |
| `NEU-COM-005` | Duplicate/conflicting bundle identity |
| `NEU-COM-006` | Missing/mismatched dependency revision |
| `NEU-COM-007` | Extra supplied bundle |
| `NEU-COM-008` | Duplicate/self/unused/undeclared dependency |
| `NEU-COM-009` | Bundle dependency cycle |
| `NEU-COM-010` | Private type exposure |
| `NEU-COM-011` | Unknown nominal type |
| `NEU-COM-012` | Embedded type cycle |
| `NEU-COM-013` | Invalid contract shape |
| `NEU-COM-014` | Invalid/incompatible/contradictory restriction |
| `NEU-COM-015` | Duplicate normalized finite choice |
| `NEU-COM-016` | Invalid closed default |
| `NEU-COM-017` | Invalid supplied closed value |
| `NEU-COM-018` | Bounded result/origin allocation failure |

Schema/graph checks precede default materialization; dependency availability,
revisions and cycles precede nominal target resolution. Compiler diagnostic
ordering, precise source attribution, recoverability, allocation-fault review,
all profile vectors and full artifact integration remain separate requirements.
No source span is invented by this captured-byte boundary.

Run `cargo test --package neutral-vocabulary --test composition` for focused
catalogue tests and `cargo xtask test all` for repository-wide regression tests.
Runtime-owned fixtures are independent of the portable plan and remain runnable
after archiving it. The vocabulary fuzz target also exercises this boundary.
