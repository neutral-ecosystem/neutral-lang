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

### `v0.8.0` — vocabulary composition extension gate

Scope: [V1-VOC-005..011](../specs/contracts/CONSUMER-READINESS.md).
Previously completed `.1`–`.4` identity gates remain historical evidence;
this additional contract -> implementation -> integration -> validation gate
must pass before the unreleased promotion. No frozen schema is amended in place.

The [composition proposal](../specs/contracts/VOCABULARY-COMPOSITION.md) defines
the candidate bundle shape and semantics; its remaining freeze work is explicit.
The vocabulary crate's `compatibility_*` tests protect the old schema but do not
count as implementation or validation of the extension.

- [x] Implement and test the separate composition catalogue boundary and shared
  raw IR model: composite vocabulary types, variants, exact dependency closure,
  closed defaults/restrictions, presence distinctions, limits and cancellation.
  This sub-step does not activate source syntax, project IR/wire/identities,
  reader/probe integration, or full extension conformance.
- [x] Expose public composition catalogue facts through compiler-independent
  reader APIs, with exact revision lookup and private-type/debug redaction.
  Complete project/wire/probe/view/provenance integration remains pending.

- [ ] Freeze the new bundle schema, source-aligned list/nullable/reference
  composition, semantic defaults/restrictions, compatibility, diagnostic codes,
  public closure, recursive-type rules, and independent bounds.
- [ ] Register literal positive/negative/boundary/migration fixtures and new
  identity vectors before implementation; pin reviewed inputs at activation.
- [ ] Implement composition, closed defaults, finite choices, numeric and
  string/list length restrictions through compiler, IR, encoding, and reader.
- [ ] Expose complete new contract facts and reference dependencies through
  independent reader/probe APIs with public-view closure and source attribution.
- [ ] Pass hostile schema/decoder, invalid default/constraint/reference,
  exact/one-over, cancellation, ordering, clean/cache, and old-schema suites.
- [ ] Verify semantic restrictions/defaults affect the appropriate identities,
  presentation does not, and existing identity profiles/vectors remain immutable.
- [ ] Freeze and implement closed tagged variants across source, vocabulary,
  semantic typing, IR, wire, reader, and identity; reject wrong/unknown tags and
  payloads and validate heterogeneous typed collections.
  Both source declarations and vocabulary declarations are required, sharing
  one semantic model; the [contract proposal](../specs/contracts/VARIANTS.md)
  and proposed fixtures record the accepted ownership decision, not completion.
- [ ] Freeze and implement cross-vocabulary public type dependencies and exact
  transitive lock closure, including diamond/conflicting/private/missing/cyclic
  dependency rules without compilation-time acquisition.
- [ ] Freeze required/omitted/null/default distinctions and safe origin facts;
  verify equivalent materialized meaning and distinct source evidence correctly.
- [ ] Pass variant, cross-bundle closure, omission/default, old/new schema,
  public-view, hostile-input, exact/one-over, and independent identity suites.

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
