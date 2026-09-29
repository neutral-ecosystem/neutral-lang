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

- [ ] Freeze repeated aliased `use`, exact lock coverage, public vocabulary
  types, metadata separation, and `url`/`path` behavior.
- [ ] Register multiple-alias, missing/extra/conflicting lock, private type,
  executable payload, and location-value fixtures.

### `v0.5.2` — core implementation gate

- [ ] Implement exact multi-vocabulary validation and canonical identities.
- [ ] Implement public vocabulary type enforcement.
- [ ] Implement distinct inert `url` and `path` source/IR values.

### `v0.5.3` — public integration gate

- [ ] Expose canonical vocabulary identities/revisions through IR and reader.
- [ ] Verify aliases remain source-local and non-semantic.
- [ ] Verify vocabulary metadata affects authoring presentation only.

### `v0.5.4` — validation gate

- [ ] Run strict schema, duplicate/unknown field, executable-content,
  lock-cover, alias, and location-value suites.
- [ ] Prove location values cannot fetch/open/normalize/authorize anything.
- [ ] Run hostile vocabulary decoder limits and fuzz tests.

### Promote to `v0.6.0`

- [ ] Confirm Stage 5 checklist, manifest, and traceability are complete.
- [ ] Release `v0.6.0`; update the ledger to `released`.

## Stage 6 — project IR, reader, and views

Target transition: `v0.6.0 -> v0.7.0`.

### `v0.6.1` — contract and fixture gate

- [ ] Freeze project IR, companions, reader validation, result envelopes,
  resource facts, view request, and dependency-closure rules.
- [ ] Register complete/private/public/view/malformed artifact fixtures.

### `v0.6.2` — core implementation gate

- [ ] Lower only fully valid projects to complete project IR.
- [ ] Implement source maps, provenance, derivation/resource facts, and public
  reader validation.
- [ ] Implement post-compilation view derivation without changing project IR.

### `v0.6.3` — public integration gate

- [ ] Implement standalone reader-only project probe.
- [ ] Verify views preserve all interpretive dependencies while redacting
  private implementation provenance.
- [ ] Verify roots never affect capture, logical equality, or project identity.

### `v0.6.4` — validation gate

- [ ] Run IR round-trip, hostile decoder, source-map, provenance, view, bounds,
  and independent reader-probe suites.
- [ ] Run clean/incremental and serialization-order equivalence tests.
- [ ] Review all public result and failure envelopes.

### Promote to `v0.7.0`

- [ ] Confirm Stage 6 checklist, manifest, and traceability are complete.
- [ ] Release `v0.7.0`; update the ledger to `released`.

## Stage 7 — canonical identity and reproducibility

Target transition: `v0.7.0 -> v0.8.0`.

### `v0.7.1` — contract and vector gate

- [ ] Freeze canonical logical form, domain tags, transcript framing, SHA-256
  usage, identity exclusions, and artifact derivation inputs.
- [ ] Publish literal captured/logical/derivation/artifact vector inputs and
  expected transcripts/digests.

### `v0.7.2` — core implementation gate

- [ ] Implement bounded canonical logical form and every identity layer.
- [ ] Keep graph-local IDs, host mappings, aliases, roots, capture order, and
  source evidence out of logical identity.
- [ ] Implement deterministic artifact identity by kind and format.

### `v0.7.3` — public integration gate

- [ ] Expose typed identities and derivation facts through public reader APIs.
- [ ] Produce identity vectors from a second independent implementation.
- [ ] Verify cache keys use the correct identity layer.

### `v0.7.4` — validation gate

- [ ] Compare both implementations for every accepted/adversarial vector.
- [ ] Run formatting/order/equivalence/non-equivalence/collision-path tests.
- [ ] Review transcript and identity-profile version immutability.

### Promote to `v0.8.0`

- [ ] Confirm Stage 7 checklist, manifest, and traceability are complete.
- [ ] Release `v0.8.0`; update the ledger to `released`.

## Stage 8 — dynamic authoring bridge and Editor probe

Target transition: `v0.8.0 -> v0.9.0`.

### `v0.8.1` — contract and fixture gate

- [ ] Freeze profile discovery, descriptor catalogue, overlay, editable model,
  mappings, projection, formatting, diagnostics, and authoring limits.
- [ ] Register core/vocabulary catalogue, model, stale-revision, projection,
  round-trip, and mismatch fixtures.

### `v0.8.2` — core implementation gate

- [ ] Implement deterministic static catalogue and project overlay.
- [ ] Implement closed bounded editable model with connections as the sole
  reuse/reference edge representation.
- [ ] Implement deterministic multi-source projection and mappings.

### `v0.8.3` — public integration gate

- [ ] Implement generic Editor probe from discovery only.
- [ ] Verify create, import, edit, project, compile, save, reopen, and no-op
  round trip without private parser/AST access.
- [ ] Verify compiler authority on authoring-preflight disagreement.

### `v0.8.4` — validation gate

- [ ] Run catalogue identity, overlay revision, limits, stale/cancelled request,
  formatting, diagnostic, and round-trip suites.
- [ ] Audit that no adapter or descriptor contains executable callbacks.
- [ ] Review authoring/core independent version compatibility.

### Promote to `v0.9.0`

- [ ] Confirm Stage 8 checklist, manifest, and traceability are complete.
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
- [ ] Pass Flow-boundary probe proving data access without mapper execution or
  Neutral-owned CI/CD semantics.

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
