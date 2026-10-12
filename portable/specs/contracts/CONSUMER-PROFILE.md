<!-- SPDX-License-Identifier: Apache-2.0 -->

# Consumer data and reader protocol

Status: frozen V1-CONS-001..010 design and literal oracles, 12-10-2026;
implementation not activated

This refines [consumer readiness](CONSUMER-READINESS.md) without changing frozen
project IR, wire formats, identities, or [authoring foundation](AUTHORING.md).
`neutral.consumer/1` is a reader/protocol profile, not a source header, package
version, execution engine, or universal operation vocabulary. V1-CONS-011..016
remain unfrozen; this protocol does not grant arithmetic/template syntax.

## Ownership and ordinary typed data — V1-CONS-001..005

Consumers define exact versioned semantic vocabularies for operations, inputs,
results, resources, prerequisites, conditions, and handles. Neutral retains and
validates their ordinary types/defaults/restrictions/variants/references and
independently exposes them. A type named `Operation` or a field named `needs`
has no built-in compiler meaning. Different nominal owners/revisions are not
interchangeable merely because their fields look alike.

The project-neutral example convention has these exact roles:

| Role | Required information | Interpretation owner |
| --- | --- | --- |
| Operation | Stable declaration identity, exact operation contract, typed arguments and result contract | Consumer |
| Resource | Exact resource contract and ordinary typed configuration; referenced by identity | Consumer |
| Prerequisite | Explicit predecessor and dependent operation identities | Consumer scheduler |
| Output connection | Producing operation identity, declared result-type selector, receiving argument path/type | Consumer binding validator |
| Deferred input | Closed variant `literal`, `input`, `output`, or `handle` with exactly its typed payload | Consumer binder |
| Condition | Closed typed records/variants for literal Boolean, equality, conjunction, disjunction and negation | Consumer condition interpreter |
| Handle | Exact provider-neutral scope and symbolic name; no credential value | Consumer authorization/credential boundary |

This is an example convention, not a mandatory global schema or magic field
recognition. Its exact file-backed vocabulary/source fixtures belong to the
separate operation-graph registration task. Source authors use ordinary records,
variants and refs; core never evaluates an operation result or condition. Unknown
condition alternatives and malformed payloads fail type/schema validation.
Condition interpretation must specify unavailable outputs and short-circuit rules
in the consumer's exact contract; it cannot borrow backend expression strings.
General pure-expression semantics are deliberately not frozen here.

Prerequisite edges, potential output/value dependencies, structural refs, and
module imports remain distinct. Core-valid graphs may have consumer-invalid
execution cycles or unavailable outputs. Consumers validate those separately;
reports must not mislabel them as compiler failures. No outputs are fetched,
packages installed, handles resolved, conditions evaluated or policies authorized
by core acceptance. Symbolic handle names are ordinary data, not proof of access
or proof that a user has not accidentally placed secret text in a string.

Generic authoring discovers these ordinary vocabulary types and preserves their
connections/default/restriction facts with no product-specific palette. Their
record fields can represent selectors as data without extending authoring /1's
whole-declaration edge syntax. Native selector ports or expression/template
editing require separately frozen authoring features; no hidden source escape
hatch or implicit flattening is permitted.

## Capability/mapping descriptions and reports — V1-CONS-003

Consumers own the payload schema and capability taxonomy. The closed protocol
envelope is `{schema,contract,adapter_revision,target_revision,required_features,
payload}`. `schema` is `neutral.consumer-description/1`; `contract` is the exact
public nominal type ID of the validated payload. Adapter/target revisions are
nonempty opaque public revision identifiers, never paths or credentials.
Required features are sorted unique exact strings. There are no executable
plugins, mapping callbacks, unvalidated bags, or automatic alternatives.

A report is `{schema,contract,project_identity,entry,context,status,diagnostics}`,
with schema `neutral.consumer-report/1`, a typed versioned report contract,
complete project logical identity, selected public symbol or explicit null,
and context `{adapter_revision,target_revision,input_revision}`. A context
revision identifies relevant bindings without containing raw credentials.
`status` is `accepted`, `rejected`, or `alternative-required`. Each diagnostic
contains `{phase,code,subject,path,primary,related}`; phase is `reader-contract`
or `consumer`. Codes are registered by their owning protocol, not human text.
Primary location may be explicitly unavailable; related locations are bounded.
Consumer diagnostics never impersonate core compiler diagnostics.

Unsupported capabilities are rejected explicitly. `alternative-required` is not
acceptance: an alternative must be identified by exact contract/revision,
explicitly approved by the host, rebound into a new context, and revalidated.
Never silently drop security, availability, ordering, or result requirements.
Compiler logical identity excludes adapter, target, selection and external
binding choices; consumer plan/cache keys bind all of them plus consumer profile
and contract revisions. No universal cache-key digest algorithm is asserted.

## Member selectors — V1-CONS-007

Request schema `neutral.member-selector/1` has exactly
`schema,mode,root,path,expected_type`. Mode `value` roots at an existing public
binding symbol `[source_profile,module,name]`; mode `type` roots at an exact
public source/vocabulary nominal type ID. Type IDs and wrapper shapes follow
[AUTHORING's canonical type descriptors](AUTHORING.md#static-catalogue--v1-auth-001-v1-auth-002).
All lookups use validated facts, not source aliases or dotted strings.

The path is an ordered list of closed tagged segments:

- `{kind:"field",name:...}` selects an existing record field;
- `{kind:"index",index:...}` selects a list element, with unsigned integer
  index in `0..=2^32-1` (Boolean, negative, fractional and string indices fail);
- `{kind:"alternative",tag:...}` selects a declared closed-variant payload.

An empty path selects the root itself. Lists do not use string keys. Paths cannot
implicitly unwrap nullable values, follow Ref targets, traverse a private type,
apply functions, read a host location, or acquire a missing dependency. Callers
select a separately authorized public target explicitly instead of auto-deref.
For value mode, index bounds, selected alternative and non-null availability
are checked against the retained value. Type mode checks element/payload type
but cannot establish future index/tag/value availability. It returns an explicit
retained availability obligation for each such segment. An optional field also
retains presence obligations; nullable terminal values retain their nullable type.

`expected_type` must exactly match the resolved type, including nominal owner,
revision and wrappers; no structural duck typing or implicit conversion. Type
mode alone does not prove an operation produces this result contract: consumer
validation must check that association and output availability before binding.
It cannot turn a user-supplied `expected_type` into the authority for a lookup.

The canonical selector identity is the structural tuple
`["neutral.member-selector/1",mode,root,segments,resolved_type]`, where segments
are `["field",name]`, `["index",unsigned_integer]`, or `["alternative",tag]`.
Wrapper types become `["list",element]`, `["nullable",inner]`, or `["ref",target]`;
leaf IDs are the exact tagged tuples. Compare canonical typed tuples, not raw
JSON spelling, graph-local indices, aliases, source offsets or hash coincidence.
If selector records are stored as ordinary project values their validated fields
already participate in core identity; external selector requests belong to the
consumer request/plan context and cannot change complete project identity.
This does not amend historical identity transcripts or introduce a new digest.

## Entry discovery and public closure — V1-CONS-004, V1-CONS-008

Discovery request `neutral.entry-request/1` has `schema,contract,selection`.
`contract` is an exact public nominal type ID; a name or package version is
insufficient. Discovery enumerates all public binding symbols of exactly that
resolved type in stable symbol order. Record/type declarations are not entries.
Enumeration never executes them and never picks the first declaration.

Selection is `{kind:"enumerate"}`, `{kind:"unique"}`, or
`{kind:"explicit",roots:[symbol,...]}`. Unique selection requires exactly one
candidate; zero is MissingEntry, multiple is AmbiguousEntry. Explicit selection
requires distinct existing public bindings matching the exact contract; empty
roots yield an empty view, not all candidates. Roots sort canonically on success.
Private roots cannot be discovered or selected. Entry selection is post-compilation
and preserves complete project identity and captured closure.

Views retain selected public values, exact signatures, type/default/restriction/
variant contracts, vocabulary revisions/transitive dependencies, reference targets,
and all interpretive dependencies of selected selectors/consumer data. Redact
private implementation identities, source IDs and spans; do not lose intentionally
exported materialized values through private reuse. Missing public interpretive
closure is an error, not a redacted partial success or implicit network lookup.

## Value attribution — V1-CONS-006

Request `neutral.value-origin/1` has `schema,symbol,path`, using a public binding
and the same value-mode path grammar. The reader resolves each segment against
validated values and companion ownership before exposing a location. Invalid
path, ownership, UTF-8 byte range or value kind fails; never return an invented
declaration-wide span as a precise field/list/reference occurrence.

An origin record contains `{symbol,path,presence,occurrence,origin}`. Presence is
`supplied`, `omitted`, `explicit-null`, or `defaulted` under the existing composition
contract. Occurrence is either `{state:"available",source_digest,start,end}` or
`{state:"unavailable",reason}`. Reasons are `redacted`, `not-retained`, or
`not-source-origin`. Origin is `supplied`, `reuse` with separately checked origin
location, or `default` with canonical owner/type/field and optional checked
vocabulary location. All variants are closed; unavailable locations contain no
hidden source IDs, digests, spans or private owner hints.

Source/vocabulary locations use exact recorded digest ownership and half-open
original-byte offsets. Bounds use retained original byte lengths. Without source
bytes, the independent reader proves structural ownership/range/coverage only;
it cannot prove a producer's span corresponds to truthful source text. If the
caller supplies bytes for stronger checking, verify their exact digest first.
Original-byte attribution is not a host path or a claim of producer authenticity.
For omissions use the checked omission occurrence or an explicit unavailable
state, never a fabricated value span. Materialized default meaning and its
vocabulary owner are separate from the user's source omission evidence.

Public views may expose safe public occurrences and public contract owners only.
Reuse from a private declaration keeps the public use-site occurrence but redacts
its private origin. Changing source formatting or attribution never changes
logical meaning; the governed evidence/derivation layers retain relevant changes.

## Self-contained artifacts and compatibility — V1-CONS-009, V1-CONS-010

Protocol compatibility discovery returns exact supported tuples, feature sets,
and availability independently of Cargo/package versions. A consumer request
requires `neutral.consumer/1` and sorted unique features
`entry-discovery-v1`, `member-selectors-v1`, `value-attribution-v1`.
Only advertise a feature after its public implementation passes integration.
Requests may not reinterpret unknown required features as optional hints.
Authoring is optional and is negotiated separately; headless readers do not
require an Editor profile. Presentation metadata is not a semantic dependency.

| Producer artifact/profile | Reader capability | Consumer /1 behavior |
| --- | --- | --- |
| Legacy `NIR-CBOR/0.1` | Legacy document reader | Legacy reading unchanged; consumer /1 rejects Unsupported |
| `neutral.project-ir/1`, `NIR-PROJECT-CBOR/1`, identity /1 | Project /1 reader | Existing reading unchanged; consumer /1 rejects Unsupported |
| `neutral.project-ir/2`, `NIR-PROJECT-CBOR/2`, identity /2, source `1.0`, complete composition features | Project /2 + consumer /1 + all required consumer features | Structural validation, then selectors/entries/attribution allowed |
| Same /2 artifact | Project /2 reader without consumer /1 | Existing project reading allowed; consumer requests reject Unsupported |
| Same /2 artifact | Consumer /1 missing a required consumer feature | Reject consumer request before public result publication |
| Unknown IR/wire/identity or required project feature | Any reader lacking exact support | Reject artifact before accepted data/view publication; no fallback |

Semantic bundle schema `2.0` and explicitly adapted legacy `1.0` leaves retain
their frozen composition rules. Self-contained artifacts carry all referenced
contract facts and validated companions/resources; reader-only requests need no
compiler linkage, reparsing, ambient schema registry, backend code or acquisition.
Absent source evidence yields explicit unavailable attribution where allowed;
absent required type/default/restriction/dependency facts is rejection, not an
invitation to fetch schemas. Existing wire /2 is unchanged: protocol requests
operate over its validated facts and ordinary typed values. Any future artifact
extension requires a separately frozen schema/encoding/identity profile.

## Diagnostics and independent bounds

The reader-contract registry is separate from compiler and consumer codes:

| Code | Class | Meaning |
| --- | --- | --- |
| `NEU-CONS-001` | Unsupported | Unknown/unavailable profile tuple or required feature |
| `NEU-CONS-002` | InvalidInput | Malformed/duplicate/unknown request member or segment |
| `NEU-CONS-003` | MissingMember | Missing public root, field, index or alternative |
| `NEU-CONS-004` | TypeMismatch | Expected/resolved type differs or illegal path traversal |
| `NEU-CONS-005` | PrivateTarget | Explicit target is not public |
| `NEU-CONS-006` | MissingEntry | No entry matches unique selection |
| `NEU-CONS-007` | AmbiguousEntry | Multiple entries match unique selection |
| `NEU-CONS-008` | InvalidAttribution | Invalid companion ownership/path/span |
| `NEU-CONS-009` | IncompleteContract | Missing required interpretive dependency |
| `NEU-CONS-010` | Limit | Exceeded/invalid controls, allocation or arithmetic failure |
| `NEU-CONS-011` | Cancelled | Cooperative cancellation before publication |

Check envelope/profile, controls, already-cancelled state, complete reader
validation, public targets, path/type checks, then safe attribution/public closure.
Private explicit targets report PrivateTarget without disclosing their types or
locations. At later checkpoints cancellation/faults terminate atomically. Sort
diagnostics by phase, canonical public subject, typed path, then code. Consumer
cycle, output-availability and backend-capability failures use consumer-owned
codes/phase, never this registry or compiler codes. No failure returns partial
accepted values, views, locations, descriptors or a replaced cache generation.

All operations take positive independent controls, intersecting existing producer,
reader and hard limits. Additional ceilings are 67,108,864 request/report bytes,
1,000,000 selectors/roots/candidates/locations/closure edges each, 64 path segments,
1,048,576 bytes per identifier/text, 1,000,000 diagnostics, and 100,000,000 work
visits/emitted bytes. Each collection has its own control; one does not substitute
for another. Count before retention; exact fits succeed, one over fails before
proportional allocation. Cycles use bounded visited sets. Cooperative cancellation
and fallible allocation apply through lookup, closure, mappings and report output.

## Literal oracles and activation

[Boundary requests](../fixtures/consumer/boundary.json) and
[oracles](../../conformance/oracles/consumer/boundary.json) freeze selectors,
entry selection, attribution/privacy, self-contained closure and compatibility
outcomes. Fixture records are public-boundary facts to be assembled and checked
by the future harness, not forged accepted artifacts or passing decoder evidence.
Registration and integrity checks alone do not implement this protocol. The
broader source/vocabulary operation graph and malformed/limit corpus remain a
separate registration gate; independent production and authoring comparisons
must pass before required-active status or V1-CONS implementation checkmarks.
