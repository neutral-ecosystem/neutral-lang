<!-- SPDX-License-Identifier: Apache-2.0 -->

# 07 — Full implementation and validation checklist

Status: active implementation tracker

This is the operational tracking file for the complete journey from `v0.1.0`
to `v1.0.0`. Check an item only when its linked implementation and retained
validation evidence exist. Design acceptance alone does not complete an item.

Developers choose and review the next unchecked item. `cargo xtask dev` and
`cargo xtask ci pr` retain the toolchain, commit, commands, and results in
ignored workflow logs. This checklist is the routine completion authority;
do not create a separate Markdown evidence note for each small change. Add a
durable review only for a contract decision, exceptional risk, or gate that
needs human interpretation. The [validation ledger](06-VALIDATION-LEDGER.md)
is historical release context, not a duplicate daily tracker.

## Baseline — `v0.1.0`

- [x] Confirm the implementation reports exactly `v0.1.0`.
- [x] Run and retain the complete v0.1 conformance result.
- [x] Confirm `neu "0.1"` remains explicit and no v1 syntax is accepted.
- [x] Freeze the starting public API, dependency, diagnostic, and performance
  snapshots used for regression comparison.
- [x] Verify the reproducible development environment and clean CI baseline.

Baseline exit: all five items pass before Stage 1 implementation starts.

Evidence: [v0.1.0 baseline validation](evidence/v0-baseline.md).

## Stage 1 — profile dispatch and v1 contract freeze

Target transition: `v0.1.0 -> v0.2.0`.

### `v0.1.1` — contract and fixture gate

- [x] Freeze v0.1/v1.0 profile selection and unsupported-profile behavior.
- [x] Freeze v1 exclusions, shared limits, diagnostic families, and migration
  policy.
- [x] Register Stage 1 positive, negative, boundary, and lookalike fixtures.
- [x] Map Stage 1 requirements to fixture/oracle identifiers.

### `v0.1.2` — core implementation gate

- [x] Implement explicit profile dispatch without reinterpreting v0.1.
- [x] Implement deterministic rejection for unavailable v1 capabilities.
- [x] Implement shared limits and stable diagnostic envelopes.

### `v0.1.3` — public integration gate

- [x] Expose profile/capability discovery through public APIs.
- [x] Prove reader and CLI profile reporting agree.
- [x] Prove no public API leaks compiler-private syntax or semantic types.

### `v0.1.4` — validation gate

- [x] Run inherited v0.1 corpus, Stage 1 corpus, dependency audit, limits, and
  deterministic repeated/concurrent tests.
- [x] Review release notes and retained evidence with no skipped required test.
- [x] Mark every Stage 1 item in the v1 contract checklist complete.

Evidence: [Stage 1 profile-foundation validation](evidence/stage1.md).

### Promote to `v0.2.0`

- [x] Confirm `v0.1.4` evidence is complete and immutable.
- [x] Release `v0.2.0`; update the ledger to `released`.

## Stage 2 — complete no-I/O project capture

Target transition: `v0.2.0 -> v0.3.0`.

### `v0.2.1` — contract and fixture gate

- [x] Freeze `CapturedProjectRequest`, processing controls, limits, and failure
  outcomes.
- [x] Register closure, disconnected-unit, duplicate/mismatch, exact-lock, and
  host-mapping fixtures.
- [x] Freeze request/header and logical source/module identity rules.

Evidence: [Stage 2.1 captured-project contract gate](evidence/stage2-1-contract-gate.md).

### `v0.2.2` — core implementation gate

- [x] Implement bounded request validation and immutable capture.
- [x] Implement complete supplied closure and exact vocabulary-lock coverage.
- [x] Reject resolver callbacks and prove capture performs no external I/O.

Evidence: [Stage 2.2 core capture validation](evidence/stage2-2-core-capture.md).

### `v0.2.3` — public integration gate

- [x] Expose capture outcomes and resource facts through public contracts.
- [x] Integrate CLI, tests, and Editor-host request construction through the
  same request schema.
- [x] Verify shuffled source/lock order produces equivalent capture meaning.

Evidence: [Stage 2.3 public capture integration](evidence/stage2-3-public-integration.md).

### `v0.2.4` — validation gate

- [x] Run inherited v0.1 and all active capture suites.
- [x] Run malformed-input, allocation-before-validation, cancellation, and
  structural-limit tests.
- [x] Review closure identity and capture replay evidence.

Evidence: [Stage 2.4 capture validation](evidence/stage2-4-validation.md).

### Promote to `v0.3.0`

- [x] Confirm Stage 2 checklist, manifest, and traceability are complete.
- [x] Release `v0.3.0`; update the ledger to `released`.

## Stage 3 — modules, imports, SCCs, and diagnostics

Target transition: `v0.3.0 -> v0.4.0`.

### `v0.3.1` — contract and fixture gate

- [x] Freeze module-name grammar, one-unit rule, import grammar, alias rules,
  graph ordering, SCC behavior, and diagnostic ordering.
- [x] Register valid cycle, invalid semantic cycle, missing/self/duplicate
  import, alias collision, and graph-limit fixtures.
- [x] Review and pin Stage 3 fixture/oracle outcomes and the governing contract;
  keep executable conformance pending until the implementation gate.

Evidence: [Stage 3.1 module graph contract gate](evidence/stage3-1-contract-gate.md).

### `v0.3.2` — core implementation gate

- [x] Implement module/import parsing and deterministic graph construction.
- [x] Implement SCC condensation and independent per-module import, edge, and
  SCC bounds; condensation depth remains bounded by the source-unit limit.
- [x] Keep imports logical and unable to use paths, URLs, or acquisition.

Evidence: [Stage 3.2 core module graph](evidence/stage3-2-core-graph.md).

### `v0.3.3` — public integration gate

- [x] Expose module/import facts through captured project and diagnostics.
- [x] Verify source maps cover all cross-unit diagnostics.
- [x] Verify incremental and clean graph construction are equivalent.

Evidence: [Stage 3.3 public graph integration](evidence/stage3-3-public-integration.md).

### `v0.3.4` — validation gate

- [x] Run full module/import/SCC corpus under shuffled and concurrent schedules.
- [x] Run graph limit, recovery, ambiguity, and diagnostic-order tests.
- [x] Audit exclusions: wildcard, relative, implicit, re-export, partial module.

Evidence: [Stage 3.4 graph validation](evidence/stage3-4-validation.md).

### Promote to `v0.4.0`

- [x] Confirm Stage 3 checklist, manifest, and traceability are complete.
- [x] Release `v0.4.0`; update the ledger to `released`.

## Stage 4 — public APIs and cross-module semantics

Target transition: `v0.4.0 -> v0.5.0`.

### `v0.4.1` — contract and fixture gate

- [x] Freeze private-by-default/public syntax, public type closure, qualified
  access, reuse/ref rules, and semantic-cycle behavior.
- [x] Register visibility leak, private ref target, type compatibility,
  provenance, and cross-SCC value-cycle fixtures.

### `v0.4.2` — core implementation gate

- [x] Implement visibility-aware name/type/value/ref resolution.
- [x] Implement public signature closure and exposed-reference target checks.
- [x] Implement stable cross-module module-symbol identity edges.

The Stage 4 core analyzer resolves declaration dependencies and validates
visibility before any project-IR publication. Its narrow public-interface
snapshot is independently validated by the reader; complete project IR remains
Stage 6 work. [Stage 4 validation evidence](evidence/stage4-validation.md)
records the integration and compatibility review.

### `v0.4.3` — public integration gate

- [x] Expose public export indices and redacted provenance through the reader.
- [x] Verify private implementation details cannot leak through public views.
- [x] Verify independent consumer enumeration of cross-module values and refs.

### `v0.4.4` — validation gate

- [x] Run visibility, public closure, reuse, ref, semantic cycle, and reader
  security suites.
- [x] Run deterministic ordering and public-API fingerprint tests.
- [x] Review compatibility behavior for all extended v0 semantic rules.

### Promote to `v0.5.0`

- [x] Confirm Stage 4 checklist, manifest, and traceability are complete.
- [x] Release `v0.5.0`; update the ledger to `released`.

## Stage 5 — multiple vocabularies and inert location values

Target transition: `v0.5.0 -> v0.6.0`.

### `v0.5.1` — contract and fixture gate

- [x] Freeze repeated aliased `use`, exact lock coverage, public vocabulary
  types, metadata separation, and `url`/`path` behavior.
- [x] Register multiple-alias, missing/extra/conflicting lock, private type,
  executable payload, and location-value fixtures.

### `v0.5.2` — core implementation gate

- [x] Implement exact multi-vocabulary validation and canonical identities.
- [x] Implement public vocabulary type enforcement.
- [x] Implement distinct inert `url` and `path` source/IR values.

### `v0.5.3` — public integration gate

- [x] Expose canonical vocabulary identities/revisions through IR and reader.
- [x] Verify aliases remain source-local and non-semantic.
- [x] Verify authoring metadata cannot enter semantic vocabulary bundles or
  alter the canonical vocabulary catalogue. The separate presentation-only
  effect is a Stage 8 authoring validation, not a Stage 5 semantic input.

### `v0.5.4` — validation gate

- [x] Run strict schema, duplicate/unknown field, executable-content,
  lock-cover, alias, and location-value suites.
- [x] Prove location values cannot fetch/open/normalize/authorize anything.
- [x] Run hostile vocabulary decoder limits and fuzz tests.

### Promote to `v0.6.0`

- [x] Confirm Stage 5 checklist, manifest, and traceability are complete.
- [x] Release `v0.6.0`; update the ledger to `released`.

## Stage 6 — project IR, reader, and views

Target transition: `v0.6.0 -> v0.7.0`.

### `v0.6.1` — contract and fixture gate

- [x] Freeze project IR, companions, reader validation, result envelopes,
  resource facts, view request, and dependency-closure rules.
- [x] Register complete/private/public/view/malformed artifact fixtures.

### `v0.6.2` — core implementation gate

- [x] Lower only fully valid projects to complete project IR.
- [x] Implement source maps, provenance, derivation/resource facts, and public
  reader validation.
- [x] Implement post-compilation view derivation without changing project IR.

Evidence: [Stage 6 contract and core](evidence/stage6-contract-core.md).
These gates publish an in-process library boundary, not a standalone encoded
project artifact. Project identity transcripts remain Stage 7 work.

### `v0.6.3` — public integration gate

- [x] Implement standalone reader-only project probe.
- [x] Verify views preserve all interpretive dependencies while redacting
  private implementation provenance.
- [x] Verify roots never affect capture, logical equality, complete serialized
  IR, or public-interface fingerprint inputs.

Capture, logical equality, complete serialized IR, and public-interface
fingerprint inputs are verified invariant under root selection. Complete
project identity and its digest assertion are Stage 7 work; that part remains
unclaimed. Evidence: [Stage 6 integration and validation](evidence/stage6-integration-validation.md).

### `v0.6.4` — validation gate

- [x] Run IR round-trip, hostile decoder, source-map, provenance, view, bounds,
  and independent reader-probe suites.
- [x] Run clean capture, captured replay, changed-unit reconstruction, and
  serialization-order equivalence tests.
- [x] Review all public result and failure envelopes.

Clean capture, immutable captured replay, changed-unit reconstruction, shuffled
serialization, and concurrent compilation were verified in Stage 6. That review
did not include an incremental cache; actual cached/clean equivalence is now
verified separately in Stage 7.
The project-digest and actual cache-equivalence assertions are retained as
explicit Stage 7 validation gates below, where their identity APIs belong.

### Promote to `v0.7.0`

- [x] Confirm Stage 6 checklist, manifest, and traceability are complete.
- [x] Release `v0.7.0`; update the ledger to `released`.

## Stage 7 — canonical identity and reproducibility

Target transition: `v0.7.0 -> v0.8.0`.

### `v0.7.1` — contract and vector gate

- [x] Freeze canonical logical form, domain tags, transcript framing, SHA-256
  usage, identity exclusions, and artifact derivation inputs.
- [x] Publish literal captured/logical/derivation/artifact vector inputs and
  expected transcripts/digests.

### `v0.7.2` — core implementation gate

- [x] Implement bounded canonical logical form and every identity layer.
- [x] Keep graph-local IDs, host mappings, aliases, roots, capture order, and
  source evidence out of logical identity.
- [x] Implement deterministic artifact identity by kind and format.

Evidence: [Stage 7 contract, vectors, and integration review](evidence/stage7-contract-core.md).
All four typed layers are exposed through the validated reader; the reader-only
probe reports complete logical identity. A retained independent Python encoder
checks literal and adversarial vectors. Actual bounded syntax cache execution
rebuilds semantics and companions and is compared with clean compilation.
No version bump, approval, tag, or release is implied.

### `v0.7.3` — public integration gate

- [x] Expose typed identities and derivation facts through public reader APIs.
- [x] Produce identity vectors from a second independent implementation.
- [x] Verify cache keys use the correct identity layer.

### `v0.7.4` — validation gate

- [x] Compare both implementations for every accepted/adversarial vector.
- [x] Run formatting/order/equivalence/non-equivalence/collision-path tests.
- [x] Verify root selection never changes complete project identity transcripts
  or digests; extend the Stage 6 root-invariance tests to the identity APIs.
- [x] Verify actual incremental/cache execution and clean construction are
  equivalent, including changed units and stale-cache rejection. Captured replay
  alone does not satisfy this gate.
- [x] Review transcript and identity-profile version immutability.

### Promote to `v0.8.0`

- [x] Complete the vocabulary composition extension gate below; existing
  Stage 7 identity evidence does not cover these newly scheduled requirements.
- [x] Confirm Stage 7 checklist, manifest, and traceability are complete.
- [x] Release `v0.8.0`; update the ledger to `released`.

Package publication and extension conformance remain separate evidence; the
explicit successor extension is now covered by the required conformance suite.

### `v0.8.0` — vocabulary composition extension gate

Scope: [V1-VOC-005..011](../specs/contracts/CONSUMER-READINESS.md).
Previously completed `.1`–`.4` identity gates remain historical evidence;
this additional contract -> implementation -> integration -> validation gate
remains required even if a package tag was already created. Package publication
does not establish extension conformance. No frozen schema is amended in place.

The [composition contract](../specs/contracts/VOCABULARY-COMPOSITION.md) and
[profile supplement](../specs/contracts/COMPOSITION-PROFILE.md) freeze bundle,
source/diagnostic, successor IR/wire and identity requirements. Literal inputs
are pinned separately from production activation.
The vocabulary crate's `compatibility_*` tests protect the old schema; passing
those tests alone does not establish implementation or validation of the extension.

#### Completed standalone foundation

- [x] Implement and test the separate composition catalogue boundary and shared
  raw IR model: composite vocabulary types, variants, exact dependency closure,
  closed defaults/restrictions, presence distinctions, limits and cancellation.
  This sub-step does not activate source syntax, project IR/wire/identities,
  reader/probe integration, or full extension conformance.
- [x] Expose public composition catalogue facts through compiler-independent
  reader APIs, with exact revision lookup and private-type/debug redaction.
  Complete project/wire/probe/view/provenance integration remains pending.
- [x] Register byte-pinned standalone positive/negative/boundary/migration
  catalogue inputs and literal oracles; runtime copies pass without the portable
  plan. Full source/project/wire activation remains pending; successor identity
  design vectors are registered separately below.
- [x] Share closed supplied-value materialization with the default validator;
  validate variant lists and restrictions, preserve optional absence/explicit
  null/defaulted origins, and keep materialized meaning separate from origin facts.
- [x] Expose independent reader materialization and canonical reference-type
  paths through fields, all variant alternatives, lists, and nullable wrappers;
  redact private/missing roots and reject exhausted traversal/reference/depth
  limits or cancellation without partial enumeration.
- [x] Test supplied values and default expansion at independent node, depth,
  list-item, string-byte, numeric-digit and scale boundaries; test invalid
  fields/tags/payloads/restrictions, ordering and concurrent request isolation.
- [x] Run `cargo xtask test all`: 638 Nextest tests, zero skips, plus the
  compile-fail doctest. Standard `cargo xtask ci pr`, strict Clippy, portable
  verification and all 59 registered fixture/oracle hash checks pass.
- [x] Extend and build the vocabulary fuzz harness to exercise bounded
  supplied-value probes on accepted catalogues. This is build validation, not
  evidence of a completed fuzz campaign or allocation-fault review.

Evidence: [standalone composition catalogue/value validation](evidence/stage7-contract-core.md#standalone-composition-cataloguevalue-slice-06-10-2026),
[literal/value tests](../../crates/neutral-vocabulary/tests/composition_values.rs)
and [independent reader tests](../../crates/neutral-reader/tests/composition.rs).
These completed items do not activate source reference-value edges, complete
project companions, codec/probe support, public views or new identity profiles.

#### Explicit capture and captured-identity integration

- [x] Add a separate `neutral.capture/v2` API with exact required features,
  independent composition controls and no old-compiler fallback/downcast.
- [x] Validate exact source/bundle integrity and complete transitive lock cover
  before publication; retain module-local repeated aliases and exact replay.
- [x] Share validated catalogues with the independent reader without copying
  contracts; inspect restrictions, defaults, public types and reference-type paths.
- [x] Implement separately typed captured identity /2 and compare production
  transcript bytes/digest with the frozen literal vector, including exact/one-over
  byte/frame bounds. Preserve existing /1 transcripts and vectors.
- [x] Validate this slice with `cargo xtask test all` and `cargo xtask ci pr`:
  669 Nextest tests, zero skips, and two compile-fail doctests pass. Portable
  verification and all 64 fixture/oracle checks pass without changing frozen inputs.

Evidence: [capture and captured-identity integration](evidence/stage7-contract-core.md#composition-capture-and-captured-identity-integration-06-10-2026),
[capture tests](../../crates/neutral-compiler/tests/project_capture/composition.rs)
and [cross-package tests](../../crates/neutral-test-suite/tests/composition_contract/capture.rs).
This is capture/catalogue integration, not source compilation, new project IR,
wire/probe/view support, or complete logical/derivation/artifact identity.

#### Shared resolved source/vocabulary type scope

- [x] Associate source module-symbol owners with the same raw record/variant
  bodies used by vocabularies; validate resolved ownership, all type branches,
  public closure, embedded cycles, restrictions and closed defaults atomically.
- [x] Share contextual value materialization across both nominal origins,
  including heterogeneous variant lists and source-to-vocabulary defaults;
  keep default/supplied origins separate from equivalent materialized meaning.
- [x] Apply the frozen occurrence-depth rule to the new resolved scope: root
  zero, one level per record/list/variant child, nullable visits without extra
  depth. Preserve the standalone catalogue API's existing depth behavior.
- [x] Expose public resolved source/vocabulary contracts and exact reference-type
  paths through a compiler-independent reader; redact private/missing roots and
  private debug information without inventing source spans or binding edges.
- [x] Validate ordering, invalid tags/payloads/defaults/presence, unselected private
  types, embedded versus reference cycles, aggregate/per-module/choice/default
  visit boundaries, Unicode/list restrictions, cancellation and concurrent isolation.
- [x] Run complete workspace tests and standard CI: 699 Nextest tests, zero
  skips, both compile-fail doctests and strict Clippy pass. Portable verification
  and all 64 fixture/oracle pairs pass without manifest or freeze updates.

Evidence: [shared resolved composition scope](evidence/stage7-contract-core.md#shared-resolved-composition-scope-06-10-2026),
[type/value tests](../../crates/neutral-vocabulary/tests/composition_scope.rs)
and [independent reader tests](../../crates/neutral-reader/tests/composition_scope.rs).
This validates resolved contracts, not `.neu` parsing/import resolution, source
binding reuse/ref semantics, compiled project IR, encoded artifacts or public views.
No frozen suite/profile/vector is activated or rewritten by this sub-step.

#### Resolved binding values and independent reference inspection

- [x] Share one structural value model and materializer between closed defaults
  and reference-capable resolved bindings; keep binding references impossible in
  contract defaults by construction.
- [x] Check complete binding indices, invariant target types, forward/cyclic
  non-embedding references and public/external target visibility atomically.
- [x] Materialize defaults/restrictions with cumulative request budgets and retain
  separate origin and actual field/list/payload binding-reference paths.
- [x] Recheck stricter semantic controls against unused contracts/defaults;
  preserve captured-byte/digest validation as a separate capture responsibility.
- [x] Expose public binding values, origins and actual reference targets through
  a compiler-independent reader, with private/missing lookup and debug redaction.
- [x] Validate both nominal origins, numeric/Unicode/list boundaries, omission
  versus null, incompatible/private/dangling targets, cancellation, exact/one-under
  value budgets, ordering and concurrent isolation. Standard CI passes 712 Nextest
  tests with zero skips and three compile-fail doctests.

Evidence: [resolved binding validation and reader inspection](evidence/stage7-contract-core.md#resolved-binding-validation-and-reader-inspection-07-10-2026),
[value/reference tests](../../crates/neutral-vocabulary/tests/composition_bindings.rs)
and [reader tests](../../crates/neutral-reader/tests/composition_bindings.rs).
These are resolved-model binding edges, not `.neu` parsing/reuse/import resolution,
checked original-byte attribution, complete successor IR or encoded probe/views.

#### Complete extension gates

Reviewed 08-10-2026: the explicit successor library pipeline and standalone probe
are covered by the required `composition-contract` suite. Frozen contracts,
fixtures, vectors and the original independent oracle remain unchanged. There is
no implicit fallback or source-header/compiler-CLI activation. Existing JSON
catalogue fixtures are review inputs, not `.nir` artifacts.

- [x] Freeze the new bundle schema, source-aligned list/nullable/reference
  composition, semantic defaults/restrictions, compatibility, diagnostic codes,
  public closure, recursive-type rules, and independent bounds.
- [x] Register literal positive/negative/boundary/migration fixtures and new
  identity vectors before implementation; pin reviewed inputs at activation.
- [x] Implement composition, closed defaults, finite choices, numeric and
  string/list length restrictions through compiler, IR, encoding, and reader.
- [x] Expose complete new contract facts and reference dependencies through
  independent reader/probe APIs with public-view closure and source attribution.
- [x] Pass hostile schema/decoder, invalid default/constraint/reference,
  exact/one-over, cancellation, ordering, clean/cache, and old-schema suites.
- [x] Verify semantic restrictions/defaults affect the appropriate identities,
  presentation does not, and existing identity profiles/vectors remain immutable.
- [x] Freeze and implement closed tagged variants across source, vocabulary,
  semantic typing, IR, wire, reader, and identity; reject wrong/unknown tags and
  payloads and validate heterogeneous typed collections.
- [x] Freeze and implement cross-vocabulary public type dependencies and exact
  transitive lock closure, including diamond/conflicting/private/missing/cyclic
  dependency rules without compilation-time acquisition.
- [x] Freeze required/omitted/null/default distinctions and safe origin facts;
  verify equivalent materialized meaning and distinct source evidence correctly.
- [x] Pass variant, cross-bundle closure, omission/default, old/new schema,
  public-view, hostile-input, exact/one-over, and independent identity suites.

Evidence: [pipeline and consumers](evidence/stage7-contract-core.md#composition-public-views-probe-and-attribution-07-10-2026),
[fallible processing and recovery](evidence/stage7-contract-core.md#replay-and-presentation-allocation-closure-08-10-2026),
and [identity/corpus closure and final validation](evidence/stage7-contract-core.md#composition-identity-and-registered-corpus-closure-08-10-2026).
Both declaration origins share one semantic model. Presentation inputs remain
separate Stage 8 work; the frozen identity exclusion is tested here, not an
unimplemented authoring projection. Source evidence is checked, not authenticated.

## Stage 8 — authoring bridge, pure expressions, and finite composition

Target transition: `v0.8.0 -> v0.9.0`.

Execute in dependency order: (1) selectors/input phases/compatibility,
(2) pure expressions and identities, (3) templates/finite expansion,
(4) full authoring integration. Each slice needs reviewed contracts, literal
oracles, implementation, public integration, and boundary evidence before its
dependent slice starts. The `.1`–`.4` gates below summarize release readiness,
not permission to implement every subsystem before testing any of them.

### `v0.8.1` — contract and fixture gate

- [x] Freeze profile discovery, descriptor catalogue, overlay, editable model,
  mappings, projection, formatting, diagnostics, and authoring limits.
  Foundation: [AUTHORING/1](../specs/contracts/AUTHORING.md), pinned in the
  [freeze record](../specs/contracts/freeze.toml); extensions remain separate gates.
- [x] Register core/vocabulary catalogue, model, stale-revision, projection,
  round-trip, and mismatch fixtures.
  [26 literal requests](../specs/fixtures/authoring/foundation.json) and
  [oracles](../conformance/oracles/authoring/foundation.json) are registered as
  frozen, not required-active; production comparisons remain implementation work.
- [x] Freeze [V1-CONS-001..010](../specs/contracts/CONSUMER-READINESS.md):
  operation/input/result/resource data, explicit prerequisites/output connections,
  structured conditions/deferred inputs, symbolic handles, capability/mapping
  descriptions, validation reports, and consumer/core ownership boundaries.
  [Consumer/1](../specs/contracts/CONSUMER-PROFILE.md) freezes the protocol and
  ownership boundaries; production implementation remains pending.
- [x] Freeze field/element source attribution, typed member selectors, explicit
  entry discovery, self-contained artifact contracts, required-feature negotiation,
  and the producer/reader compatibility matrix with literal fixture oracles.
  [32 literal cases](../specs/fixtures/consumer/boundary.json) and
  [oracles](../conformance/oracles/consumer/boundary.json) are pinned in the
  [freeze record](../specs/contracts/freeze.toml), frozen but not required-active.
- [x] Register project-neutral operation graph, map-entry, condition/output,
  unsupported-capability, explicit-alternative, source-attribution, privacy,
  and malformed/limit fixtures with literal expected consumer reports.
  Use file-backed `.neu` and captured vocabulary inputs; separate core type
  rejection from core-valid consumer cycle, availability and capability failures.
  [31 literal cases](../specs/fixtures/consumer/graphs/cases.json), 13 source files,
  one captured bundle and [reports](../conformance/oracles/consumer/graphs.json)
  are pinned as frozen-only; consumer implementation/comparison remains pending.
- [x] Freeze approved V1-CONS-011..016: pure comparisons/Boolean expressions,
  conditional selection/exhaustive matching, single-`num` deterministic decimal
  arithmetic with approximate nonterminating division, typed string/list
  construction, static/deferred inputs, typed parameterized descriptions, and
  finite static matrix expansion; specify syntax, scope, types, short-circuit
  behavior, division precision/rounding, diagnostics, independent bounds, and phases.
  Nontermination is not an error; distinguish division by zero and resource limits.
  Freeze expression/expansion/evaluator profiles separately without changing
  historical numeric values, wire formats or identity vectors.
  Approved and frozen on 12-10-2026: [expressions](../specs/contracts/EXPRESSIONS.md),
  [evaluator](../specs/contracts/EVALUATOR.md), [expansion](../specs/contracts/EXPANSION.md).
  Reviewed bytes are pinned in [freeze.toml](../specs/contracts/freeze.toml).
  Compatibility/vectors and implementation remain separate gates; syntax is not activated.
- [x] Freeze the shared pure evaluator contract: explicit typed bindings;
  validated value, required-bindings and unavailable/failure states; guarded
  dependencies; retained restrictions; independent work/depth/numeric/output
  limits; cancellation, attribution and deterministic failure reports. No ambient
  I/O, credential lookup, scheduling or backend execution.
- [x] Register literal positive/negative/boundary expression/composition fixtures
  and independent result/transcript vectors before implementation; freeze source,
  IR/wire/authoring feature compatibility and explicitly amend source exclusions
  without reinterpreting existing profiles or vectors.
  Approved and frozen on 12-10-2026: [successor compatibility](../specs/contracts/EXPRESSION-PROFILE.md),
  [68 literal cases](../specs/fixtures/expressions/cases.json), 42 source files,
  one bundle and [independent vectors](../specs/fixtures/expressions/vectors.json).
  [Complete residual/template artifacts and ten identity variations](../specs/fixtures/expressions/complete-vectors.json)
  and [authoring projections/rejections](../specs/fixtures/expressions/authoring-vectors.json)
  are registered and independently checked. Written compatibility/source-amendment
  and literal-byte approval is recorded in [freeze.toml](../specs/contracts/freeze.toml).
  Registration is not production conformance; activation remains pending.
- [x] Freeze the [Stage 8 design boundaries](../specs/contracts/CONSUMER-READINESS.md#stage-8-design-boundaries):
  guarded static/deferred evaluation, structural/value/scheduling edge separation,
  retained restriction obligations, normalized static/expanded logical identity,
  non-callable template semantics, and distinct source-authoring/semantic IR views.
  Approved and byte-pinned on 12-10-2026 in [freeze.toml](../specs/contracts/freeze.toml).
  Public template contracts remain inspectable, not executable; implementation
  and production conformance remain unchecked below.

### `v0.8.2` — core implementation gate

Slice 1 — consumer contract foundation:

- [x] Implement the closed data-only capability/report boundary and examples;
  do not add an executor, backend adapter, or implicit deferred evaluation.
  `neutral-reader::consumer` validates typed envelopes against complete decoded
  projects, exact public contracts, explicit features/context and real logical
  identity. File-backed examples test statuses, privacy, limits and atomic failures.
  JSON transport codecs and full consumer-profile activation are not implied.
- [x] Implement bounded public field-level attribution and typed selectors;
  preserve safe supplied/default/reuse origins without runtime evaluation.
  Reader-only typed field/index/alternative lookup checks exact public types and
  retains type-mode availability obligations. Origin projection preserves checked
  default/reuse ownership and safe occurrences with explicit redaction/absence;
  decoded-artifact tests cover independent bounds, allocation faults and cancellation.
- [x] Implement explicit typed entry discovery and complete contract facts in
  standalone artifacts/views with exact profile/feature compatibility discovery.
  Reader/probe APIs select exact public binding contracts, preserve complete
  redacted interpretation closure, and negotiate exact artifact tuples/features.
  Decoded-fixture, independent-bound, allocation/cancellation and recovery tests
  pass; standalone source activation and JSON transport are not implied.
- [ ] Retain typed external/output inputs and unresolved restriction obligations;
  expose separately guarded value dependencies and explicit prerequisite data,
  with consumer binding/availability/failure reports and no implicit scheduler.
  Raw `/3` project data and compiler-independent all-branch semantic checks retain
  derived restriction/selector obligations and recompute all seven expression facts.
  Shared work/retention budgets, exact/one-over controls and allocation/cancellation
  recovery tests pass. Public template frames/defaults are typed without instantiation.
  Complete source/IR/wire/reader integration, evaluation, expansion and guarded
  consumer binding/dependency reports remain; model validity
  alone does not satisfy this gate or activate `/3`.

Slice 2 — pure semantics and identity:

- [ ] Implement one bounded typed expression model through source, compiler,
  IR, encoding, and reader, including pure static normalization and
  retained deferred operands/dependencies; no external input lookup or effects.
- [ ] Implement a shared compiler-independent pure evaluator used by both static
  folding and deferred evaluation. Keep one `num`; use identical exact/rounded
  decimal semantics and errors in both paths, never host floating-point defaults.
  Accept explicit typed bindings only; report required bindings without acquisition.
- [ ] Type-check every branch without eagerly evaluating unselected or
  deferred-guarded error paths; resolve every name, define static expression-cycle
  rejection, and never demand outputs from unselected branches.
- [ ] Validate supplied bindings and calculated results against exact types,
  visibility and retained vocabulary restrictions. Keep missing, unavailable,
  explicit null and failed distinct; fail atomically with safe source attribution.
- [ ] Implement canonical static-value/deferred-expression identities and
  independent interpretation before dependent expansion/authoring integration.

Slice 3 — compile-time description composition:

- [ ] Implement hygienic typed parameter binding and finite static list/matrix
  expansion with deterministic identities, source origins, cancellation, and
  fallible independent expansion/work/storage limits; reject recursive templates
  and deferred expansion sizes, generated names or topology. Deferred values may
  populate already expanded descriptions, not control their structure.
- [ ] Lower templates to expanded semantic descriptions, preserving source forms
  and expansion mappings separately for authoring; reject runtime callable values,
  generated-name collisions, and aggregate nested expansion/work overflow.

Slice 4 — complete authoring integration:

- [ ] Implement deterministic static catalogue and project overlay.
- [ ] Implement closed bounded editable model with connections as the sole
  reuse/reference edge representation.
- [ ] Implement deterministic multi-source projection and mappings.
- [ ] Extend catalogue/model/projection for composite vocabulary fields,
  semantic defaults/restrictions, expressions, template applications, matrices,
  and consumer connections; authoring cannot invent or weaken semantic rules.
- [ ] Cover variants, cross-vocabulary types, selectors, and origin distinctions
  in generic authoring discovery, editing, projection, and diagnostics.

### `v0.8.3` — public integration gate

- [ ] Implement generic Editor probe from discovery only.
- [ ] Verify create, import, edit, project, compile, save, reopen, and no-op
  round trip without private parser/AST access.
- [ ] Verify compiler authority on authoring-preflight disagreement.
- [ ] Pass a standalone reader-only consumer probe enumerating typed operations,
  inputs/results, resource/prerequisite/output connections, deferred conditions,
  and their public interpretive closure without compiler-private access.
- [ ] Verify reports preserve safe source attribution and bind external plan
  context to mapping/target/input revisions without changing project identity.
- [ ] Verify compiler-free readers locate selected members and public entry
  points, explain field-level failures, and discover all necessary contract facts
  without reparsing source, fetching schemas, or depending on another project.
- [ ] Expose expression types, static/deferred phase facts, dependencies, and
  parameter/expansion origins through standalone readers and generic authoring;
  verify no-op projection/formatting, public-view closure/redaction, and explicit
  rejection of unsupported expression features by consumer probes.
- [ ] Exercise the shared evaluator from a compiler-independent reader consumer;
  negotiate required expression/arithmetic/evaluator features before evaluation,
  and keep explicit prerequisites separate from guarded value dependencies.
- [ ] Verify reader-only binding checks retain and enforce unresolved numeric,
  choice, length, and reference restrictions before sample operation dispatch;
  no actual backend operations or credential resolution are performed.
- [ ] Verify authoring round trips preserve template/application/matrix source
  structure, while semantic IR round trips preserve expanded meaning without
  requiring template reconstruction from IR.

### `v0.8.4` — validation gate

- [ ] Run catalogue identity, overlay revision, limits, stale/cancelled request,
  formatting, diagnostic, and round-trip suites.
- [ ] Verify separate vocabulary authoring metadata affects presentation only;
  semantic contracts, project IR, and identity stay unchanged.
- [ ] Audit that no adapter or descriptor contains executable callbacks.
- [ ] Review authoring/core independent version compatibility.
- [ ] Pass bounded consumer-data/protocol and authoring tests for wrong output
  types, unavailable outputs, private dependencies, cyclic consumer graphs,
  unsupported targets, explicit alternatives, and malformed/unknown members.
- [ ] Verify data transport is distinguished from consumer interpretation;
  core-valid consumer-invalid graphs cannot be mislabeled compiler failures.
- [ ] Pass composite-field/default/constraint projection, formatting, no-op
  identity, cancellation, hostile-input, and clean/incremental equivalence suites.
- [ ] Pass member-path/type/bounds, ambiguous/private/missing-root, attribution
  redaction, static fan-out/join, self-contained artifact, and producer/reader
  compatibility/unknown-required-feature suites, including old-profile regressions.
- [ ] Pass expression typing, short-circuit/unselected-branch errors, exhaustive
  matching, arithmetic/division boundaries, Unicode/conversion, parameter scope,
  duplicate generated symbols, finite matrix ordering and exact/one-over suites.
- [ ] Compare static folding, the shared deferred evaluator and an independent
  interpreter against literal exact/rounded decimal vectors: `1 / 8`, `1 / 3`,
  signed rounding, precision boundaries, division by zero and resource limits.
  Prove skipped branches do not demand unavailable outputs or evaluate errors.
- [ ] Pass static/deferred phase and missing/skipped-output consumer cases,
  cancellation/allocation faults, independent interpretation, expression identity
  vectors, and actual clean/cache equivalence without running backend operations.
- [ ] Compare selected/unselected unavailable and invalid-value paths; verify
  structural typing checks all branches, guarded outputs do not become implicit
  prerequisites, and bound values/results obey retained restrictions.
- [ ] Prove static-expression/literal and template/explicit-expansion logical
  equivalence, captured/source-evidence differences, canonical deferred-expression
  non-equivalence, and external-binding/project-identity separation.

### Promote to `v0.9.0`

- [ ] Confirm Stage 8 checklist, manifest, and traceability are complete.
- [ ] Confirm V1-VOC-005..011 and V1-CONS-001..016 have complete public evidence
  and no dependency on a separate product implementation.
- [ ] Release `v0.9.0`; update the ledger to `released`.

## Stage 9 — full conformance and `v1.0.0` qualification

Target transition: `v0.9.0 -> v1.0.0`.

Entry requires feature-complete Stage 8 with no deferred semantic implementation.
Stage 9 permits defect fixes, migration work, and compatibility corrections only.
New features or changed operator/type/identity semantics return to Stage 8 contract
review and invalidate affected evidence; changes require fresh final integration
and all source-bound measurements invalidated by the changed inputs.

### `v0.9.1` — complete contract and corpus gate

- [ ] Freeze all v1 requirements, contracts, diagnostic registries, examples,
  fixtures, oracles, vectors, manifest, and traceability.
- [ ] Confirm there are no planned active cases or unresolved portability
  exceptions.
- [ ] Audit the already frozen V1-CONS-011..016 operator/type/phase and bounded
  composition corpus, compatibility matrix, literal results, diagnostics, and
  identity vectors; unresolved division, conversion, or expansion semantics
  block entry and return to Stage 8 rather than being designed during qualification.

### `v0.9.2` — defect closure and migration gate

- [ ] Confirm all approved v1 features are implemented; close qualification
  defects and remove temporary gates not part of the public profile contract.
  Return missing semantic features to Stage 8; do not expand release scope here.
- [ ] Complete migration tooling/documentation without altering v0.1 meaning.
- [ ] Revalidate affected contracts/oracles and regenerate invalidated evidence
  after fixes; rerun final public integration against the corrected inputs.

### `v0.9.3` — public consumer gate

- [ ] Pass independent Reader probe over the complete v1 corpus.
- [ ] Pass generic Editor probe over core and captured vocabulary features.
- [ ] Pass the project-neutral consumer boundary probe over composite vocabulary
  data, conditions/output connections, capability reports, source attribution,
  and identity separation without mapper execution or core-owned product semantics.
- [ ] Pass independent pure-expression interpretation against literal oracles
  and reader-only end-to-end examples combining typed parameters, finite matrices,
  output selectors, and conditions; preserve unavailable/skipped-output failures
  and reject unsupported capabilities without silent semantic degradation.
  Compare compiler folding and compiler-independent shared evaluation under the
  identical arithmetic/profile context; unsupported required features fail closed.

### `v0.9.4` — release validation gate

- [ ] Pass inherited v0.1 and complete v1 conformance suites.
- [ ] Pass formatter/linter/docs, unit/integration/system/property/fuzz,
  malformed/hostile input, limits, determinism, concurrency, soak, performance,
  security, dependency, and license gates.
- [ ] Pass all identity vectors and clean/incremental equivalence tests.
- [ ] Pass expression/composition hostile-input, arithmetic-growth, short-circuit,
  exact/one-over expansion, cancellation/allocation, concurrent determinism, and
  clean/cache suites. Verify semantic expression/parameter changes affect the
  correct identities only when normalized meaning or exported contracts change;
  equivalent static values/expansions remain logically equal, while external
  bindings remain consumer plan context. Test deferred restriction enforcement
  and guarded dependencies independently from consumer scheduling.
- [ ] Verify every checklist item and traceability row has retained evidence.
- [ ] Review release notes, supported profiles, limits, exclusions, and public
  compatibility policy.

### Promote to `v1.0.0`

- [ ] Approve immutable `v0.9.4` evidence and release candidate artifacts.
- [ ] Publish `v1.0.0` and its portable conformance bundle.
- [ ] Mark all nine ledger rows `released`; archive the completed checklist.

## Final validation

- [ ] No unchecked item remains in this file.
- [ ] No unchecked item remains in the
  [v1 contract checklist](../specs/contracts/v1-checklist.md).
- [ ] No active manifest case is missing, skipped, flaky, retried, or
  indeterminate.
- [ ] Documentation links, standalone portable paths, and website publication
  validation pass.
- [ ] v1 is claimed conformant only after all preceding evidence is complete.
