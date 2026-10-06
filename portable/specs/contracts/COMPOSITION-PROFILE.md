<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition project profile

Status: frozen design and literal oracle inputs, 06-10-2026; **not activated**

This completes the contract gate for [vocabulary composition](VOCABULARY-COMPOSITION.md)
and [variants](VARIANTS.md). It defines the successor boundaries without changing
[project IR /1](PROJECT-IR.md), [identity /1](PROJECT-IDENTITY.md), old vocabulary
schemas, or their literal vectors. The fixture suite is `composition-contract`;
its status is `frozen`, not `required`. Registration is not implementation evidence.

## Explicit selection and migration

The new captured request is `neutral.capture/v2`: the /1 fields plus required
`required_features` and `composition_limits`. The sorted unique feature set is
exactly `["tagged-variants-v1", "vocabulary-composition-v2"]`; both declaration
origins are supported together, never as different semantic systems. Core source
profile remains `1.0`. A source header alone does not enable this feature set.
No package version, source content, or failed old-schema parse selects /2.
Unavailable producers/readers reject before interpreting new declarations.

| Input | Boundary | Required behavior |
| --- | --- | --- |
| `neu "0.1"`, legacy document/vocabulary | Existing v0 boundary | Unchanged; variants rejected |
| Capture /1, vocabulary schema `1.0` | Existing project /1 | Unchanged; no optional absence or new type forms |
| Capture /1, vocabulary schema `2.0` | Existing project /1 | Unsupported schema; no retry as /2 |
| Capture /2, schema `2.0` exact transitive locks | Composition project /2 | Complete new validation before publication |
| Capture /2, schema `1.0` leaf | Composition project /2 | Explicit adapter: required fields, empty restrictions, no defaults/dependencies |
| Unknown feature/schema/request/IR/wire/identity profile | Any boundary | Unsupported; no fallback, heuristic conversion, or partial result |

Old data is not silently rewritten. An explicit /2 request may produce new /2
identities even for equivalent legacy meaning; equality of different profiles'
digests is neither promised nor treated as semantic conversion. Presentation is
a separate authoring input; schema `2.0` rejects presentation/executable members.

## Diagnostics, attribution and failure precedence

Capture /2 retains the /1 `NEU-CAP` classifications. `NEU-CAP-001` also covers
an unsupported feature set; `NEU-CAP-012` covers zero composition limits. Catalogue
failures retain the frozen `NEU-COM-001` through `018` table. The source boundary
adds the following codes without renumbering existing source diagnostics:

| Code | Meaning |
| --- | --- |
| `NEU-COMP-001` | Malformed variant declaration or contextual tag/payload literal |
| `NEU-COMP-002` | Duplicate tag or duplicate/unknown/missing variant member |
| `NEU-COMP-003` | Unknown selected tag |
| `NEU-COMP-004` | Incompatible payload, field, reuse or reference type |
| `NEU-COMP-005` | Missing required field, including required nullable fields |
| `NEU-COMP-006` | Semantic restriction violation |
| `NEU-COMP-007` | Private type/reference exposed by a public signature/value |
| `NEU-COMP-008` | Embedded type/default expansion cycle |
| `NEU-COMP-009` | Non-reference value/reuse cycle |
| `NEU-COMP-010` | Processing/output limit or allocation/arithmetic failure |
| `NEU-COMP-011` | Cooperative cancellation |

Preflight checks unsupported envelopes/features, then zero controls before work;
an already cancelled
request then fails before capture. Thereafter cancellation wins at the next
checkpoint, and a limit/allocation failure terminates work immediately. These
are whole-request failures, not recoverable semantic diagnostics. Integrity,
strict JSON/schema/lock validation, dependency closure, type/public closure,
default validation, then supplied-value checks run in that dependency order.
Do not resolve a value against an invalid signature or cascade diagnostics from
an invalid tag into payload typing. A malformed variant value reports shape
before tag lookup, then payload typing, then restrictions. Missing fields are
reported in field-name byte order. Recovery can skip to the next declaration
but may never publish recovered IR.

Within a phase, declarations/bundles use canonical owner order; fields/tags use
exact UTF-8 order. Diagnostics sort by `(phase, owner, original byte start,
original byte end, code, safe parameter tuple)`; no scheduler order enters the
key. At the diagnostic cap, fail with `NEU-COMP-010`, not truncated success.

Source diagnostics use original-byte spans for the declaration/type, selected
tag, offending payload/field, or closing brace for omission. A vocabulary-origin
default/restriction error identifies `(identity, revision, type, field/tag)` and
an optional checked bundle-byte span, never a fabricated source span. A supplied
value violating that contract has a primary source occurrence and a related
canonical vocabulary owner. Reuse preserves original occurrence facts; public
views redact private source IDs/spans/owners. Unavailable/redacted attribution is
explicitly absent. Host paths, raw source text and acquisition hints are never
diagnostic parameters.

## Independent resource policy

All counters are positive `u64`, checked before proportional allocation. Existing
capture, JSON, project, reader and identity limits remain independent. /2 carries
the following additional ordered tuple; this order is also used by derivation
and wire contracts:

`bundles, captured-bytes, dependencies-per-bundle, dependency-edges,
dependency-depth, total-types, total-fields, alternatives-per-type,
total-alternatives, choices-per-field, total-choices, type-depth, value-depth,
value-nodes, work`.

These are the standalone `CompositionLimits` counters, not semantic restrictions.
Counts intersect 1,000,000; bytes intersect 67,108,864; depths intersect 64.
At the project boundary, bundle/byte/dependency counters cover the complete
captured vocabulary closure. Type/field/alternative/choice totals cover both
source and vocabulary definitions, including private/unused definitions, exactly
once per canonical owner; materialized occurrences are counted separately.
`type-depth` counts composed wrapper descent with a scalar/nominal owner at zero;
embedded-owner closure has its own bounded traversal and may not bypass work or
the shared depth ceiling. Dependency depth counts bundles on the longest path,
including the root and leaf (a leaf alone has depth one); a reused diamond leaf
does not shorten another path. Field/tag/default public closure includes every
unselected branch. Counting all declarations is not permission to double-charge
one locked bundle reached through multiple source aliases.
The existing JSON policy additionally bounds per-string bytes, numeric digits
and absolute scale, fields, list items, array items and total nodes; object
members intersect eight. Decoded text byte counts and Unicode scalar length
restrictions are different measures. Exact numeric comparison charges inspected
coefficient bytes to work; exponent-sized padding is forbidden. Every traversal
charges work, including reference checks and unselected variant alternatives.
Reference targets are not embedded, and traversals memoize resolved owners.

Materialization counts the root at depth zero. Record/list/variant child descent
adds one; nullable checks consume a value visit without adding depth. Optional
absence and defaulted children consume visits. Shared defaults/reused values
charge each retained occurrence: a DAG is not permission for exponential copying.
All retained strings, origin paths, defaults, choices, type nodes and value nodes
also obey the project retention/output caps. Reader reference-type enumeration
has independent `visits`, `references`, `depth` caps as already implemented.
Diagnostics and encoded bytes are independently bounded by capture controls.
Fallible reservations, checked arithmetic and cancellation checks before/within
loops and immediately before publication are required. No allocator abort,
panic, stack overflow, partial origins/view/IR, or implicit unlimited fallback
is an accepted failure mode. Implementation allocation-fault review is a later
validation gate, not evidence supplied by this design freeze.

## Complete logical IR /2

Schema is `neutral.project-ir/2`; view and result envelopes are
`neutral.project-view/2` and `neutral.project-result/2`. The /1 complete-set,
independent reader, companion validation and public-root rules still apply.
Nominal owners are source symbols `(profile,module,declaration)` or exact
vocabulary tuples `(identity,revision,name)`. There are no alias/path owners.

The complete logical projection has ordered sections `schema`, `profile`,
`modules`, `declarations`, `vocabulary-types`, `vocabulary-catalogues`,
`vocabulary-dependencies`, `public-edges`. Modules/imports, symbols and edge
ordering follow identity /1. Declarations retain `symbol`, `public`, `signature`,
`value`. A signature is a binding type, record fields, or variant alternatives.
Each record field retains `name`, resolved `type`, `presence`, restrictions and
a default state; source fields without defaults are required with empty
restrictions. Source defaults become defaulted fields. Each alternative retains
`tag` and resolved payload type. Vocabulary types retain their exact owner,
visibility and record/variant body, including private and unused definitions.
Catalogues retain logical schema, feature set and public type names; dependencies
retain every exact edge, not source alias spellings. A legacy leaf is explicitly
represented with schema `1.0` and empty features/dependencies.

Optional absent fields use an `omitted` state, not null or a missing dictionary
key. Declaration-without-value uses `absent`, permitted only for type declarations.
Variants retain selected tag and payload; their owner comes from the validated
expected type. Lists remain ordered. Defaults are fully materialized closed
values; non-null references in defaults are forbidden. Ref values carry stable
binding symbols; they do not embed targets. Both type and value reference
dependencies remain reader-visible, including unselected alternative types.

Companions retain safe origin paths and `supplied`, `explicit-null`,
`omitted-optional`, `defaulted` classifications separately from meaning. Defaults
need canonical vocabulary/type/field attribution, not invented source bytes.
Public closure traverses every field/tag/default and exposed reference target;
private identities/provenance cannot escape in a public view. Complete roots
never affect capture, logical projection or their identities.

## Canonical identity /2

Identity profile is `neutral.project-identity/2`. NHT framing, SHA-256, fixed
integer widths, bounds and exclusions follow identity /1. New domains are
`neutral/project-{captured,logical,derivation,artifact}/v2`. No new tags are
added to /1. The captured body and artifact body have the /1 grammar but the
new profile/domain; captured /2 additionally appends `features` with the exact
request feature set. Exact source/bundle bytes and lock fields remain captured.
Derivation has the /1 fields followed by `composition-limits` in the tuple above.

The logical body uses the eight sections listed above. All structural sections
are frames, even when empty. A declaration is framed as `declaration` containing
symbol frames, `public` byte, `signature` wrapping `binding`/`record`/`variant`,
then `value`. Record bodies contain sorted `field` frames; variant bodies contain
sorted `alternative` frames (`tag`, type). A field contains `name`, type,
`presence`, `restrictions`, `default`. Default wraps `absent` when none or its
materialized value. Restrictions always contain, in order: `choices`, `minimum`,
`maximum`, `min-length`, `max-length`. Choices contain scalar value frames;
absent bounds contain `absent`, present numeric bounds contain a `num` value;
present length bounds contain a `length` unsigned integer frame. Choices sort
by exact scalar value (numbers mathematically, Booleans false before true,
text exact UTF-8); all choices have one type. Duplicate normalized choices reject.

Types retain /1 framing. `nullable` cannot nest directly; `Ref` must wrap a
nominal/vocabulary type. Values retain /1 framing with two additions:
`omitted` is an empty frame permitted only in optional record fields; `variant`
contains `tag` UTF-8 followed by `payload` wrapping its typed value. `absent`
cannot occur inside a value/default. Number coefficients/scales/signs must be
normalized: zero is unsigned coefficient `0`, scale zero; nonzero coefficients
have no leading/trailing zero. No float, raw JSON number or textual numeric
spelling becomes logical identity.
The value is `sign * coefficient * 10^scale`, as in the existing exact-number
contract; the sign is negative iff its byte is `01`.

`vocabulary-types` contains sorted `definition` frames: `identity`, `version`,
`name`, `public`, then `body` wrapping `record` or `variant`. Catalogues contain
`vocabulary` frames with `identity`, `version`, `schema-version`, `features`,
`public-types`. Dependencies contain `vocabulary` frames with `identity`,
`version`, `dependencies` containing sorted `dependency` frames of `identity`,
`version`. Sort definitions by their full owner tuple; catalogues/dependency
owners by `(identity,version)`. Each supplied bundle has one dependency record,
including empty leaves. Existing public-edge grammar/order is unchanged.

The public interface /2 uses the same NHT profile with domain
`neutral/project-interface/v2` and the logical projection of all public
declarations and public interpretive catalogue closure, no private provenance.
Its digest is independently recomputed; it is not complete project identity.
The literal vector corpus contains complete framing/digests and changed-default,
restriction, absence/null, variant/tag/order, canonical dependency revision and
capture/root/presentation controls. The oracle consumes resolved facts, not
source text: it is not a replacement parser or an implementation conformance claim.

## Successor binary transport

`NIR-PROJECT-CBOR/2` has distinct eight-byte magic `NEUP2\r\n\x1a` and one
restricted-CBOR array of sixteen positions. /1 minimal-width integers, definite
arrays/text/bytes, no maps/floats/tags/indefinite forms or trailing bytes apply.

| Position | Content |
| --- | --- |
| 0 | `neutral.project-ir/2` |
| 1 | Modules, unchanged /1 tuples |
| 2 | `[symbol, public, signature, value_or_null]` declarations |
| 3 | `[identity, version, name, public, body]` vocabulary definitions |
| 4 | `[identity, version, schema_version, features, public_type_names]` catalogues |
| 5 | Exact 32-byte independently recomputed interface /2 digest |
| 6–8 | /1 source, source-map and provenance tuples |
| 9–11 | /1 project-limit, resource-fact and vocabulary-source tuples |
| 12 | `[identity, version, [[dependency_identity, revision], ...]]` closure |
| 13 | `[binding_symbol, path, origin_kind, attribution_or_null]` origin occurrences |
| 14 | The ordered fifteen composition limits |
| 15 | `[bundles, dependency_edges, types, fields, alternatives, choices, retained_value_nodes]` facts |

Signature/body is `["binding", type]`, `["record", fields]`, or
`["variant", alternatives]`; fields are `[name, type, presence, restrictions,
default_state]`; alternatives are `[tag, type]`. Restrictions are
`[choices, minimum_or_null, maximum_or_null, min_length_or_null, max_length_or_null]`.
Default state is `[false]` or `[true, value]`; declaration CBOR null means no
value, not semantic null. /1 type/value tuple spellings are retained;
`["omitted"]` and `["variant", tag, payload]` are new values. Origin paths use
`["field", name]`, `["element", u64]`, `["payload"]`; origin kind is one of the
four exact spellings above. Attribution is `["source", location]` or
`["vocabulary", identity, version, type_name, field_name, byte_span_or_null]`.
Every origin/attribution is independently checked against retained companions.
Reader limits intersect producer/hard caps and decoded facts are recomputed;
unknown tags, duplicate/noncanonical order and dangling targets fail closed.
Producer execution-work observations are not independently reconstructible from
IR and therefore are not asserted resource facts; reader work is charged against
its own limit. Derivation still binds every declared acceptance limit.

Transport /2, interface /2 and origin companions are frozen requirements, not
currently supported codecs. Old decoders must reject this distinct magic/schema.
Activation requires compiler/codec/independent reader/probe, hostile decoding,
fault injection and clean/cache tests. The first two checklist gates close only
design and literal registration; those implementation/validation gates stay open.
