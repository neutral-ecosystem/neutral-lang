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

- [ ] Complete the vocabulary composition extension gate below; existing
  Stage 7 identity evidence does not cover these newly scheduled requirements.
- [ ] Confirm Stage 7 checklist, manifest, and traceability are complete.
- [x] Release `v0.8.0`; update the ledger to `released`.

The release item records package publication, not completion of the composition
extension. The two conformance prerequisites above remain open.

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

#### Remaining full extension gates

Status reviewed 07-10-2026: the explicit capture /2 → source compiler → complete
project IR → independent reader → `NIR-PROJECT-CBOR/2` library pipeline is
implemented. Shared contracts cover both nominal origins, closed defaults,
constraints, tagged values, typed references and occurrence states. Production
complete logical identity matches the frozen independent /2 bytes and digest.
The standalone successor probe now independently decodes complete artifacts and
derives redacted public closure, with checked fine-grained occurrence attribution.
Validation passes 745 Nextest tests with zero skips and three compile-fail
doctests; see [pipeline evidence](evidence/stage7-contract-core.md#composition-source-ir-reader-and-wire-integration-07-10-2026)
and [consumer integration evidence](evidence/stage7-contract-core.md#composition-public-views-probe-and-attribution-07-10-2026).

Actual successor syntax-cache execution now matches clean complete artifacts,
including changed units, locked defaults, host IDs, controls and failed generations.
Next implementation order: remaining identity partitions; complete allocation/fault
review; then remaining capture/retention and transitive-bundle
adversarial validation against every frozen oracle/vector. Existing JSON catalogue
fixtures are not `.nir` artifacts.
The old project codec remains `NIR-PROJECT-CBOR/1`; successor transport uses
separate explicit library entry points with no fallback or source-compiler CLI activation.
Full extension activation remains pending. Keep the registered suite `frozen`
until all production comparisons and validation gates pass.

- [x] Freeze the new bundle schema, source-aligned list/nullable/reference
  composition, semantic defaults/restrictions, compatibility, diagnostic codes,
  public closure, recursive-type rules, and independent bounds.
- [x] Register literal positive/negative/boundary/migration fixtures and new
  identity vectors before implementation; pin reviewed inputs at activation.
  Evidence: [freeze and fixture/vector review](evidence/stage7-contract-core.md#composition-contract-and-literal-registration-06-10-2026).
  The `composition-contract` suite remains frozen, not required; existing
  standalone APIs do not establish compiler/codec/profile activation.
- [x] Implement composition, closed defaults, finite choices, numeric and
  string/list length restrictions through compiler, IR, encoding, and reader.
  Evidence: explicit source parsing/import/type/value/ref resolution, shared
  materialization, complete successor IR, restricted successor encoding and
  independent decoded-artifact validation. Literal positive/negative/boundary/
  migration and new nested-default/reuse/reference tests pass. This functional
  library gate does not claim the later quality gates.
- [x] Expose complete new contract facts and reference dependencies through
  independent reader/probe APIs with public-view closure and source attribution.
  Evidence: independent complete decoding and standalone successor probe, both-origin
  public contracts/defaults/restrictions, all alternative/reference types, actual
  binding reference targets and exact transitive vocabulary closure. Public views
  redact source/private implementation evidence without changing complete identity.
  Source subexpression/default/reuse spans and canonical vocabulary default owners
  are independently checked against retained companions. Unavailable attribution
  stays explicitly absent; no byte spans or producer authenticity are invented.
  See [consumer integration evidence](evidence/stage7-contract-core.md#composition-public-views-probe-and-attribution-07-10-2026).
- [ ] Pass hostile schema/decoder, invalid default/constraint/reference,
  exact/one-over, cancellation, ordering, clean/cache, and old-schema suites.
  Implemented: catalogue/capture/resolved-model negative, boundary, cancellation,
  ordering, concurrent-isolation and old-schema tests, plus successor truncation,
  unknown/nonminimal tags, stale companions/facts, source negatives and caller
  bounds. Actual successor clean/cache execution now compares complete IR,
  companions and wire bytes for cold/warm/changed/removed units, changed vocabulary
  defaults/controls, host-ID remapping, failures, cancellation and concurrent caches;
  stale/digest-collision/module/feature rejection is tested. All fifteen composition
  and seven CBOR consumer controls have exact acceptance-boundary sweeps; every
  artifact byte is adversarially mutated under independent validation.
  Fifteen producer-policy controls and thirteen JSON/scalar/shape controls now
  have independent acceptance-boundary sweeps. Ten compiler phase checkpoints
  inject cancellation and processing faults; failed generations never publish
  or replace a successful cache. Occurrence paths and field names use fallible
  copies with reservation-error tests. The vocabulary fuzz harness adds an exact
  four-bundle diamond through compilation, independent wire validation and probe.
  The configured five-target sanitizer runs completed outside the sandbox;
  successor source/wire/probe fuzz paths are included. The source-bound command is
  rerun on fixed inputs after concurrent edits; the automatic validated receipt,
  not raw logs or this checklist, is measurement authority.
  Added: fallible successor shared ownership, recursive contract/value copies,
  materialization boxes, scope/dependency ordered indexes and graph worklists.
  JSON reservation checkpoints test faults and mid-parse cancellation. Chain,
  fanout, diamond and unequal-path DAG tests cover deterministic/concurrent wire
  output and independent dependency controls, missing/extra/conflicting/private
  and cyclic dependencies.
  Added: fallible successor schema retention and shared CBOR lexical retention;
  each exercised reservation checkpoint rejects synthetic allocation faults and
  mid-parse cancellation. Wire catalogue joining is linear and borrows keys rather
  than cloning a tree. Successor cache syntax/token/type copies and pending indexes
  are fallible; retention faults preserve the previous generation. Exact cache
  byte and UTF-8 lexer boundaries are tested. Reader declaration copies and supplied
  origin reconstruction now reserve storage fallibly. Exact-number normalization
  reuses its fallibly reserved coefficient instead of allocating additional copies.
  Scope decision standalone legacy validation/allocation review is
  not required for this gate. Existing compatibility tests and behavior remain
  unchanged. Shared code called by the successor pipeline remains in scope;
  excluding standalone legacy work does not waive the successor's frozen
  fail-closed allocation contract.
  Added fallible resolver signature/value/active indexes, syntax/body/
  value retention, boxes and complete companion assembly. Successor compilation
  uses separately retained import topology and iterative fallible SCC worklists,
  not old-profile public graph owners. Reader companion/reuse/reachability indexes
  and public-view retention are fallible with charged index movement. Logical and
  interface framing borrow numeric restrictions and visibility facts; semantic
  edge retention is fallibly reserved and canonicalized without tree allocation.
  Source chains/diamonds/reference-type cycles agree with the existing graph API
  and independent wire/probe validation; exact SCC/work bounds, pre-cancellation
  and a long iterative import chain are tested. Syntax-first diagnostic precedence,
  diagnostic overflow and pre-resolution decoded-string bounds are guarded by
  regressions. Capture alias retention is fallible.
  Added: shared source/header/lock capture validation and the explicitly adapted
  old-schema leaf validator now use fallible indexes, copies and graph worklists.
  Shared capacity, box and owner operations have non-default, thread-local
  reservation failure observation. Actual capture/compiler/reader/view/identity/
  encoding/decoding/probe operations sweep every observed reservation, including
  defaults, reference cycles, vocabulary-backed values and old-leaf embedding.
  Warm and changed-unit cache sweeps preserve the prior successful generation.
  Real reservation-boundary cancellation found and fixed final capture publication
  after cancellation; compiler and consumer publication checks also pass.
  Eight independent capture controls have below/exact/one-over tests.
  `cargo xtask test all` and final CI pass 777 tests without skips and three
  compile-fail doctests. All five configured untraced sanitizer targets pass
  with the existing 15-second-per-target policy and a validated source-bound
  receipt; this is not a 900-second campaign. Reservation-error simulation is
  not physical whole-process memory exhaustion.
  Remaining for the broader allocation guarantee: successor `replay_request`
  still calls the infallible old request-copy adapter, and probe JSON presentation
  still uses infallible formatted strings/collections. Those operations were
  outside the processing reservation sweeps; they are not waived by excluding
  old-only validation. The broad checkbox remains open until these reachable
  adapters have fallible paths and corresponding fault tests.
  Evidence: [hardening and actual cache execution](evidence/stage7-contract-core.md#composition-hostile-boundaries-and-actual-cache-execution-07-10-2026).
- [ ] Verify semantic restrictions/defaults affect the appropriate identities,
  presentation does not, and existing identity profiles/vectors remain immutable.
  Implemented: separately typed captured identity /2 and unchanged /1 vectors.
  Production complete logical /2 matches its frozen independent transcript;
  interface /2 is independently recomputed by the reader. Remaining: derivation
  and artifact /2 layers and every frozen restriction/default/presentation
  variation, independent interface and root-invariance comparison.
- [ ] Freeze and implement closed tagged variants across source, vocabulary,
  semantic typing, IR, wire, reader, and identity; reject wrong/unknown tags and
  payloads and validate heterogeneous typed collections.
  Both source declarations and vocabulary declarations are required, sharing
  one semantic model; the [frozen contract](../specs/contracts/VARIANTS.md)
  and registered fixtures record requirements, not implementation completion.
  Implemented: vocabulary declarations and the common resolved record/variant
  model, source syntax/binding semantics, payload validation and heterogeneous
  lists through complete project/wire/logical identity. Remaining: complete
  identity partition and registered hostile/boundary validation.
- [ ] Freeze and implement cross-vocabulary public type dependencies and exact
  transitive lock closure, including diamond/conflicting/private/missing/cyclic
  dependency rules without compilation-time acquisition.
  Implemented: frozen rules, exact captured catalogue closure, diamond deduplication
  and conflicting/private/missing/cyclic rejection, now retained through source,
  complete project, wire and public-view closure validation. Remaining:
  full registered-oracle comparisons.
- [ ] Freeze required/omitted/null/default distinctions and safe origin facts;
  verify equivalent materialized meaning and distinct source evidence correctly.
  Implemented: frozen states, closed/binding-value materialization, safe origin
  paths, source reuse/ref edges and retained independently checked companions.
  Fine-grained original-byte source attribution, canonical vocabulary default
  owners and public-view redaction are implemented. Remaining: every registered
  omission/default evidence and identity equivalence comparison.
- [ ] Pass variant, cross-bundle closure, omission/default, old/new schema,
  public-view, hostile-input, exact/one-over, and independent identity suites.
  Existing catalogue/resolved-model passes are partial evidence only. Complete
  source-to-artifact and public-view tests now pass; complete hostile decoder/fault/
  fuzz, independent /2 identity and clean/cache suites must pass before activation
  or promotion.

## Stage 8 — dynamic authoring bridge and Editor probe

Target transition: `v0.8.0 -> v0.9.0`.

### `v0.8.1` — contract and fixture gate

- [ ] Freeze profile discovery, descriptor catalogue, overlay, editable model,
  mappings, projection, formatting, diagnostics, and authoring limits.
- [ ] Register core/vocabulary catalogue, model, stale-revision, projection,
  round-trip, and mismatch fixtures.
- [ ] Freeze [V1-CONS-001..010](../specs/contracts/CONSUMER-READINESS.md):
  operation/input/result/resource data, explicit prerequisites/output connections,
  structured conditions/deferred inputs, symbolic handles, capability/mapping
  descriptions, validation reports, and consumer/core ownership boundaries.
- [ ] Freeze field/element source attribution, typed member selectors, explicit
  entry discovery, self-contained artifact contracts, required-feature negotiation,
  and the producer/reader compatibility matrix with literal fixture oracles.
- [ ] Register project-neutral operation graph, map-entry, condition/output,
  unsupported-capability, explicit-alternative, source-attribution, privacy,
  and malformed/limit fixtures with literal expected consumer reports.

### `v0.8.2` — core implementation gate

- [ ] Implement deterministic static catalogue and project overlay.
- [ ] Implement closed bounded editable model with connections as the sole
  reuse/reference edge representation.
- [ ] Implement deterministic multi-source projection and mappings.
- [ ] Extend catalogue/model/projection for composite vocabulary fields,
  semantic defaults/restrictions, and consumer data connections; semantic
  restrictions remain compiler-owned, not presentation hints.
- [ ] Implement the closed data-only capability/report boundary and examples;
  do not add an executor, backend adapter, or implicit condition evaluation.
- [ ] Implement bounded public field-level attribution and typed selectors;
  preserve safe supplied/default/reuse origins without runtime evaluation.
- [ ] Implement explicit typed entry discovery and complete contract facts in
  standalone artifacts/views with exact profile/feature compatibility discovery.
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

### Promote to `v0.9.0`

- [ ] Confirm Stage 8 checklist, manifest, and traceability are complete.
- [ ] Confirm V1-VOC-005..011 and V1-CONS-001..010 have complete public evidence
  and no dependency on a separate product implementation.
- [ ] Release `v0.9.0`; update the ledger to `released`.

## Stage 9 — full conformance and `v1.0.0` qualification

Target transition: `v0.9.0 -> v1.0.0`.

### `v0.9.1` — complete contract and corpus gate

- [ ] Freeze all v1 requirements, contracts, diagnostic registries, examples,
  fixtures, oracles, vectors, manifest, and traceability.
- [ ] Confirm there are no planned active cases or unresolved portability
  exceptions.

### `v0.9.2` — complete implementation gate

- [ ] Implement every remaining v1 requirement and remove temporary feature
  gates that are not part of the public profile contract.
- [ ] Complete migration tooling/documentation without altering v0.1 meaning.

### `v0.9.3` — public consumer gate

- [ ] Pass independent Reader probe over the complete v1 corpus.
- [ ] Pass generic Editor probe over core and captured vocabulary features.
- [ ] Pass the project-neutral consumer boundary probe over composite vocabulary
  data, conditions/output connections, capability reports, source attribution,
  and identity separation without mapper execution or core-owned product semantics.

### `v0.9.4` — release validation gate

- [ ] Pass inherited v0.1 and complete v1 conformance suites.
- [ ] Pass formatter/linter/docs, unit/integration/system/property/fuzz,
  malformed/hostile input, limits, determinism, concurrency, soak, performance,
  security, dependency, and license gates.
- [ ] Pass all identity vectors and clean/incremental equivalence tests.
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
