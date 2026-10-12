<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression project compatibility

Status: approved and frozen, 12-10-2026; **not implemented or activated**.

This is the byte-level companion to [EXPRESSIONS](EXPRESSIONS.md),
[EVALUATOR](EVALUATOR.md) and [EXPANSION](EXPANSION.md). Their accepted semantic
rules are unchanged. Written approval covers this contract, its source amendment
and the registered [corpus](../fixtures/expressions/README.md). The acceptance
record and immutable byte pins are in [freeze.toml](freeze.toml).

## Exact selection and source amendment

| Boundary | Exact successor ID |
| --- | --- |
| Captured request | `neutral.capture/v3` |
| Complete logical IR / view / result | `neutral.project-ir/3`, `neutral.project-view/3`, `neutral.project-result/3` |
| Identity | `neutral.project-identity/3` |
| Binary transport | `NIR-PROJECT-CBOR/3` |
| Expression / evaluator / expansion | `neutral.expression/1`, `neutral.evaluator/1`, `neutral.expansion/1` |
| Authoring / authoring fixture | `neutral.authoring/2`, `neutral.authoring-fixture/2` |

Source remains `neu "1.0"`. Capture /3 requires the exact sorted feature set
`decimal-division-v1, finite-expansion-v1, pure-evaluator-v1, pure-expressions-v1,
tagged-variants-v1, vocabulary-composition-v2`. The fixed decimal context is
`neutral.decimal/1`: 34 significant digits, nearest/ties-to-even for
nonterminating division; terminating results remain exact within bounds.
Version/package names and source spelling never select an envelope.

Capture /3 extends the /2 request with exact semantic profile tuple and positive
expression/expansion controls. There is no feature-subset fallback or automatic
migration of an old artifact. This profile adds no vocabulary schema version:
closed schema 2.0 defaults/restrictions remain data, not expression programs.

**Proposed source exclusion amendment:** only an explicitly selected /3 request
admits pure value operators, guarded `if`/`match`, typed input/output leaves,
interpolation and compile-time template/matrix declarations. These are value
evaluation and finite description expansion, not imperative control flow.
Assignments after declaration, general functions, mutation, statement loops,
effects, ambient lookup, acquisition and executable vocabulary payloads remain
excluded. Unavailable profiles still reject before body parsing. Approval of this
amendment must not rewrite the historical [SOURCE](SOURCE.md) contract in place;
this versioned supplement is the amendment and takes effect only on /3 activation.
The v0 protected-name set is unchanged; new keywords/intrinsics are contextual
under explicit successor selection.

| Producer/input | Reader/authoring target | Outcome |
| --- | --- | --- |
| Old /1 or /2, no new syntax | Matching historical boundary | Unchanged |
| /1 or /2 with an expression/input/template | Historical boundary | Reject; never retry /3 |
| /3, exact required tuple | Implemented /3 reader | Validate complete closure before publication |
| /3 | /1 or /2 reader | Unsupported distinct magic/schema; no evaluation |
| /3 | Authoring /1 | Unsupported; no lossy editable model |
| /3 | Authoring /2 with all required descriptors | Preserve source structure and expanded semantic linkage |
| Unknown/missing/duplicate/out-of-order feature or profile | Any successor boundary | Unsupported/malformed before evaluation |
| Only source header or Cargo version changed | Any boundary | No capability activation |

Recognition is not availability. Until implemented, even a recognized successor
tuple reports unsupported/unavailable, never usable descriptors or partial IR.

## Canonical typed trees

Types, values, stable symbols, canonical collection order and visibility retain
the [composition /2](COMPOSITION-PROFILE.md) grammar. A project binding body is
exactly `["value", value]`, `["input"]`, or `["expression", node]`. A type
declaration has CBOR null body. Source-level ordinary reuse is materialized.
An external-input declaration has its binding signature but no fabricated value.

Every residual node is a definite array. Here `T` is the complete resolved type,
`S` a stable symbol, and children remain in evaluation order:

| Node | Exact tuple |
| --- | --- |
| Static value | `["value", T, value]` |
| External input | `["input", T, S]` |
| Future output | `["output", T, target_S, result_T, selector]` |
| Unary | `["unary", T, operator, child]` |
| Binary | `["binary", T, operator, left, right]` |
| Conditional | `["if", T, guard, selected_true, selected_false]` |
| Match | `["match", T, scrutinee, [[tag, payload_T, arm], ...]]` |
| Explicit conversion | `["text", ["string"], child]` |
| Interpolation | `["interpolate", ["string"], [string_node, ...]]` |
| List | `["list", ["List", element_T], [node, ...]]` |
| Record | `["record", nominal_T, [[field, node_or_omitted], ...]]` |
| Variant | `["variant", nominal_T, tag, payload_node]` |

Unary operators are exactly `!` and `-`; binary operators are exactly
`* / + - ++ < <= > >= == != && ||`. No host opcode or executable string is
permitted. Record fields and match arms sort by UTF-8 name/tag; list and operand
order never sort. `["omitted"]` occurs only in an optional record field.
`selector` uses the frozen Consumer/1 typed segments; the result association
remains independently validated consumer input, not core scheduling meaning.

**Binding representation decision for review:** match payloads and public
template parameters require lexical references absent from ordinary external-input
leaves. A parametric tree may contain `["bound", T, depth, slot]`: zero-based
lexical frame depth and declared parameter slot. A match arm pushes one frame
with payload slot zero; a template pushes its ordered parameter frame.
Depth/slot/type must resolve within the enclosing tree; an orphan bound node
rejects. This is a closed binding notation, not an external binding key or an
additional user-language feature. It cannot escape an arm/template as a root
value. Parameter names/binder spellings do not enter normalized tree identity.
Readers check binder scopes and types independently; template substitution is
capture-free, and substituted match trees keep their bound payload nodes.
Review this clarification with the frozen semantic contracts before approval.

Public template contracts are sorted `[symbol, parameters, result_T, body]`
tuples. Each parameter is `[name, static_or_deferred, T, default_node_or_null]`;
names are semantic because applications use named arguments. Earlier parameter
defaults use the bound notation above. Body/default trees resolve all nominal
owners and public constant references at definition scope; static public constants
are materialized and deferred public references use their canonical input/output
leaves. Template application recursion is rejected before lowering. Expanded
ordinary bindings retain no template invocation node. Only public templates
remain exported semantic contracts; private unused templates stay source evidence.

Restrictions remain part of complete nominal contracts, not caller assertions.
A decoder retypes every node, including skipped arms, checks all visible owners,
selectors, restrictions and guarded dependency closure, then validates companions.
A valid residual tree is not proof that external values satisfy its obligations.

For the candidate boundary cases, a scalar/input leaf has evaluation depth one;
each enclosing operator adds one. Evaluation node budget counts the complete
validated root tree before evaluation, including skipped branches. Numeric digit
budgets count each normalized coefficient; output text bytes count decoded UTF-8,
not Unicode scalars (semantic vocabulary length restrictions still count scalars).
Retained structural/envelope bytes have their own independent budgets.

## Limits, facts and attribution

Expression-limit order is `nodes, syntax-depth, evaluation-depth, bindings,
dependency-edges, work, retained-bytes, output-bytes, coefficient-digits,
absolute-scale, diagnostics`. Expansion-limit order is `definitions,
applications, parameters, axes, axis-items, product-tuples, aggregate-tuples,
expanded-bindings, expanded-nodes, depth, work, retained-bytes, output-bytes`.
All are positive u64 and intersect the accepted semantic and historical reader
hard ceilings. Distinct dimensions never borrow unused budget. Supplied numeric
coefficient digits and scale are checked before expansion; static cardinality
multiplication is checked before visiting/filtering or allocating products.

Recomputable successor facts, in order, are `residual-nodes, residual-depth,
input-declarations, output-leaves, potential-value-edges, public-templates,
retained-expression-text-bytes`. These describe retained trees, not producer
execution counts. Expansion visitation/folding work is derivation evidence;
the reader cannot prove it from normalized values. Limits enter derivation,
not logical identity. Cancellation/fault injection stays a validation task.
Retained trees include binding residuals and public template bodies/defaults.
Count every node occurrence (including repeated subtrees), with leaf depth one.
Retained text bytes sum UTF-8 lengths of every text item in those wire trees,
including kind/type/operator tags, symbols and numeric coefficients; array
framing, integers, source text and other wire sections do not contribute.
Public templates are counted separately from ordinary project declarations.

Expression attribution is a separate ordered collection
`[owner_symbol, node_path, occurrence_or_null, related_public_locations]`.
Node paths are child indices in canonical trees; source locations retain /2
checked byte-range grammar. Expanded declaration attribution additionally links
application, definition, argument and original axis-index tuple, never generated
fictional source bytes. Public views redact private origins and source identities.
The expansion tuple is `[owner_symbol, application_location, definition_location,
argument_locations, original_axis_indices]`; argument locations follow parameter
order and indices follow source axis order. An ordinary template application has
an empty axis-index tuple. Source-map owners may name public template definitions
as well as ordinary declarations. Expression paths address canonical child order;
the template body uses its template symbol as owner.
The reader checks all retained locations against source companions; no supplied
path or URL is opened to verify attribution.

Potential output dependencies are derived from typed trees with their guards,
not an authoritative independently supplied scheduling graph. Structural
references, guarded value edges and explicit prerequisite values remain distinct.
Views close over all interpretive types and all potential public dependencies,
including unselected branches; evaluation demands only selected branches.

## Successor wire envelope

Magic is exactly the eight bytes `NEUP3\r\n\x1a` (hex `4e455550330d0a1a`).
Follow it with one definite restricted-CBOR array of 22 positions. No maps,
floats, tags, indefinite lengths, nonminimal integers/lengths or trailing bytes.
All tuples have exact arity; unknown node/operator/body tags reject.

Positions 0–15 use the /2 layout, with these explicit /3 substitutions:
position 0 is `neutral.project-ir/3`; position 2 declarations are
`[symbol, public, signature, body_or_null]` using the body sum above;
position 5 is the recomputed interface /3 digest. Existing type/default value
tuples do not gain executable nodes.

| Position | Content |
| --- | --- |
| 16 | Exact profile tuple: `[expression, evaluator, expansion, decimal, required_features]` |
| 17 | Canonical public template contracts |
| 18 | Ordered expression-limit tuple |
| 19 | Ordered expansion-limit tuple |
| 20 | Ordered recomputable successor resource facts |
| 21 | `[expression_attributions, expansion_attributions]` |

The complete artifact must retain all existing source/map/provenance/vocabulary
companions. Position 8 structural edge ordinals retain /2 meaning; guarded value
dependencies come from trees, not a new interpretation of those ordinals.
Decode and validate all sections atomically before exposing roots or evaluator
requests. A node/vector fragment below is explicitly **not** a complete artifact.

## Identity /3 byte grammar

Use the unchanged NHT framing and SHA-256 from [PROJECT-IDENTITY](PROJECT-IDENTITY.md):
`F(tag,payload) = u16be(len(tag)) || ASCII(tag) || u64be(len(payload)) || payload`.
The outer frame remains `neutral-nht-v1`; the nested identity-profile frame
contains `neutral.project-identity/3`. New domains are
`neutral/project-{captured,logical,derivation,artifact,interface}/v3`.
Historical /1 and /2 domains, profiles and vectors are never extended.

The captured body uses /2 fields followed by `semantic-profiles`.
That frame contains, in order, `expression`, `evaluator`, `expansion`,
`decimal` UTF-8 frames. The /2 features frame contains the full /3 feature set.
Source/bundle evidence remains exact; source IDs, not host paths, are retained.

The logical body uses the eight /2 sections with schema /3; each binding
declaration's `value` frame now contains a `constant` frame wrapping its
canonical /2 value, an empty `external-input` frame, or an `expression` frame
wrapping the typed tree framing below. Type declarations retain `absent`.
Append `semantic-profiles`, `required-features` (sorted `feature` frames),
and `public-templates` (sorted `template` frames).
A template contains its symbol, `parameters` with `parameter` frames
(`name`, `phase`, `type`, `default`), `result-type`, and `body`.
Type/default/body wrappers contain their canonical type/tree or empty `absent`.

Tree framing is an `expression` frame containing `kind` UTF-8, `type`
wrapping the /2 type frame, followed by the following fields in order:

| Kind | Fields after kind/type |
| --- | --- |
| value | `value` wrapping canonical /2 value |
| input | `symbol` wrapping canonical symbol frames |
| output | `target` symbol, `result-type` type, `selector` |
| unary | `operator` UTF-8, `operand` tree |
| binary | `operator` UTF-8, `left` tree, `right` tree |
| if | `guard` tree, `then` tree, `else` tree |
| match | `scrutinee` tree, `arms` containing sorted `arm` frames (`tag`, `payload-type`, `body`) |
| text | `operand` tree |
| interpolate/list | `items` containing ordered `item` tree frames |
| record | `fields` containing sorted `field` frames (`name`, `value` tree or omitted) |
| variant | `tag` UTF-8, `payload` tree |
| bound | `depth`, `slot` unsigned eight-byte integers |

A selector contains ordered `segment` frames: `kind` (field/index/alternative)
followed by `name` UTF-8 or `index` u64. There is no implicit dereference.
Stable symbols and number normalization retain /2 framing, with scale interpreted
as signed exponent: value = sign × coefficient × 10^scale.

Interface /3 uses the same logical grammar with only public declarations and
their public interpretive closure, no private origins. Public templates are
included; all structural section frames remain present even when empty.
Residual expression order is semantic; aliases, binder spelling, source byte
evidence, roots and external binding values are not logical inputs.

Derivation /3 uses /2 grammar followed by semantic-profiles, expression-limits
and expansion-limits, each ordered as above. Artifact /3 retains /2 grammar
under its new domain; format is exactly the selected representation contract
(e.g. `NIR-PROJECT-CBOR/3`). Root selection changes artifact identity only,
never complete logical/captured identity. Source-preserving authoring has its
own revision keys, not a second project meaning.

## Authoring /2

Authoring /2 requires capture/IR/identity /3 and all semantic profiles/features.
It extends the foundation operation/result/error/limit protocol with explicit
expression trees, scoped binder IDs, input/output ports, template definitions,
named applications, matrix axes/filters/exclusions and expansion mappings.
The model retains source order/comments/aliases and stable editor IDs separately
from normalized semantic trees. Scoped bindings have one representation;
ordinary reuse/ref connections remain foundation connections. No opaque embedded
expression strings or backend command widgets substitute for typed descriptors.

Projection preserves type/phase and binder scope, formats ordinary successor
source, and compiles under the exact selected tuple. Expanded values cannot
reconstruct an original template or matrix; source-authoring round-trip and
semantic expanded equivalence are separate comparisons. Public views expose only
public origins. Foundation /1 remains unchanged and cannot load this model.

### Closed model extensions

`neutral.authoring-model/2` retains the foundation root `{schema, units,
connections}` and unit fields. Core type descriptors remain `["core", name]`.
The following additional declaration objects have exactly these fields:

| Kind | Fields |
| --- | --- |
| `external-input` | `id, kind, name, public, type` |
| `template` | `id, kind, name, public, parameters, result_type, body` |

Each parameter is `{id, name, phase, type, default}`; phase is `static` or
`deferred`, and absent default is JSON null, distinct from a typed null node.
Ordinary bindings retain `{id, kind, name, public, type, value}`. New expression
objects have common fields `id, kind, type` and these exact additional fields:

| Kind | Additional fields |
| --- | --- |
| `external` | `target` (external-input declaration ID) |
| `bound` | `binder` (lexically enclosing parameter/arm ID) |
| `binary` | `operator, left, right` |
| `unary` | `operator, operand` |
| `if` | `guard, then, else` |
| `instantiate` | `target, arguments` |

Named arguments are `{name, value}` in parameter order. Foundation literal
nodes retain their existing fields. The foundation `input` connection node
still denotes reuse/ref wiring, never an external input. IDs are editor-local,
globally unique, and absent from logical identities. Resolve every target and
binder in scope before projection; reject unknown fields and stale revisions.
The literal projection vectors cover these closed extensions, not a complete
implementation of the later authoring service. Match, construction and matrix
editable object shapes still require registration before that service is built.

## Literal review and activation

The frozen suite includes source files, vocabulary bytes, request controls,
literal results/errors, exact/one-over cases, rounding, guards, template hygiene,
filtered cardinality and compatibility cases. Independent result vectors use
integer-rational arithmetic, never a host float. Literal transcript vectors
carry explicit frame trees, byte strings and digests; wire-node vectors carry
resolved tuples and bytes. Their scopes are named: fragment vectors cannot be
claimed as full-project transport or identity evidence.

Written approval does not establish production implementation or conformance.
Before activation, extend the minimal complete-project /3 partition/artifact
vectors to all registered semantic/evidence variations, compare
the unchanged literals through production folding, the shared evaluator and an
independent interpreter, and run authoring/reader/hostile/fault/clean-cache suites.
Registration or this independent oracle self-check cannot activate syntax.

