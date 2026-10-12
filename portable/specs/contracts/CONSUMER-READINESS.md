<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vocabulary composition and consumer readiness

Status: composition and consumer /1 contracts/oracles frozen; consumer implementation pending; pure-expression extension unfrozen

This supplement schedules language infrastructure for `v0.8.0` and `v0.9.0`.
It does not activate syntax, change an accepted encoding, or introduce an
executor. Implementation follows reviewed schema, compatibility, diagnostic,
fixture, and identity-vector decisions in the implementation checklist.

## Ownership

Neutral owns bounded structural validation, typed data, exact vocabulary locks,
complete IR, reader views, identity, source attribution, and authoring projection.
The approved Stage 8 extension also requires a shared, compiler-independent pure
evaluator for static folding and deferred values. Independent consumers own
binding acquisition, scheduling, backend selection, mapping implementation,
authorization, and effects; they invoke the evaluator with explicit typed inputs.

The approved Stage 8 extension V1-CONS-011..016 schedules bounded pure
expressions and finite composition. It is not activated or contract-frozen:
syntax, profile/feature selection, numeric rules, diagnostics, wire forms,
identity vectors, and amendments to the current SOURCE exclusions must be
reviewed before implementation. Existing profiles and vectors are immutable.
Neutral evaluates statically known pure expressions under those frozen rules;
it never resolves external inputs or operation outputs during compilation.
Later, the same pure semantics evaluate explicitly supplied consumer bindings.
This adds value evaluation, not an operation executor or an ambient runtime.

Vocabularies remain closed data. Neither a semantic bundle nor presentation
metadata can load code, run a validator callback, grant privileges, or resolve a
credential. References are identity edges, not implicit execution prerequisites.
Module SCC acceptance never implies acceptance of an execution or expression cycle.

## `v0.8.0` — vocabulary composition

The Stage 7 extension gate must close these requirements before promotion:

The [composition contract](VOCABULARY-COMPOSITION.md) specifies concrete JSON
member sets, presence/default/restriction semantics, and dependency closure.
[COMPOSITION-PROFILE](COMPOSITION-PROFILE.md) and pinned literal requests/vectors
complete design and registration; the full project extension is not activated.

- **V1-VOC-005:** A newly versioned bundle schema supports bounded composition
  of scalar/nominal field types with lists, nullable values, and typed references.
  Freeze admissible nesting, recursive-type rules, public closure, target typing,
  and each independent limit. Match source semantics; do not invent a parallel
  type system inside JSON.
- **V1-VOC-006:** Support closed, data-only field defaults and declarative
  restrictions: finite scalar choices, numeric bounds, and string/list length
  bounds. Freeze exact comparison, Unicode length units, empty/null behavior,
  contradictory restrictions, and reference/default rules. Defaults satisfy the
  same restrictions as supplied values. No regex engine, script, callback,
  network validator, or consumer-specific constraint enters the compiler.
- **V1-VOC-007:** Preserve the existing project bundle schema and frozen v0
  behavior. New forms require exact schema/feature selection and matching lock
  coverage; unknown forms fail closed. Semantic defaults/restrictions belong in
  semantic locks and identities, never in authoring metadata. Carry enough
  contract data in IR for independent reader revalidation and view closure.
- **V1-VOC-008:** Review canonical forms and literal vectors for new type and
  value shapes across captured, logical, derivation, and artifact identities.
  Never reinterpret an existing identity profile or encoding. Freeze any needed
  new profile/version before publishing artifacts; retain all old vectors.
- **V1-VOC-009:** Support both source-declared and vocabulary-declared closed
  tagged variants for heterogeneous typed values and collections, using one
  resolved semantic model. See the [frozen variant contract](VARIANTS.md).
  Freeze source forms, nominal alternative identity, tag and
  payload typing, admissible nesting, public closure, and structural limits.
  Missing/duplicate/unknown tags, wrong payloads, and invalid field combinations
  fail closed. This is not open inheritance, implicit structural subtyping, or
  an untyped argument bag. Carry variants through all reader/authoring/identity
  boundaries under explicitly reviewed schema/profile versions.
- **V1-VOC-010:** Support explicit cross-vocabulary public type dependencies
  using canonical identities and exactly locked revisions. Hosts supply the
  entire bounded transitive bundle closure before capture; compilation never
  fetches dependencies. Freeze dependency syntax, exact lock-cover accounting,
  diamond/conflicting revision handling, cycle rules, unused dependency rules,
  and public-type closure. Source aliases cannot determine dependency identity.
  Preserve existing schema behavior instead of silently changing its lock rules.
- **V1-VOC-011:** Distinguish required input, omitted optional input, explicit
  null, and materialized default in the new schema and public provenance API.
  Freeze omission/null/default compatibility and expose safe value-origin facts
  without inventing new states in old artifacts. Deferred inputs are explicitly
  typed data descriptions, never encoded as missing/null or an unevaluated string.
  Source omission evidence belongs in companions, not logical identity when
  materialized meaning is equivalent; semantic defaults remain identity inputs.

Required fixtures include nested/empty lists, nullable values, typed references,
defaults, finite choices, exact/one-over limits, malformed forms, private-type
leaks, invalid reference targets, invalid defaults, and changed restrictions.
Source -> compiler -> IR -> wire -> independent reader -> probe must agree.
Alias, declaration-order, and presentation changes remain non-semantic; changing
semantic restrictions or defaults must change the governing semantic identity.

Also require mixed-variant lists, wrong-tag/payload failures, cross-bundle
references, transitive/diamond closures, missing/private/cyclic/conflicting
dependencies, and omitted/null/default distinction fixtures. Verify both old and
new bundles, independent decoding, view closure, literal identity vectors, and
exact/one-over dependency/type limits. No historical gate claims these additions.

## `v0.9.0` — consumer-facing data and authoring

Stage 8 must close these requirements alongside the authoring bridge:

[CONSUMER-PROFILE](CONSUMER-PROFILE.md) freezes the concrete V1-CONS-001..010
ownership/protocol boundary, member paths, source attribution, entry discovery,
closed reports and exact compatibility table, with 32 pinned literal oracles.
It does not activate implementation. The separate
[example convention](EXAMPLE-CONSUMER.md) now registers 31 literal graph/map/
condition/capability/privacy cases with file-backed sources and captured vocabulary;
consumer implementation and unchanged-oracle comparisons remain pending.

- **V1-CONS-001:** Publish project-neutral, file-backed examples of typed
  operation descriptions, arguments/results, reusable resources, explicit
  prerequisites, and output connections using ordinary vocabulary records and
  references. Stable contract/type/revision identities distinguish meanings;
  source aliases and backend command names cannot substitute for them.
- **V1-CONS-002:** Publish bounded data conventions for conditions and deferred
  inputs, including operation-output references, literal values, and symbolic
  credential handles. Conditions are structured typed records, not shell or
  embedded backend expression strings. Freeze their representational types and
  interpreter boundary; consumer condition interpretation, output availability, scheduling cycles,
  failure policies, retries, timeout behavior, and desired-state/idempotence
  semantics remain consumer responsibilities. This data boundary alone grants
  no computation syntax; V1-CONS-011..016 separately schedule pure expressions
  and bounded composition. General functions, unbounded loops, mutation, secret
  scalars, built-in maps, and execution syntax remain excluded.
- **V1-CONS-003:** Enable closed, versioned data envelopes for externally
  supplied capability/mapping descriptions and consumer validation reports.
  Identify contract revision, adapter revision, supported/unsupported capability,
  and explicit alternatives. Never silently drop unsupported meaning. This is
  generic typed transport boundary, not a universal capability taxonomy,
  product mapping protocol, or backend adapter implementation. Consumers own
  their envelope schemas and interpretation. External plan/cache
  keys must bind logical identity plus mapping revision, target configuration,
  and relevant bound inputs; those choices must not modify Neutral project
  identity. No raw credentials appear in fixtures or public reports.
- **V1-CONS-004:** Reader-only consumers can locate operation values, result
  contracts, prerequisites, and output/reference targets and retain their public
  interpretive closure. Consumer reports can identify the originating declaration
  and relevant source occurrence through existing public source-map/provenance
  contracts without accessing private ASTs or exposing private implementation
  details. Define missing/redacted-location behavior explicitly.
- **V1-CONS-005:** The descriptor catalogue, overlay, editable model, and source
  projection cover all new composite types, semantic restrictions, defaults,
  and data connections. Presentation hints cannot weaken semantic restrictions
  or evaluate conditions. Generic authoring and standalone reader-only probes
  agree with compilation, formatting, encoding, reopening, and no-op identity.
- **V1-CONS-006:** Expose bounded field/list-element value-path source attribution
  through public reader APIs, including reference occurrences, supplied values,
  and materialized default/reuse origins. Reports support stable codes, primary
  and related safe locations, and explicit unavailable/redacted attribution.
  Reject invalid paths/spans and retain cancellation/size bounds. Complete
  declaration spans alone do not satisfy precise argument/connection diagnostics.
- **V1-CONS-007:** Provide a versioned typed member-selector representation for
  references to particular contract fields/results rather than only whole
  declarations. Freeze target identity, path grammar, result typing, visibility,
  indexing rules, limits, source attribution, and canonical identity. Validate
  statically knowable member existence/type without evaluating a future result.
  Dynamic output production/availability remains consumer validation; a selector
  must not become runtime field access or implicit expression execution.
- **V1-CONS-008:** Provide compiler-independent discovery and selection of public
  entry points by stable symbol identity and exact contract type/revision, with
  defined missing/private/ambiguous-root failures and interpretive closure.
  No declaration is implicitly executable and declaration order cannot choose
  the entry point. Selection remains a post-compilation view and never changes
  complete project identity; consumers include selection in their own plan keys.
- **V1-CONS-009:** Standalone artifacts/views expose complete required type and
  vocabulary facts: canonical revisions, defaults, restrictions, variants,
  dependencies, selector types, and required features. Independent readers can
  structurally validate and inspect them without compiler linkage, source
  reparsing, ambient schemas, network discovery, or backend code. This does not
  assert a producer's authenticity or correct execution of a consumer contract.
- **V1-CONS-010:** Publish producer/reader compatibility discovery for exact
  source, bundle, IR, encoding, identity, and authoring profiles separately from
  package versions. Unknown required features and unsupported versions fail
  before exposing accepted data; no silent downgrade is allowed. Freeze a
  tested producer/reader compatibility matrix and migration/rejection fixtures.

### Approved Stage 8 extension — pure expressions and finite composition

These requirements are planned, not implemented or passing evidence. Share one
typed semantic contract across source, IR, wire, independent readers, and
authoring, with distinct authoring syntax and normalized IR representations;
do not create a second expression language inside strings or vocabularies.

Concrete written review candidates are [EXPRESSIONS](EXPRESSIONS.md),
[EVALUATOR](EVALUATOR.md) and [EXPANSION](EXPANSION.md). They specify syntax,
34-digit nonterminating division, explicit evaluator states and deterministic
expansion rules. They remain unfrozen until written approval; the separate
IR/wire/authoring compatibility and independent-vector gate remains unchecked.

- **V1-CONS-011:** Freeze and implement a separately versioned bounded pure
  expression contract: typed equality and ordered comparisons, `&&`, `||`, `!`,
  value-producing `if ... else`, and exhaustive closed-variant `match`.
  Specify admissible operand types, precedence, evaluation order, short-circuit
  behavior, branch result compatibility, and missing/duplicate match alternatives.
  All branches are type-checked; unselected branches are not evaluated. Reject
  effects and implicit coercions. Expressions are not imperative statements.
- **V1-CONS-012:** Keep one `num` type for pure addition, subtraction,
  multiplication, division, and unary negation; no integer-division mode or
  fraction type. Preserve exact decimal values/results when possible and support
  the closest decimal approximation for nonterminating division, using an
  explicitly versioned deterministic precision/rounding rule. Nontermination
  alone is not an error. Distinguish division by zero from resource-limit failure;
  never inherit host floating-point behavior or silently reduce precision to fit
  a caller budget. Freeze intermediate/result bounds and numeric rules below. Bound
  expression depth, nodes, work, numeric growth, and retained/output bytes, with
  cancellation and fallible allocation before expansion.
- **V1-CONS-013:** Support typed string/list construction and interpolation with
  explicit conversions and deterministic formatting. Freeze admissible types,
  Unicode behavior, list element compatibility, and independent size/work limits.
  No shell substitution, implicit command escaping, credential resolution, or
  environment lookup occurs; generated strings remain inert values.
- **V1-CONS-014:** Distinguish literal/static values, externally supplied typed
  inputs, and deferred operation-output selectors in expression contracts.
  Freeze dependency retention, static/deferred phase classification, binding
  rules, and unavailable/error states without overloading null/omission/default.
  Statically known evaluation is deterministic and bounded; a compatible consumer
  supplies typed bindings to the shared pure evaluator for deferred evaluation.
  Reader-only consumers receive complete operand/result types, guarded
  dependencies, and retained restriction obligations. Unsupported expression features
  fail explicitly rather than falling back to backend strings or weaker meaning.
- **V1-CONS-015:** Support bounded reusable descriptions with typed parameters
  and declared results. Freeze parameter binding/default rules, lexical scope,
  hygienic substitution, type compatibility, expansion identity and attribution,
  and independent invocation/expansion limits. Reject recursive composition and
  implicit capture of ambient inputs. These are immutable description templates,
  not general callable functions or execution units.
- **V1-CONS-016:** Support finite deterministic expansion over statically known
  lists for repeated descriptions and multi-dimensional matrices. Freeze order,
  filtering/exclusion semantics, generated symbol uniqueness, and attribution.
  Bound dimensions, Cartesian-product work, expanded nodes, and output size
  independently; reject one-over requests before proportional allocation.
  Deferred inputs cannot determine expansion cardinality, names, or topology;
  they may populate already expanded typed descriptions. Dynamic fan-out,
  reassignment, unrestricted recursion, and `while` remain out of scope.

All six requirements must retain public interpretive closure, safe source
attribution, version compatibility, and independently checked identities.
Consumer plan keys bind selected entries, mapping/target revisions, and relevant
external input bindings separately from complete Neutral project identity.
A shared pure evaluator is required inside Neutral, independently usable without
compiler-private ASTs. No consumer-specific executor or backend adapter belongs
inside it. Independent test interpreters verify the same versioned semantics
without delegating evaluation to the production evaluator or performing effects.

### Stage 8 design boundaries

Freeze the following policies before implementing V1-CONS-011..016. They are
development requirements, not changes to an already accepted source/IR profile.

#### Numeric policy — one `num`, deterministic approximation

Keep literal decimals and existing stored numeric values exact. Addition,
subtraction, multiplication and negation preserve exact decimal results within
independent numeric/work limits. Division retains a terminating exact result when
it fits those limits; a nonterminating quotient is rounded to the nearest decimal
under the separately versioned arithmetic context, not rejected or truncated as
integer division. Division by zero always fails. An exact result or required
intermediate that exceeds a resource limit fails rather than silently changing
the arithmetic context.

The proposed context is 34 significant decimal digits with round-to-nearest,
ties-to-even for nonterminating division. This precision is a contract-freeze
decision, not an activated capability or caller-adjustable approximation budget.
For example, `1 / 8` is exactly `0.125`; `1 / 3` produces a finite rounded
approximation, and `(1 / 3) * 3` need not equal `1`. Do not reassociate expressions
or claim algebraic equivalence across rounded intermediates. Freeze normalization,
rounding boundaries, scale/digit/work limits and literal vectors before activation.
Compiler folding and deferred evaluation must produce identical values/errors
under the same context, independently of host architecture or floating-point mode.

Existing number encodings/identity vectors are unchanged. New arithmetic and
expression/expansion semantics require separately versioned contracts and
compatible IR/wire/authoring features; do not append them to frozen profiles.

#### Shared pure evaluator and explicit binding boundary

Use one typed semantic model and production evaluation implementation for
compile-time folding and compiler-independent deferred evaluation. It takes
validated expression/profile facts, explicit typed bindings and independent
controls/cancellation; it returns a validated value, required bindings, or an
explicit unavailable/failure result. Missing bindings, unavailable outputs,
explicit null and failed evaluation are distinct states, never implicit defaults.
The evaluator reports binding needs; it never fetches or waits for an output.

Consumers acquire bindings and own operation scheduling, credential resolution,
authorization and effects. Evaluation has no ambient filesystem, network,
environment, clock, randomness or callback access, and no runtime function calls,
mutation or unbounded loops. Data-only consumer conditions can remain interpreted
by their exact consumer contract; adopting the language expression profile uses
this shared evaluator rather than backend-specific expression strings.

Before accepting supplied inputs or returning calculated values, validate exact
types, vocabulary restrictions, visibility and target contracts. Validate the
supplied envelope boundedly, but demand values only along selected guarded paths.
Every unresolved obligation retains safe source/contract attribution. No error
publishes a partial accepted result or replaces a successful cache generation.

Independently bound expression nodes/depth, binding count/bytes, work, numeric
growth, retained dependencies, diagnostics and output bytes. Check budgets and
cancellation before allocation/expansion, use fallible retention, and keep
diagnostic ordering deterministic. Negotiation covers arithmetic context,
operator semantics, result states, restriction enforcement and required evaluator
features, not just expression tag recognition. Reject unsupported semantics before
evaluation or consumer execution; callers cannot weaken them via local settings.

#### Evaluation phases and dependencies

| Expression | Compilation behavior | Consumer behavior |
| --- | --- | --- |
| Literal or pure expression with known operands | Normalize to a typed value, or return a bounded evaluation error | Consume the validated value |
| External input or future output selector | Validate type, public target contract, and retained constraints; do not resolve it | Bind and validate the supplied value |
| Ordinary operator with a deferred operand | Retain a typed expression and its potential value dependencies | Supply bindings to the shared evaluator under the negotiated semantics |
| Conditional, match, or short-circuit operator | Type-check every branch; evaluate only a statically selected path; otherwise retain guarded branches | Shared evaluator evaluates the guard and selected path only |

An unselected branch cannot fail through value evaluation or demand a future
output. An invalid type, missing symbol, or private target anywhere is still a
compile-time error. Do not eagerly fold a potentially failing branch whose guard
is deferred. Publish literal vectors for known and deferred guards, including
division by zero and missing/skipped outputs in selected/unselected paths.

Keep three relationships distinct: structural references checked during typing,
potential value dependencies (including their guards), and explicit consumer
execution prerequisites. A future output reference is not automatically a
scheduling edge. Core rejects static expression/template expansion cycles;
consumers validate their own output-availability and execution graphs. Consumers
must not conservatively require an unselected output merely because it appears
in a guarded expression. Unavailable bindings and evaluation failures are
explicit consumer result states, not null, omission, or materialized defaults.

#### Deferred restrictions and compatibility

Compilation proves structural typing and restrictions for known values; it does
not certify unknown future values. Retain every unresolved restriction and its
safe source/contract attribution in standalone IR and public views. A consumer
must validate bound inputs, selected outputs, and calculated results before
passing them to an operation, using the same exact restriction rules as static
values. Report an explicit failure rather than executing with invalid data.
Freeze which checks are statically discharged versus retained obligations, and
test both successful binding and type/restriction/availability failures.

Consumer compatibility covers operator semantics, deterministic numeric/rounding behavior,
conversions, deferred-value states, restriction enforcement, and evaluation
budgets, not just recognition of expression tags. Unsupported required behavior
is rejected before operation execution. Consumers may lower expressions to a
backend only when the frozen semantics are preserved; no backend's coercion,
rounding, null, or short-circuit rules become Neutral defaults.

#### Canonical identity

Captured identity retains source, expression, and template-definition changes.
Logical identity uses normalized static values and expanded descriptions:
`2 + 3` and `5` have the same logical meaning when the surrounding typed contracts
are identical. Source spelling, expansion provenance, and unused template
definitions do not independently change logical identity. Any exported semantic
contract remains an identity input.

Deferred expressions retain versioned canonical typed operator/operand structure
and stable symbolic targets. Do not attempt general algebraic equivalence,
commutative reordering, or effectful evaluation. Compile-time selected branches
normalize under the frozen rules; retained deferred branches retain their
meaning. Templates have no runtime callable identity: their expanded public
symbols and values participate under normal module-symbol/type rules.
Static parameter changes affect logical identity only when they change expanded
meaning or an exported contract. External input values, entry selection,
mapping/target revisions, and non-semantic consumer execution policy remain consumer plan
context. The declared evaluator/arithmetic profile is project meaning, not mutable
plan context; changing a referenced input's declared type/contract changes project
meaning. Define derivation/artifact partitions separately and publish equivalent
and non-equivalent vectors in a new profile without altering historical vectors.

#### Templates, expansion, and authoring

Templates are compile-time description constructors with explicitly named typed
parameters and declared result shapes, not callable values. No runtime dispatch,
recursive calls, higher-order parameters, mutation, or implicit ambient capture.
Reject recursive expansion even in otherwise unselected template branches.
Static parameters may determine expansion; deferred parameters may populate
typed values but cannot determine expansion cardinality, symbol names or topology.
All references are explicit and hygienically bound; generated public names must
be deterministic and collision-checked, never based on process-global counters.

Complete semantic IR retains expanded descriptions and residual deferred
expressions, not executable template definitions. The authoring model separately
retains template definitions, applications, matrix axes, and safe source mappings;
it must not reconstruct original templates from expanded IR. Freeze both the
source-preserving authoring round trip and the meaning-preserving IR round trip.
Bounds cover aggregate nested expansion and Cartesian-product work even when
filters discard results; per-template limits alone are insufficient.

Map-like data uses typed key/value entry records and lists; duplicate-key and
lookup policy belong to the declared consumer contract. A graph of operation
records demonstrates transport, not execution. Capability examples must include
an unsupported target and an explicitly approved alternative, with literal
expected reports. Condition/output examples must include unavailable outputs,
bad target types, cyclic evaluation/dependency graphs, and private dependencies.
Core may accept structurally valid graphs that a sample consumer rejects;
fixtures must distinguish those phases rather than claiming compiler rejection.

The minimum consumer fixture is a finite static dependency graph with typed
arguments, fan-out/join connections, explicit public entry selection, and
inspectable contract facts. Deferred conditions are a separate representation
family, not a prerequisite for executing or dynamically expanding that graph.
Require field-level diagnostic, default/reuse-origin, typed-selector, multiple
root, self-contained artifact, and old/new reader compatibility cases. Public
redaction must not expose private paths or implementation provenance.

Hosts, not the compiler, acquire and resolve the complete source/vocabulary
closure. Consumers receive validated immutable artifacts and derivation facts;
they do not depend on compiler-side acquisition or reinterpret source syntax.

## Evidence and completion

The consumer /1 design and boundary-oracle freeze completes only its Stage 8
contract tasks. The operation/resource/condition/capability corpus is registered
separately with 31 cases and pinned file bytes. Public reader/authoring implementation,
hostile/boundary validation and production comparisons remain unchecked.
V1-CONS-011..016 are not covered by this freeze.

The operational checklist is the sole task tracker. Register reviewed fixture
families and literal outcomes in the conformance manifest at gate activation;
planned examples are not active or passing evidence. Add exact limits,
cancellation, hostile decoder, shuffled/concurrent, clean/cache, migration, and
independent-reader tests. No new product repository or executable backend is
required to validate these data interfaces.

Before `v1.0.0`, the complete corpus must include these requirements. An example
or probe alone does not establish a runtime, universal backend portability,
authorization, or consumer execution correctness.
