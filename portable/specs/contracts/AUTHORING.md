<!-- SPDX-License-Identifier: Apache-2.0 -->

# Authoring bridge contract

Status: frozen foundation contract, 12-10-2026; implementation not activated

The authoring bridge is a profile versioned separately from Neutral core. An
Editor requiring generic visual authoring needs a compatible core and authoring
profile; headless consumers need only core unless they request authoring.

Discovery takes exact core profile, semantic vocabulary locks, and separate
authoring metadata and returns a deterministic static descriptor catalogue.
The catalogue declares source-authorable constructs/types/value forms,
properties, ports, constraints, presentation hints, formatting capabilities,
and explicit exclusions. It is data, not executable plugins.

For a captured project, a revision-bound overlay provides local/imported types,
symbols, aliases, actions, comments, mappings, and exact required vocabulary
identities. The editable model is closed and bounded. Connections are the sole
representation for ordinary reuse/reference edges; they do not encode execution
or ordering.

Projection deterministically writes ordinary `.neu` source and mappings. The
compiler compiles that source and is authoritative on mismatch. Formatting and
diagnostic presentation are authoring concerns. The bridge never exposes a
compiler-private AST, grants acquisition, or assigns Flow/CI/CD semantics.

## Foundation selection and completion boundary

The foundation profile is `neutral.authoring/1`; its JSON fixture envelope is
`neutral.authoring-fixture/1`. Exact selection requires core source profile
`1.0`, capture `neutral.capture/v2`, IR `neutral.project-ir/2`, and the captured
composition feature set. Headless compilation does not require this profile.
The foundation adds no expression, template, selector, runtime, or acquisition
syntax. V1-CONS-001..016 and their authoring extensions remain separate unchecked
gates. A foundation implementation must reject those required features until
their separately frozen compatibility tuples are implemented.

Freezing this contract and registering its literal oracles completes only the
first two Stage 8 contract/fixture tasks, not V1-AUTH-001..006 implementation.
Package versions, source spelling, and unknown extension members cannot select
or silently extend a profile. The public availability response distinguishes a
recognized-but-unimplemented tuple from an implemented tuple. It never advertises
an unavailable capability as usable. Unknown versions/features fail before
constructing descriptors or editable output; no fallback is attempted.

## Closed boundary and operations

Requests and results are immutable data. No call accepts an I/O provider,
resolver, executable validator, plugin, environment lookup, or editor callback.
All project input bytes, semantic locks, and presentation metadata are supplied
by the host. JSON transport rejects duplicate keys, unknown members, invalid
UTF-8, invalid enums, and incorrect member types before publication. An in-process
API must enforce the same checks and bounds; JSON is an interchange/test form,
not a requirement to serialize internal calls.

The operation sequence is:

1. `discover`: exact profile tuple + already validated complete vocabulary
   catalogue + separate metadata + controls -> static descriptors.
2. `open`: descriptors + complete immutable validated project + its original
   captured sources -> revision-bound overlay and source-preserving model.
3. `project`: base revision + edited model + the same descriptors/overlay +
   controls -> complete source buffers and mappings, never filesystem writes.
4. `format`: original captured source buffers + controls -> formatted buffers
   and mappings; invalid input fails without partially formatted output.
5. `verify`: projected buffers compiled under the exact core tuple, then
   independently read and compared with intended typed meaning. Compiler
   rejection or a mismatch fails the authoring request; it never publishes IR
   labelled successful merely because preflight passed.

All operations return either complete success or a failure with stable code and
bounded diagnostics. Failed edits/cancellation cannot replace the last accepted
model, overlay, or project generation. The caller explicitly adopts a successful
result; opening a project never mutates compiler or shared global state.

## Static catalogue — V1-AUTH-001, V1-AUTH-002

The catalogue has exact profile tuple, sorted required features, type descriptors,
construct descriptors, format capabilities, and explicit exclusions. Foundation
construct kinds are module/import/use, record/variant declaration, immutable
binding, scalar/list/record/variant literal, typed null, reuse, and identity ref.
No catalogue entry may enable a form absent from the governing core profile.

A descriptor contains canonical `id`, `kind`, source-authorable/public status,
ordered properties, typed input/output ports, semantic contract facts, and
separate presentation hints. Type identifiers are tagged tuples:

- core leaf: `["core", name]`, with `name` exactly `num`, `string`, `bool`,
  `url`, or `path`;
- source nominal: `["source", logical_module, type_name]` (overlay only);
- vocabulary nominal: `["vocabulary", identity, revision, type_name]`.

Composite types use `{ "kind": "list", "element": Type }`,
`{ "kind": "nullable", "inner": Type }`, or
`{ "kind": "ref", "target": Type }`. Nominal IDs are leaves, not recursive
inline copies. Record fields and variant alternatives retain the exact public
resolved contract, presence/default/restriction facts, and dependency IDs from
the independent reader. Core primitive constructors have fixed descriptor kinds;
vocabulary types are discovered from supplied facts, never hardcoded palettes.
No semantic fact is inferred from a label, port color, source alias, or metadata.

Descriptor identity order is lexicographic over tuple components as exact UTF-8;
properties/ports/fields/tags sort by their exact names. Features and dependency
IDs are sorted unique sets. Input bundle order and JSON object member order have
no effect. Two source aliases of one exact bundle do not duplicate its descriptor.
Conflicting revisions, duplicate canonical descriptors, private exposed types,
and invalid default/restriction contracts fail before discovery success.

## Separate presentation metadata — V1-AUTH-003

Metadata schema `neutral.authoring-metadata/1` contains exactly `schema` and
`entries`. Each entry contains `target` (canonical descriptor ID), and optional
`label`, `description`, and `group` (plain Unicode text). An entry must have at
least one hint. Targets are unique and must exist in the supplied catalogue.
Source-local metadata may target overlay types only when opening a project.
An empty entries list is valid. Metadata cannot contain defaults, constraints,
types, source snippets, scripts, event handlers, URLs for loading assets, or
executable markup. Render hints as escaped text. Unknown fields fail, including
attempts to override semantic facts. No acquisition is implied by text content.

Metadata changes invalidate authoring descriptors/overlay revision, but cannot
change captured semantic inputs, emitted source for an otherwise unchanged
model, logical IR, or any existing semantic identity. Metadata has its own
bounded bytes/entry/text limits; it is never inserted into semantic bundles.

## Project overlay and stale revisions

The overlay contains source-local/imported nominal descriptors, symbols, exact
alias bindings, available edit actions, retained comments, and source mappings.
It is built from the complete validated project plus original source buffers,
including private/disconnected modules for a caller authorized to edit the
complete project. Public reader views alone cannot reconstruct private source.
Missing or redacted source cannot be invented: `open` fails as not editable.

Every overlay/model carries a revision tuple with `captured_identity`,
`logical_identity`, exact `profiles`, and `metadata_digest`. Identities use the
governing complete-project /2 identity contract. `metadata_digest` is SHA-256 of
the supplied metadata's exact UTF-8 bytes (including formatting); no metadata
is represented by the digest of the empty byte sequence. Presentation-only
revision changes are intentional and remain outside project identity.
The originating immutable project/catalogue must be retained and validated;
user-supplied digest labels alone cannot authorize a forged snapshot.

Every edit/projection checks exact revision equality before model traversal.
Source/comment/alias edits change captured identity even when logical meaning
does not. Changing metadata, profiles, or semantic locks also invalidates the
overlay. Stale requests return `NEU-AUTH-003`; there is no implicit merge,
automatic rebase, or overwrite of a newer generation. The host may explicitly
open a new snapshot and reapply its edit after review.

## Closed editable graph — V1-AUTH-004

Model schema `neutral.authoring-model/1` has exactly `schema`, `units`, and
`connections`; the revision is carried separately by the request. Each unit has
`id`, `source_id`, `module`, `imports`, `uses`, `declarations`, and `comments`.
Unit/source/module IDs are unique. Imports have `module` and `alias`; uses have
`identity`, `revision`, and `alias`. They must agree with the supplied project
closure and locks. Logical imports never become paths or acquisition requests.

Declarations have `id`, `kind`, `name`, `public`; `kind` is `record`, `variant`,
or `binding`. Records add `fields` of `{name,type,presence,restrictions}` plus
`default` only for defaulted fields. Variants add `alternatives` of `{tag,type}`.
Bindings add `type` and `value`. These retain the source/core contract; the model
cannot express a vocabulary-only restriction as new source syntax.

Every value node has unique `id` and `kind`. Scalar kinds carry `value` (exact
numeric text, Unicode string/location text, or Boolean as appropriate); `null`
has no payload; `list` has `items`; `record` has `fields` of `{name,value}`;
`variant` has `tag` and `payload`. An `input` node has no payload and must have
exactly one incoming connection. A connection has `kind` (`reuse` or `ref`),
`target` (declaration ID), and `input` (input node ID). Whole-declaration targets
only: typed member selectors are a separately scheduled extension. No other
node may have an incoming connection. Connections carry identity/data meaning,
never scheduling. All IDs are nonempty opaque model-local strings; labels and
array offsets cannot resolve a connection. No raw source/expression escape hatch.

Validate globally unique declaration/value IDs, existing endpoints, exact port
types/visibility, one connection per input, complete required fields and valid
presence/defaults, then core cycle/public-closure rules. Reuse is by value;
`ref` preserves identity without recursive value expansion. Cycles legal for
typed refs remain legal; value-reuse cycles fail. Display coordinates are host UI
state outside this model, semantic identity, and projection.

Comments have `anchor` (unit/declaration/value ID), `placement` (`before` or
`after`), and `text` (original valid line/block comment bytes without delimiters
being changed). Same-anchor comments retain array order. Dangling anchors fail;
comments cannot be reinterpreted as nodes. Text is retained, never executed.

## Projection, formatting, and mappings — V1-AUTH-005

Projection returns a complete ordered set of `{source_id,module,source_utf8}`
buffers; no paths. Units sort by module UTF-8; import/use lists sort by logical
target/alias; original declaration/field/value-list order is preserved within
each unit. Moving a declaration is a deliberate authoring edit, not a semantic
sort. Emission is LF, four spaces per indentation level, no trailing whitespace,
and exactly one final LF. Separate the header/import/use block from the first
declaration with one blank line; declarations are otherwise consecutive unless
anchored comments intervene. Do not trim bytes inside literal/comment tokens.
Braces containing record declarations/initializers or
variants are multiline; nonempty members have trailing commas. Empty aggregates
stay compact. Lists stay inline unless they contain multiline aggregate values.
Strings preserve decoded scalar values using canonical escapes for quote,
backslash, LF, CR, tab and controls; other Unicode scalars are emitted literally.
Exact numbers use the governing canonical numeric representation without rounding.
Projection cannot rename bindings, rewrite aliases, normalize inert locations,
insert external dependencies, remove unused/private declarations, or materialize
an omitted source field just because a semantic default exists.

Every emitted declaration/value and reuse/ref occurrence has a mapping record
`{model_id,source_id,start,end,role}`. Spans are checked half-open original UTF-8
byte offsets in the **returned** buffer, not character indices or old-source
offsets; roles are `declaration`, `value`, or `connection`. Sort mappings by
`(source_id,start,end,role,model_id)`. Declaration spans include the declaration
through its closing literal/type token, excluding following whitespace/comments.
Value spans cover the emitted literal or reuse/ref expression; connection spans
cover that whole reuse/ref expression and use its input node ID as `model_id`.
Empty results are valid only when the model/core contract permits them.

Formatting uses the same emission rules and preserves all source constructs,
decoded values, omissions, comments, aliases, and source IDs. Reformatting is
idempotent. Format-only changes can change captured identity and invalidate old
byte mappings, never logical meaning. Return fresh mappings for the new buffers.
`open -> project -> compile -> open -> project` produces byte-identical projected
source after the first canonical projection and equal logical IR/identity.
The first canonical projection need not equal noncanonical original source.
Byte-for-byte lossless concrete whitespace editing is not promised.

The source-preserving model is not reconstructed from semantic IR alone. Future
template/expression extensions must retain their editable source forms separately
and undergo their own freeze; /1 cannot silently flatten them as ordinary values.

## Diagnostic registry and compiler authority

| Code | Class | Meaning |
| --- | --- | --- |
| `NEU-AUTH-001` | Unsupported | Unknown/unavailable profile tuple or required feature |
| `NEU-AUTH-002` | InvalidInput | Malformed envelope, metadata, model, duplicate/unknown member or ID |
| `NEU-AUTH-003` | StaleRevision | Base revision differs from the retained current snapshot |
| `NEU-AUTH-004` | InvalidConnection | Dangling, duplicate, incompatible, private, or cyclic reuse connection |
| `NEU-AUTH-005` | NotEditable | Missing/redacted original source or unrepresentable required construct |
| `NEU-AUTH-006` | Limit | Invalid/exceeded control, allocation, or checked-arithmetic failure |
| `NEU-AUTH-007` | Cancelled | Cooperative cancellation before publication |
| `NEU-AUTH-008` | CompilerRejected | Projected source rejected by the selected compiler profile |
| `NEU-AUTH-009` | Mismatch | Compiler/reader accepted source but its typed meaning differs from the model |

Preflight order: envelope/profile, controls, already-cancelled request, revision,
metadata/catalogue integrity, model shape/endpoints, typing/public closure,
projection/mapping bounds, then compile/read/compare. Cancellation is observed
at subsequent bounded checkpoints and before publication. Limit/allocation faults
terminate immediately. Within a phase diagnostics order by
`(unit_id,model_id,field_path,code)` using UTF-8 ordering and numeric list indices.
At the diagnostic cap fail with Limit, not truncated success. No partial model,
buffers, mappings, or IR accompany failure. Compiler diagnostics retain their
original stable codes and checked projected-source locations under
CompilerRejected; Mismatch is not a replacement compiler diagnostic.

Diagnostics identify model IDs/field paths and checked source spans where
available, never host paths, raw source excerpts, secret material, or private
facts redacted by the governing view. Missing location is explicit. This
authoring registry does not renumber core diagnostics or change core recovery.

## Independent controls and limits

Every operation receives positive integer controls; there are no implicit
unbounded defaults. Test cases may override individual controls below these
hard ceilings. Count requests before allocation; exact fits succeed, one over
fails before proportional retention. All controls intersect existing core,
composition, reader and capture limits, never replace or loosen them.

| Control | Counted quantity | Hard ceiling |
| --- | --- | --- |
| `input_bytes` | All supplied source, metadata and model UTF-8 bytes | 67,108,864 |
| `descriptors` | Core plus unique canonical vocabulary/source descriptors | 1,000,000 |
| `properties` | All descriptor fields/alternatives/ports retained | 1,000,000 |
| `units` | Complete supplied source units | 1,000,000 |
| `nodes` | Units, declarations and value nodes, each once | 1,000,000 |
| `connections` | All reuse/ref edges | 1,000,000 |
| `depth` | Value/composite nesting; leaf at zero | 64 |
| `metadata_entries` | Separate presentation entries | 1,000,000 |
| `text_bytes` | Any one retained identifier/comment/hint/string | 1,048,576 |
| `mappings` | Emitted mapping records | 1,000,000 |
| `output_bytes` | Sum of emitted source UTF-8 buffer lengths | 67,108,864 |
| `diagnostics` | All retained diagnostics | 1,000,000 |
| `work` | Input scalar visits, node/edge/descriptor visits and emitted bytes | 100,000,000 |

Repeated shared references do not bypass work or edge counts. Cyclic type graphs
use bounded visited/work sets, never recursive unbounded traversal. Check
cancellation before large copies and regularly during traversal/emission; use
fallible retention and publish atomically. Controls are request policy, not
semantic identity or catalogue types. No physical whole-process OOM guarantee
is implied by a bounded in-process foundation.

## Literal contract corpus

[Foundation requests](../fixtures/authoring/foundation.json) and
[literal oracles](../../conformance/oracles/authoring/foundation.json) cover
profile discovery, core/vocabulary catalogue, presentation separation, model
connections, stale metadata/source revisions, source projection/mappings,
format idempotence, round trips, compiler disagreement, and exact/one-over limits.
Inputs/oracles are pinned in the conformance registry and the authoring freeze
record. Contract-integrity checks do not pretend to exercise a production bridge;
the registered suite remains frozen, not required-active, until integration.
