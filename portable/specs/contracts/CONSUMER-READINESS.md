<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vocabulary composition and consumer readiness

Status: scheduled requirements; schemas and fixture oracles await gate review

This supplement schedules language infrastructure for `v0.8.0` and `v0.9.0`.
It does not activate syntax, change an accepted encoding, or introduce an
executor. Implementation follows reviewed schema, compatibility, diagnostic,
fixture, and identity-vector decisions in the implementation checklist.

## Ownership

Neutral owns bounded structural validation, typed data, exact vocabulary locks,
complete IR, reader views, identity, source attribution, and authoring projection.
Independent consumers own scheduling, expression evaluation, backend selection,
mapping implementation, authorization, acquisition, and effects.

Vocabularies remain closed data. Neither a semantic bundle nor presentation
metadata can load code, run a validator callback, grant privileges, or resolve a
credential. References are identity edges, not implicit execution prerequisites.
Module SCC acceptance never implies acceptance of an execution or expression cycle.

## `v0.8.0` — vocabulary composition

The Stage 7 extension gate must close these requirements before promotion:

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
  resolved semantic model. See the [variant contract proposal](VARIANTS.md).
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

- **V1-CONS-001:** Publish project-neutral, file-backed examples of typed
  operation descriptions, arguments/results, reusable resources, explicit
  prerequisites, and output connections using ordinary vocabulary records and
  references. Stable contract/type/revision identities distinguish meanings;
  source aliases and backend command names cannot substitute for them.
- **V1-CONS-002:** Publish bounded data conventions for conditions and deferred
  inputs, including operation-output references, literal values, and symbolic
  credential handles. Conditions are structured typed records, not shell or
  embedded backend expression strings. Freeze their representational types and
  interpreter boundary; evaluation, output availability, scheduling cycles,
  failure policies, retries, timeout behavior, and desired-state/idempotence
  semantics remain consumer responsibilities. No new core arithmetic, function,
  loop, secret scalar, built-in map, or execution syntax is implied.
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

The operational checklist is the sole task tracker. Register reviewed fixture
families and literal outcomes in the conformance manifest at gate activation;
planned examples are not active or passing evidence. Add exact limits,
cancellation, hostile decoder, shuffled/concurrent, clean/cache, migration, and
independent-reader tests. No new product repository or executable backend is
required to validate these data interfaces.

Before `v1.0.0`, the complete corpus must include these requirements. An example
or probe alone does not establish a runtime, universal backend portability,
authorization, or consumer execution correctness.
