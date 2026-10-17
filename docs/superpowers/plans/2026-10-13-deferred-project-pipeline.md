<!-- SPDX-License-Identifier: Apache-2.0 -->

# Deferred Project Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for native implementation after this written plan is reviewed. Work continuously through these dependent tasks; do not ask for another approval between tasks. Each task requires a demonstrated failing test followed by a passing test. The final gate cannot be checked from raw-model tests alone.

**Goal:** Close the Stage 8 deferred-input retention gate through source compilation, self-contained artifacts, independent readers and explicit consumer binding/report APIs, without a scheduler or implicit evaluation.

**Architecture:** Keep the existing raw expression model and historical composition pipeline unchanged. Add a separate complete `/3` project type, and a compiler-independent `neutral-semantics` library shared by compiler and reader for expression typing, restriction obligations, normalization and explicit evaluation. Reader validation adds complete companion/identity checks; consumer APIs expose derived guarded dependencies and explicit policy/data without interpreting ordinary fields as built-in scheduling commands.

**Tech Stack:** Existing Rust workspace, fallible `neutral-core` ownership, exact normalized `num`, restricted CBOR, NHT/SHA-256 and Nextest. No new external dependency is assumed. The new internal crate is a real shared semantic boundary, not a second parser, wire codec or executor.

**Spec:** Frozen [expression compatibility](../../../portable/specs/contracts/EXPRESSION-PROFILE.md), [expressions](../../../portable/specs/contracts/EXPRESSIONS.md), [evaluator](../../../portable/specs/contracts/EVALUATOR.md), [expansion](../../../portable/specs/contracts/EXPANSION.md), [consumer protocol](../../../portable/specs/contracts/CONSUMER-PROFILE.md), and [implementation checklist](../../../portable/development/07-IMPLEMENTATION-CHECKLIST.md). Semantic approval is already recorded; no new language syntax is proposed here.

## Global constraints

- Preserve `/1` and `/2` public behavior, types, source exclusions, CBOR frames, domains and vectors. No null placeholder for an input and no conversion through a fake successful `/2` project.
- Select capture `/3`, IR/view/result `/3`, identity `/3`, `NIR-PROJECT-CBOR/3` explicitly. Source header remains `neu "1.0"`; it cannot activate capabilities. The exact sorted feature set is `decimal-division-v1, finite-expansion-v1, pure-evaluator-v1, pure-expressions-v1, tagged-variants-v1, vocabulary-composition-v2`.
- Use `neutral.expression/1`, `neutral.evaluator/1`, `neutral.expansion/1`, `neutral.decimal/1`. Decimal division rounds nonterminating results to 34 significant digits, nearest/ties-to-even; terminating results stay exact within bounds. Never use host floats or reordering across rounded operations.
- Raw shape inspection is only preflight. Independently retype every branch, binder and selector, including unreachable branches. Evaluate only selected branches; a missing guard demands only its own bindings.
- Missing, typed null, unavailable and failed are distinct. Validate all supplied envelopes before demand traversal. A well-formed unavailable/failed binding affects only a demanded path; a malformed supplied binding is never ignored.
- Recompute restrictions/availability obligations and guarded potential dependencies from trees and retained contracts. Do not add authoritative supplied dependency/obligation graphs to the fixed wire envelope.
- Structural refs, imports, guarded value dependencies and explicit prerequisites remain separate. No built-in `needs`, `Operation`, backend identifier, executor, scheduler, I/O, environment/clock lookup, credential resolution or callback.
- Intersect independent producer, reader, expression, expansion, evaluator and consumer controls. Preserve the frozen eleven expression and thirteen expansion limit dimensions; account before recursion/allocation/expansion, check cancellation during retention and publish atomically.
- Existing frozen expression files remain immutable. Runtime regression fixtures live in owning `tests/` trees; production cannot depend on `portable/` being present. Do not erase measured evidence or modify Git history to pass a check.
- Do not advertise full `/3` capture until every required feature, including finite expansion, is implemented. Typed low-level facilities may be tested before public activation but must not masquerade as complete compatibility support.
- Authoring GUI/editable-model implementation is outside this gate. Full source-profile/authoring activation remains subject to its separate frozen activation requirements; do not mark unrelated Stage 8 authoring items complete.
- Work in the user's requested main checkout, preserving existing uncommitted foundation changes. No commit, push, tag, release or external consumer execution is authorized by this plan.

## Review focus

1. A forged node result type or private type/output target in an unselected branch must fail independent typing (Tasks 2 and 6).
2. Future list/tag/optional-field availability and field restrictions must survive wire/view projection and be enforced when bindings arrive (Tasks 2, 7 and 8).
3. Missing guards, malformed skipped bindings, unavailable outputs and failed bindings have different outcomes; no partial accepted value or eager branch demand is permitted (Tasks 4 and 8).
4. Changing external bindings, roots, source order or private provenance must not change complete logical identity incorrectly or contaminate a later request/cache generation (Tasks 5, 8 and 10).
5. A filtered matrix, hostile decoder or cancelled deep copy cannot reset aggregate budgets, allocate beyond controls or publish a passing prefix (Tasks 6, 9 and 11).

## Ownership and dependency direction

- `neutral-ir::expression`: raw complete data, exact selectors, limits/facts, companions and typed `/3` identity data. No semantic validity claims.
- New `crates/neutral-semantics`: depends only on `neutral-core`, `neutral-ir`, `neutral-vocabulary`. Owns complete expression-model typing, obligations, exact decimal operations and explicit pure evaluation. No dependency on compiler/reader/encoding/CLI.
- `neutral-compiler`: adds a dependency on `neutral-semantics`; owns explicit capture `/3`, source parsing/name resolution, static normalization, hygienic compile-time expansion and compilation cache publication.
- `neutral-reader`: adds a dependency on `neutral-semantics`; independently validates complete decoded data/companions/identities and exposes public views/consumer APIs. It never calls the compiler.
- `neutral-encoding`: owns new expression transport under its distinct magic; share checked composition scalar/type/contract/companion helpers rather than copying the `/2` codec or changing its accepted grammar.
- `neutral-probe`: uses encoding/reader only. New typed APIs inspect complete `/3` data without source parsing or evaluation. Existing CLI behavior remains unchanged unless a separately documented profile-compatible dispatch is implemented and tested.

Update `Cargo.toml`, owning manifests/READMEs, dependency-boundary enforcement and its tests when introducing the internal crate. Inspect the existing boundary checker before editing its package catalogue; do not weaken rules globally.

## Task 1: Complete raw `/3` project and shared semantic boundary

**Files:** Create `crates/neutral-ir/src/expression/project.rs`, `templates.rs`, `attribution.rs`; extend `expression/profile.rs`. Create `crates/neutral-semantics/Cargo.toml`, `src/lib.rs`, `src/model.rs`, owning tests. Modify workspace and compiler/reader dependency manifests only as required.

**Interfaces:** `ExpressionDeclaration { identity, public, signature, body: Option<ExpressionBody> }`; `ExpressionProjectIr` directly retains the sixteen historical section roles with expression declarations plus exact semantic tuple, public templates, expression/expansion limits, seven recomputable facts and expression/expansion attribution. Nominal definitions require no body; bindings require exactly a value/input/expression body. Reuse existing module/type/value/vocabulary/source/companion structures rather than inventing a second nominal model. Add `ExpansionLimits` in its frozen tuple order and `PublicTemplate` with named ordered static/deferred parameters, optional defaults, result type and body.

- [x] Write `project_inputs_have_no_fabricated_value`, `definitions_reject_binding_bodies`, and `project_profile_selection_never_falls_back`; require literal distinct bodies, exact tuple and no historic downcast.
- [x] Run focused owning tests; observe the missing APIs fail.
- [x] Implement raw structures and shared crate boundary, with bounded fallible copying/shape preflight and no `/3` availability advertisement. Add production comments explaining raw versus validated authority.
- [x] Run owning tests and `cargo xtask check`; dependency rules must retain compiler-free reader/probe behavior.

## Task 2: Independent expression typing and retained obligations

**Files:** `neutral-semantics/src/validation/{mod,types,trees,scope,obligations,facts}.rs`; owning `tests/validation/`. Reuse `validate_composition_scope`, closed defaults/restriction validation and stable symbols from existing libraries. Extract only genuinely shared pure contract checks; do not validate expressions by materializing fake bindings.

**Interfaces:** `validate_expression_model(Shared<ExpressionProjectIr>, SemanticLimits, &CancellationToken) -> Result<ValidatedExpressionModel, ExpressionValidationError>`. `SemanticLimits` contains named project/composition/expression/expansion policies. `ValidatedExpressionModel` has private fields and exposes borrowed exact declaration/type authorities, recomputed facts and `obligations(owner)`. `RestrictionObligation` binds an expression owner/node path to the actual nominal field contract; `AvailabilityObligation` retains exact selector presence/index/alternative checks. Neither is an executable predicate or a caller-owned assertion.

- [x] Write `unselected_branch_forged_type_is_rejected`, `bound_scope_depth_slot_and_type_are_checked`, `public_expression_cannot_leak_private_owners`, `match_requires_every_exact_alternative`, and `output_selector_retains_future_obligations`. Assert named NEU-EXP classifications and literal obligation paths/types.
- [x] Observe failures before implementing validation. Validate all types/operators/branches, exact nominal revision, contextual null/list/record/variant values, complete public closure and lexical frames. Reject static value cycles, dangling inputs and illegal output selectors; keep producer/result association explicitly consumer-owned.
- [x] Write/implement independent fact recomputation for all seven frozen facts, including wire-text counts; reject forged producer facts rather than trusting raw shape counts. Public template bodies/defaults are now independently typed and included in literal retained-fact tests.
- [x] Run negative/exact-one-over/cancellation/allocation sweeps. Retain previous accepted model and require a subsequent successful request after every failure.

Qualification, 14-10-2026: `tests/validation/{qualification,facts,templates,template_obligations,hardening}.rs`
cover exact public vocabulary revisions, nominal/list invariance, dangling targets,
selector rejection, all-branch/frame typing and recovery.
Input-only signatures reject non-nominal reference targets and nested nullable
wrappers before granting model authority; valid list/nullable/nominal-ref combinations
remain accepted. Model controls are tested
independently at inclusive boundaries for nodes, syntax/evaluation depth, declarations,
bindings, edges, modules, numeric growth, type depth and aggregate template definitions/
parameters. Work/storage threshold tests and observed reservation/cancellation sweeps
retain accepted authority. Ordinary source reuse cycles remain compiler-owned and
are exercised by the workspace source tests; model validation grants no source,
companion, transport or expansion-application authority.

## Task 3: Bounded deterministic decimal and typed value operations

**Files:** `neutral-semantics/src/decimal/{mod,coefficient,division}.rs`, `src/values.rs`; owning arithmetic tests. Reuse `ExactNumber` as the public stored representation and existing fallible byte/vector helpers. No general floating-point dependency.

**Interfaces:** Private decimal operations accept two exact numbers, a shared request budget and cancellation; return an exact normalized number or stable NEU-EXP failure. One shared budget counts intermediate digits/scale/work/storage, not only the final number. Expose operations only through the evaluator API in Task 4.

- [x] Write independent literals: `1/8 = 0.125`, `1/3 = 0.3333333333333333333333333333333333`, negative division, carry rounding, ties-even vectors, exact terminating results longer than 34 digits, zero canonicalization and division by zero. Use the existing rational oracle results, never host-float expectations.
- [x] Observe failures, then implement exact add/subtract/multiply/negate/compare and bounded quotient/remainder rounding. Preserve left-to-right expression semantics and checked intermediate limits.
- [x] Test typed structural equality, string UTF-8 ordering, homogeneous concatenation and scalar `text` conversion with decoded-byte limits. Test numeric/byte/work controls exactly at and one over, cancellation and reservation failures.

Qualification, 14-10-2026: owning value tests cover positional lists, nested records,
omission/null/presence, variant tags/payloads, stable refs and unnormalized UTF-8.
New literal UTF-8 and scalar-text boundaries exposed missing pre-allocation output
checks in `text(string)` and list concatenation; shared operations now reject before
proportional retention. Construction reservation/cancellation sweeps recover on the
same immutable inputs; existing independent decimal and numeric work/scale tests pass.

## Task 4: One explicit pure evaluator

**Files:** `neutral-semantics/src/evaluation/{mod,bindings,walk,results}.rs`; owning `tests/evaluation/`.

**Interfaces:** `BindingKey` is an input symbol or exact output target/result-selector key. `BindingState` is typed value, unavailable, or failed; a missing key is absent from the request. `EvaluationRequest` contains a selected owner, exact profiles, explicit binding slice, explicit output-association policy and positive controls. `evaluate(&ValidatedExpressionModel, &EvaluationRequest, &CancellationToken) -> EvaluationResult`, with closed value/required-bindings/unavailable/failure outcomes. No callback or implicit acquisition parameter.

- [x] Write `missing_guard_demands_only_guard`, `unselected_output_is_not_required`, `malformed_supplied_skipped_binding_fails_preflight`, `failed_and_unavailable_are_not_null`, and `missing_operands_are_sorted_unique`. Include `false && division_by_zero`, true selected division failure, and exact match payload binding.
- [x] Observe failures, then validate the complete binding envelope before traversal. Enforce actual contract restrictions and selector availability when values arrive. Traverse demanded operands left-to-right, collect independent missing keys, stop on demanded failures/unavailability, and never expose partial values.
- [x] Add static normalization using this same evaluator, not a compiler-specific arithmetic path. Preserve potentially failing expressions beneath unresolved guards; validate every branch before folding.
  Producer folding shares the kernel and returns owned normalized residuals.
  Unconditional static operands fold; deferred branches stay unevaluated and
  eliminated match payload frames close without capturing retained local binders.
  Compiler integration and complete project fact/companion publication remain Task 9.
- [x] Run independent literal results, exact bounds, fault/cancellation and repeated binding-generation isolation tests. Do not implement a persistent evaluation cache unless required by an existing caller.

Qualification, 14-10-2026: `tests/validation/evaluation_qualification.rs` compares
static folding and explicitly bound evaluation with independent numeric literals,
including rounded nontermination, carry and long exact terminating division.
It checks every zero expression control, exact nodes/depth/ports/edges/digits/scale/
output controls, work/storage boundaries, typed optional vocabulary record equality,
and observed numeric evaluator allocation/cancellation recovery. Binding generations
preserve missing/failed/unavailable distinctions and prior results after replacement.
Diagnostics are one closed failure, not a diagnostic collection; no persistent cache
or complete source-to-artifact/consumer qualification is claimed here.

Tasks 2–4 final verification, 14-10-2026: 74 focused semantic tests and all 1,034
workspace tests pass with zero skipped, plus doctests. `cargo xtask ci pr` passes
all seven quality steps on the final code and tracking inputs. Long campaigns are
not rerun; earlier receipts are stale after these fixes and do not close Task 11.

## Task 5: Complete `/3` identities and companion authority

**Files:** `neutral-ir/src/project_identity/expression/{mod,captured,logical,derived,interface}.rs`; `neutral-reader/src/expression/{mod,project,companions}.rs`; owning identity/reader tests. Reuse NHT framing/SHA-256 and shared historical type/value/source rules.

**Interfaces:** Distinct typed captured/logical/interface/derivation/artifact `/3` identities, using frozen domains and frame order. `ValidatedExpressionProject::from_ir(Shared<ExpressionProjectIr>, SemanticLimits, &CancellationToken) -> Result<Self, ExpressionReadError>` checks complete model, companions, resources and independently recomputed interface digest before publication. `model()` borrows validated semantics; no downcast to a `/2` authority.

- [x] Compare all five partitions against complete frozen minimal/residual/template vectors, not just node fragments. Reject stale digests/facts, bad attribution owners/paths/ranges and incomplete contract closure.
- [x] Prove roots and external binding values do not change complete logical identity; source evidence affects only its governed partitions. Check semantic profile, restriction/default and operand-order variations with independent literals.
- [x] Preserve safe public original-byte attribution; expanded source maps use original application/definition/argument/axis links, never fabricated generated source bytes. Run malformed companion/fault/cancellation tests and historical vector regressions.

Checkpoint, 14-10-2026: `cargo xtask test all` passes 1,013 tests with zero skipped
and doctests; `cargo xtask ci pr` passes all seven quality steps. Owning tests
compare complete five-part literal vectors, independently inspect restriction/default
frames, reject cross-nominal reuse and forged default ownership, and sweep reader
allocation/cancellation failures followed by successful recovery. The reader checks
retained expansion lineage consistency; omitted private-template bytes cannot prove
historical instantiation and are not invented. Work stops here at the user's request.
Later-task code already present is not qualification or activation evidence; Tasks
6–11 and the complete deferred-retention gate remain open. No long campaigns run.

## Task 6: Distinct complete expression transport

**Files:** `neutral-encoding/src/expression/{mod,trees,project,companions}.rs`; export from `src/lib.rs`; owning expression decoder tests. Factor shared private composition helpers into a neutral helper module only when used by both codecs; leave `/2` acceptance unchanged.

**Interfaces:** `encode_expression_project(&ValidatedExpressionProject, &CancellationToken) -> Result<Vec<u8>, EncodingError>` and `decode_expression_project(bytes, DecodeLimits, SemanticLimits, &CancellationToken) -> Result<ValidatedExpressionProject, DecodeError>`.

- [x] Write literal comparisons for magic `NEUP3\r\n\x1a`, exact 22-position arity, all node/body tags and complete registered CBOR vectors. Old readers reject new magic and new readers never retry old bytes as expressions.
- [x] Implement exact tuple grammar; decode all sections then independently validate before exposing any root. Reject nonminimal/truncated/trailing/oversized/unknown tuples, malformed binders and forged annotations, including unselected branches.
- [x] Sweep allocation/cancellation at every observed decode boundary, test each independent retention limit exactly/one-over, and require recovery on unchanged valid bytes. No failed decode may return a partial project/view.

Qualification (17-10-2026): owning transport tests compare hand-encoded node/operator
tuples and all three complete frozen artifacts, reject each of the 45 zero producer
and caller controls, and exercise literal semantic plus wire boundaries. Reservation
and cancellation sweeps require the injected checkpoint to be reached and valid bytes
to recover. The file-backed expression pipeline exercises 37 positive retained-count,
storage and work controls with multiple modules, diamond vocabulary closure and
multi-axis templates. This caught and fixed expansion attribution using producer
limits instead of the effective caller/producer intersection. Compile-time expansion
visitation/depth and later evaluator output controls are execution requirements, not
invented retained history; their qualification remains with the owning tasks.
`cargo xtask test all` passes (1,300 tests, zero skipped, plus doctests); full workspace
Clippy and `cargo xtask ci pr` pass, including documentation generation. Frozen bytes
and historical codec acceptance are unchanged.

## Task 7: Public closure, entry discovery and guarded dependency exposure

**Files:** `neutral-reader/src/expression/{view,dependencies,selectors}.rs`, `src/consumer/expression.rs`; owning reader tests. Share pure selector/contract traversal with existing consumer selectors where possible; do not duplicate semantic type checking in the consumer adapter.

**Interfaces:** A validated expression project exposes `dependencies(owner, limits, cancel) -> Result<GuardedDependencies, ExpressionReadError>` and a bounded redacted public view. Each dependency retains canonical consuming site, exact input/output key, and guard path conditions (Boolean branch or match tag) derived from the validated tree. Guard facts reference validated expression sites, not executable strings. Separate accessors expose structural refs and consumer-selected ordinary prerequisite data.

- [x] Write literal dependency oracles for nested if/match/short-circuit branches; repeated output keys at different sites retain their distinct guards. No traversal evaluates a guard or changes topology.
- [x] Test public views retain all potential public dependencies and all interpretation contracts, even for unselected branches, while redacting private implementation attribution. Future selector availability and restriction obligations remain discoverable after independent decode.
- [x] Test canonical entry ordering, ambiguous/missing/explicit-empty selection, exact profile/features and independent root/edge/work/output controls on `/3`. Do not advertise a profile through a wildcard tuple.

Qualification (17-10-2026): six owning-reader tests and three source → wire →
reader integration tests extend the existing cardinality and projection suites.
Literal nested if/match/AND/OR paths distinguish four occurrences of one exact
output key. Decoded views retain optional-field presence, list-index and variant
availability obligations, exact vocabulary restrictions and all potential public
contracts. Complete private reuse evidence is redacted by the owning reader test;
decoded source exports preserve materialized values without exposing private
occurrences. Positive root/edge/work/storage boundaries, shared selection/projection
budgets and observed allocation/cancellation recovery pass. Deliberate guard leaks,
budget resets, private-origin leaks and dropped availability obligations each fail
the corresponding oracle; all temporary faults were restored. No production API,
type checker, codec, frozen vector or profile-availability change was needed.
`cargo xtask test all` passes (1,309 tests, zero skipped, plus doctests), as do
formatting, full workspace Clippy and `cargo xtask ci pr`, including documentation.

## Task 8: Explicit consumer binding and availability/failure reports

**Files:** `neutral-reader/src/consumer/expression/{bindings,reports,context}.rs`; owning consumer tests and project-neutral runtime fixtures in `neutral-test-suite/tests/expression_pipeline/fixtures/`.

**Interfaces:** `ExpressionConsumerBoundary` borrows complete validated `/3` authority and an explicit consumer policy. The policy registers exact report contract, public producer/result associations and consumer diagnostic codes. `validate_bindings(request, cancel)` checks borrowed explicit input/output binding envelopes without evaluating; explicit `evaluate(request, cancel)` delegates to Task 4. Reports bind complete logical identity, selected public entry and adapter/target/input revisions. Reuse existing closed status/location/diagnostic concepts; never relabel consumer availability or scheduling-policy failures as compiler errors.

- [ ] Write `forged_output_association_is_rejected`, `restriction_debt_survives_until_value_supply`, `prerequisites_do_not_become_guarded_value_edges`, `binding_revisions_do_not_rewrite_project_identity`, and safe unavailable/failed report oracles.
- [ ] Verify explicit producer/result association is checked against registered consumer policy and public model contracts; it is not evidence of authorization or real backend success. No universal operation field name conveys authority.
- [ ] Use a file-backed project-neutral operation/result/prerequisite vocabulary. Expose its explicit prerequisite values separately from expression dependencies; consumer-specific cycle/capability decisions remain reported consumer decisions, not a core scheduler.
- [ ] Test duplicate/unknown/malformed/skipped bindings, exact type/revision/visibility, null versus absence, private-origin redaction, atomic failure and subsequent-request recovery.

Continuation evidence (17-10-2026): runtime-owned file-backed source/vocabulary
fixtures now exercise static decimal/short-circuit results, filtered template
expansion, guarded input/output dependencies, explicit prerequisite separation,
independent public views, result-root validation and retained vocabulary restrictions
through source → complete wire → independent reader → explicit consumer reports.
Binding-generation changes preserve complete identity; unavailable/failed bindings
have literal outcomes and later valid requests recover. These integration cases
supplement the component suites; they do not yet close every Task 7–9 corpus gate.

## Task 9: Explicit `/3` capture, source semantics and finite expansion

**Files:** `neutral-compiler/src/project_capture/expression.rs`; `frontend/expressions/{mod,parser,resolution,lowering}.rs`; `frontend/expansion/{mod,parameters,matrix,attribution}.rs`; owning compiler tests. Reuse existing lexer/header/capture/graph ownership and shared semantic library; make contextual successor parsing explicit rather than widening historical parser acceptance.

**Interfaces:** `CapturedExpressionProjectRequest`, `capture_expression_project(request)`, and `compile_expression_project(&CapturedExpressionProject, cancel) -> Result<Shared<ExpressionProjectIr>, ExpressionCompileFailure>`. Requests carry exact `/3` tuple and positive expression/expansion controls. The compiler resolves source/import/binder names, builds raw typed trees and uses Tasks 2–4 for typing/normalization; it cannot read external bindings from ambient state.

- [ ] Test every frozen operator/precedence, input/output intrinsic, match exhaustiveness, interpolation and existing source construction form. All branches resolve/type-check; selected static paths normalize, unresolved guarded bodies remain residual. Historical boundaries reject the same new syntax.
- [ ] Implement definition-scoped templates, static/deferred parameters, ordered named/default arguments, capture-free lexical substitution and recursion rejection. Templates are never runtime-callable values.
- [ ] Implement finite static matrices with preflight Cartesian cardinality, original axis-index names, aggregate visitation/work/storage limits, filters/exclusions and source attribution. Deferred values cannot affect count/names/topology. Test empty axes, repeated values, collisions and filtered one-over products.
- [ ] Run all registered source/type/evaluation/expansion literal cases through compilation and independent reading; preserve errors and no-I/O behavior. Complete all required feature paths before any explicit capture `/3` availability advertisement.

Known source-companion qualification gap: expression normalization currently
materializes private immutable reuse without emitting its reuse edge in
`frontend/expressions/lowering.rs`. Task 7 proves redaction of supplied, checked
reuse evidence; it does not prove that this source producer retains that evidence.
Cover producer lineage before closing this task's complete source/companion gate.

## Task 10: Independent probe and actual clean/cache equivalence

**Files:** `neutral-probe/src/expression.rs`; `neutral-compiler/src/frontend/expression_cache.rs`; owning probe/compiler tests; `neutral-test-suite/tests/expression_pipeline/{mod,source,consumer,identity,cache}.rs`.

**Interfaces:** `inspect_expression_encoded(bytes, wire_limits, semantic_limits, entry_request, cancel)` returns safe counts, complete typed identities, public contracts/ports/obligations/guarded dependencies and a redacted view. It never evaluates. Reuse the existing cache generation/publication pattern with `/3` captured/semantic keys; no second persistent cache is required.

- [ ] Verify probe dependency graph is compiler-free. Compare complete independently decoded public contracts and literal consumer reports with source-produced artifacts; no source reparse or registry acquisition.
- [ ] Compare actual clean, warm-cache and changed-unit compilation, including changed vocabulary/default/restriction, source evidence and semantic tuple. Failed/cancelled attempts preserve the prior successful generation; a later valid request succeeds without cross-request leakage.
- [ ] Confirm binding context and selected roots cannot alter complete captured/logical identity. Test unsupported tuple/feature and missing contract rejection before public summary publication.

Continuation evidence (17-10-2026): independent encoded entry probing now compares
unique and explicit-empty roots on source-produced artifacts, preserves complete
logical identity, and rejects unsupported features/truncation before a summary.
Actual expression-cache execution is still required; captured replay and these
reader/probe tests are not cache equivalence evidence.

## Task 11: Qualification, scope registration and honest gate closure

**Files:** `fuzz/fuzz_targets/{source,ir,probe,formatter}.rs`, appropriate shared expression harness modules, `config/quality-gates.toml`, benchmark phase registration, owning READMEs and `07-IMPLEMENTATION-CHECKLIST.md`.

- [ ] Add expression source/decoder/consumer paths to existing fuzz harnesses, with runtime-owned registered seeds. Extend reviewed mutation targets to expression typing, guarded demand, restrictions, decoding and identity; old configured targets alone are not expression evidence. Keep thresholds unchanged.
- [ ] Expand independent complete-project identity/result vectors as required by frozen activation rules without rewriting old reviewed bytes. Verify every registered case is tested or explicitly blocked, never counted as passed by file presence or oracle integrity alone.
- [ ] Run focused unit/integration/conformance/property/security tests, exact/one-over controls, exhaustive observed reservation/cancellation sweeps, ordering/concurrent isolation, old-profile regressions and actual clean/cache tests against final unchanged inputs.
- [ ] Run `cargo fmt --all`, full Clippy, `cargo xtask test all`, `cargo xtask ci pr`, nightly coverage, reviewed mutation, fuzz campaign and release/soak performance profiles. Inspect actual reports and failures; do not reuse evidence from earlier inputs, lower thresholds or remove scopes to pass.
- [ ] Update the checklist as each verified requirement is completed. Check this particular gate only when source-to-artifact retention, independent decoding/views, typed obligations, guarded dependencies, explicit prerequisite separation and binding/availability/failure reports all have passing production evidence.
- [ ] Keep authoring/editable-model/GUI and broader profile activation gates honest: headless gate completion alone cannot satisfy their independent requirements. Report any remaining activation limitation explicitly. No source-profile default switch, release, commit or push is part of this plan.

## Execution handoff

This plan covers the complete remaining headless deferred-input path, including its pure-semantics and finite-expansion dependencies. It preserves native execution and the current main checkout. Review the new internal shared-semantics boundary and the ordered tasks before execution; then implement all tasks without intermediate permission prompts. No new production code or completed checklist item is implied by this plan's existence.
