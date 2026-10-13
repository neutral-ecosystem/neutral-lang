<!-- SPDX-License-Identifier: Apache-2.0 -->

# Typed Entry Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for native execution, or superpowers:subagent-driven-development if the user chooses delegation. Implement and verify each task before starting its dependent task. Do not mark the Stage 8 gate complete from planning alone.

**Goal:** Close the Stage 8 typed-entry/standalone-contract/compatibility gate with independently decoded, bounded reader and probe APIs.

**Architecture:** Build on `ConsumerBoundary`, the validated composition project and its existing public closure implementation. Discover exact public binding contracts without changing compilation, wire /2 or complete project identity. Share exact profile facts between producer, reader and probe; negotiate consumer support separately from standalone source availability and authoring.

**Tech Stack:** Existing Rust workspace, fallible allocation/ordered collections from `neutral-core`, Nextest and the existing file-backed conformance harness. No new dependency.

**Spec:** [Frozen consumer protocol](../../../portable/specs/contracts/CONSUMER-PROFILE.md), especially entry discovery, compatibility and independent bounds; [consumer readiness](../../../portable/specs/contracts/CONSUMER-READINESS.md), V1-CONS-008..010; [implementation gate](../../../portable/development/07-IMPLEMENTATION-CHECKLIST.md).

## Global constraints

- Preserve `neutral.project-ir/2`, `NIR-PROJECT-CBOR/2`, `neutral.project-identity/2` and historical vectors; no new serialized field or grammar.
- Entry schema is `neutral.entry-request/1`. Selection is enumerate, unique or explicit; entries are public bindings of exactly the requested public nominal type/revision, never type declarations.
- Enumerate and successful roots sort by stable symbol identity. Explicit empty roots select an empty view. Duplicate explicit roots reject; no first-match entry selection.
- Required consumer features are sorted/unique `entry-discovery-v1`, `member-selectors-v1`, `value-attribution-v1`. Unknown required features and unsupported exact tuples reject without fallback.
- Additional hard ceilings: 1,000,000 roots/candidates/closure edges each; 67,108,864 request/report bytes; 1,048,576 bytes per identifier; 100,000,000 work visits/emitted bytes. Intersect caller, retained reader and hard policies. Zero controls reject; empty request collections remain valid where specified.
- Use existing error codes `NEU-CONS-001..011`; never put supplied/private names or values in errors/debug output. Preserve the current private-or-absent lookup policy.
- Fallible reservation/copy and cancellation must cover selection, closure and output before publication. No compiler linkage, source parsing, registry lookup, execution or package acquisition in independent APIs.
- Future v2 table/alias proposals are outside this implementation. Keep frozen contract/fixture bytes unchanged.

## Review focus

1. A public type declaration must not be mistaken for an entry; a private binding with a matching type must not appear or leak through failure text (Task 1).
2. Consumers with correct nominal names but a different vocabulary revision, unknown wire profile or missing required feature must reject before selection (Tasks 1 and 3).
3. Two spellings/orderings of explicit roots must not select different meanings; duplicates and empty roots need independent oracles (Tasks 1 and 2).
4. A tiny selected value can depend on large defaults, unselected variant alternatives or cyclic reference closure. Bounds must apply before copying and views must retain interpretive facts (Task 2).
5. Failure/cancellation after some candidates or closure nodes were retained must publish nothing, preserve the reader and allow the next request to succeed (Tasks 1, 2 and 4).

## Files and ownership

- Create `crates/neutral-reader/src/consumer/entries.rs`: typed requests, bounded entry discovery and selected-entry result.
- Modify `crates/neutral-reader/src/consumer/mod.rs` and `validation.rs`: exports and independent entry controls using the existing budget/error helpers.
- Modify `crates/neutral-reader/src/composition/view.rs` and `composition.rs`: bounded public closure API, retaining the existing `derive_view` entry point as a default-policy wrapper.
- Modify `crates/neutral-ir/src/composition/profile.rs`: shared exact composition profile facts, reusing existing constants rather than package versions.
- Create `crates/neutral-reader/src/consumer/compatibility.rs`: explicit consumer-profile discovery/negotiation over implemented tuples.
- Modify `crates/neutral-probe/src/composition.rs`: typed encoded-entry inspection using independent decode and reader selection.
- Create `crates/neutral-test-suite/tests/composition_contract/pipeline/entries.rs` and register it in `pipeline/mod.rs`.
- Extend owning reader/probe/core tests only where needed, plus the consumer README and Stage 8 checklist after verification.

## Task 1: Bounded typed entry selection

**Interfaces:** Add `ENTRY_REQUEST_SCHEMA`; `EntrySelection<'a> { Enumerate, Unique, Explicit(&'a [ModuleSymbolIdentity]) }`; `EntryRequest<'a> { schema: &'a str, contract: &'a ProjectPublicType, selection: EntrySelection<'a> }`. Add `ConsumerBoundary::entries(&EntryRequest<'_>, &CancellationToken) -> Result<SelectedEntries<'p>, ConsumerError>`. `SelectedEntries::bindings()` borrows canonical checked `CompositionBinding` references, never copies private authority.

- [x] Write decoded-fixture tests in `pipeline/entries.rs` using the existing `consumer.neu` fixture and round-trip helper. `Capability` enumeration must yield exactly `copied`, `description`, `entry`; the public type itself and `private_description` must not appear. Unique `Capability` returns `AmbiguousEntry`; unique `Receipt` returns `MissingEntry`.
- [x] Test explicit reverse-order roots return canonical order, duplicate roots return `InvalidInput`, empty roots return no bindings, a public type declaration is not a binding entry, wrong public contract returns `TypeMismatch`, and private/absent targets retain the existing safe failure classification. Test exact vocabulary revision using the existing vocabulary fixture.
- [x] Run `cargo nextest run --config-file config/nextest.toml --package neutral-test-suite -E 'test(pipeline::entries::)'` and observe failures caused by the missing feature before implementation.
- [x] Add independent `roots`, `candidates`, `closure_edges` controls to `ConsumerLimits`, intersect/validate them in `ConsumerBoundary::new`, and implement the typed API using `Budget`, checked public-contract authority and fallible vectors. Count matching candidates before retention and enforce unique cardinality; never select the first match.
- [x] Test exact/one-over candidates and roots, malformed schema/controls, identifier/request/output/work bounds and cancellation. Sweep every observed entry allocation/cancellation boundary and require subsequent successful selection against the same reader.
- [x] Rerun the focused tests and existing consumer/selector tests; inspect the diff before continuing.

## Task 2: Public closure with independent pre-retention controls

**Interfaces:** Add `CompositionViewLimits { roots: u64, closure_edges: u64, work: u64, output_bytes: u64 }` with positive default hard controls. Add `ValidatedCompositionProject::derive_view_with_limits(&CompositionViewRequest, CompositionViewLimits, &CancellationToken) -> Result<CompositionView, CompositionReadError>`. Existing `derive_view` delegates with defaults. Add `SelectedEntries::derive_view(&CancellationToken) -> Result<CompositionView, ConsumerError>` using canonical selected owners and the intersected remaining request policy.

- [x] Write failing tests that selected empty roots produce empty declarations/vocabularies/references, selected private-reuse values remain visible but private origins do not, and a selected reference/variant retains all public interpretive type/default/restriction/dependency facts. Reuse existing source/variant/reference fixtures rather than invent an untyped artifact.
- [x] Write exact/one-over closure-edge and retained-output tests. Verify a large selected payload is rejected before its deep copy; a count-only root check is insufficient. Exercise cyclic reference closure, transitive bundles, unselected variant alternatives and a cancellation during closure.
- [x] Run the focused tests and observe the missing bounded API/behavior fail.
- [x] Extend the existing `Closure` accounting instead of duplicating graph traversal. Charge each dependency edge before following it; check output-retention size with a bounded borrowed walk before copying selected declarations, values/defaults, bundle contracts, origins and reference paths. Include owned strings, vectors/nodes and root/schema storage, not only shallow struct sizes. Counted retained output is distinct from serialized transport bytes.
- [x] Keep all copies fallible and closure sets iterative/cycle-safe. Map exhausted controls/allocation to `Limit`, cancellation to `Cancelled`, and missing required interpretive facts to `IncompleteContract` at the consumer boundary. Never return partial views.
- [x] Sweep selection/view allocation and cancellation checkpoints, verify prior complete identity remains unchanged, then rerun all existing view and independent reader/probe tests.

## Task 3: Exact profile and feature discovery

**Interfaces:** Add a shared, immutable composition-profile descriptor in `neutral-ir::composition::profile`, built from current source/capture/bundle/IR/view/encoding/identity constants. In `consumer::compatibility`, expose `CONSUMER_PROFILE`, the three ordered consumer feature identifiers, `consumer_compatibilities()` and `negotiate_consumer(&ConsumerCompatibilityRequest<'_>, ConsumerLimits, &CancellationToken) -> Result<&'static ConsumerCompatibility, ConsumerError>`. The request explicitly names source, capture, bundle schema, project/view schema, encoding, identity, consumer profile, project/consumer required features and optional authoring profile.

- [x] Write failing table-driven tests for the frozen compatibility matrix: legacy document and project /1 reject consumer /1; complete composition /2 with all required features succeeds; unknown encoding/identity/project feature, missing consumer feature, duplicate/unsorted feature list and unsupported authoring selection reject. Use literal unsupported tuple values, not values manufactured from the same discovery helper.
- [x] Distinguish explicit captured-project availability from standalone `LanguageProfile::V1_0` availability; do not mark standalone v1 compilation available. A headless request must not require authoring.
- [x] Run the focused test to observe failure, then implement allocation-free exact matching and bounded feature validation. Reuse existing canonical version constants. Represent explicitly adapted legacy vocabulary leaves separately from native successor bundles; do not advertise an unimplemented bundle/profile combination.
- [x] Do not advertise `neutral.consumer/1` as available until Tasks 1, 2 and 4 pass public integration. No evaluator, adapter or JSON transport support is implied by this profile.
- [x] Add consumer-independent public profile/required-feature facts to `CompositionView` via immutable accessors over the shared descriptor; retain actual exact vocabulary revisions/dependencies in its existing catalogue. Test complete facts after independent decoding, and test that required-fact corruption fails before selection.
- [x] Rerun profile, consumer, selector and entry tests; ensure authoring and presentation metadata never become required semantic inputs.

## Task 4: Standalone encoded-artifact entry inspection

**Interfaces:** Add `neutral_probe::composition::inspect_composition_entries_encoded(bytes: &[u8], wire: DecodeLimits, project: ProjectLimits, composition: ProjectCompositionLimits, compatibility: &ConsumerCompatibilityRequest<'_>, request: &EntryRequest<'_>, limits: ConsumerLimits, cancel: &CancellationToken) -> Result<CompositionProbeSummary, CompositionProbeError>`. Group policy/request arguments into one public options struct if required by workspace lint conventions; retain their explicit meanings. Add a safe consumer error variant to `CompositionProbeError`. Factor shared summary construction from the existing inspect function; do not duplicate identity/fingerprint logic or change existing CLI root behavior.

- [x] Write a failing integration test that encodes a compiled file-backed fixture, passes only the bytes/request to the new probe API, selects one exact typed entry and checks roots, contract closure, redaction and complete identity. Enumeration, explicit empty, ambiguous and cancelled requests need literal outcomes.
- [x] Run the test to observe the missing API fail, then implement compatibility negotiation, independent decoding, validation of actual decoded profile facts against the negotiated tuple, and the reader boundary/bounded view projection. Requested metadata is not authority for decoded bytes. No compiler dependency may be introduced into the probe.
- [x] Test truncated/corrupt/missing-contract artifacts, unknown required profile/features, private roots and source bytes being unavailable. Reject before accepted summary publication; do not fetch missing facts or reparse source.
- [x] Sweep the full decode/entry/view/probe call's observed allocation/cancellation boundaries and verify a subsequent successful call. Keep errors/debug output free of requested names/private data.
- [x] Run existing probe tests and `cargo xtask check` to verify dependency boundaries and unchanged legacy behavior.

## Task 5: Verification and honest gate closure

- [x] Run `cargo fmt --all`, then `cargo clippy --workspace --all-targets --all-features -- -D warnings`; fix findings without suppressing lints.
- [x] Run `cargo xtask test all` and `cargo xtask ci pr`. Record failing tests by name if any; focused passes alone do not satisfy completion.
- [x] Compare the entry/compatibility production outcomes to the existing frozen literal consumer oracles without editing their bytes. Add missing mutable runtime-owned fixtures/tests where needed; frozen registration is not implementation evidence.
- [x] Update the consumer README with one real typed-entry example and exact compatibility/availability limits. Update probe documentation for the new typed API without claiming an unimplemented CLI transport flag.
- [x] Mark the single requested item in `07-IMPLEMENTATION-CHECKLIST.md` `[x]` only if all tasks pass; add a short verified scope summary. Otherwise leave `[ ]` with a precise remaining gap. Do not activate unrelated expression/authoring/evaluator gates or claim a new release.
- [x] Review the complete diff, run `git diff --check`, and hand off without committing/pushing unless the user requests it. Long mutation/fuzz/soak campaigns are separate qualification work; report whether they were run, never imply that ordinary tests replace them.

## Approval / execution

User approved native execution with “proceed”; implementation remains in the
existing main checkout without commit/push. Tasks 1–4 are implemented and their
focused tests pass. Final review findings were fixed with demonstrated failing
regressions for work accounting and deep unused-default retention. Task 5:
formatting, workspace Clippy, all 902 tests plus doctests, and all seven CI PR
quality gates pass after the checklist update. No long mutation/fuzz/soak campaign
or expression `/3` activation is claimed.
