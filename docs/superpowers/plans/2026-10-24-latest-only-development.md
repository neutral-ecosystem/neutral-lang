<!-- SPDX-License-Identifier: Apache-2.0 -->

# Latest-only development implementation plan

> Use `superpowers:executing-plans` for the coordinated cutover. Independent host adapters may be delegated only to Luna, as requested by the user. Do not commit or push.

**Goal:** One current source → complete artifact → independent reader/probe pipeline, without maintaining pre-stable compatibility implementations.

**Architecture:** Migrate callers onto existing expression-project authority first, extract shared primitives out of historical owners, then remove obsolete public paths and implementation. Reuse one syntax cache; no compatibility feature flags or artifact conversion layer.

**Tech stack:** Existing Rust workspace, fallible ownership, restricted CBOR, Nextest and xtask.

**Spec:** [Approved design](../specs/2026-10-24-latest-only-development-design.md).

**Completed:** 25-10-2026. The current pipeline is the sole development path.
`cargo xtask test all` and `cargo xtask ci pr` pass with 877 discovered tests;
the full suite also checks category minimums and doctests. All five fuzz targets
compile with the locked manifest, the benchmark PR profile passes, the registered
literal-oracle verifier passes, and the CLI-produced artifact independently probes
as valid JSON. Final Luna review found no Important/Critical findings.
Coverage, mutation, fuzz campaigns and controlled performance/soak measurements
were not rerun; prior receipts do not qualify this changed source tree.

## Global constraints

- Sole current source header is `neu "1.0"`; complete capture/IR/wire/view/identity use the existing `/3` selectors. Current evaluator/expression/expansion `/1` and vocabulary schema `2.0` remain valid independent contracts.
- Reject old/unknown source and artifact selectors without fallback. Do not alter frozen historical bytes or current identity/wire vectors.
- Preserve independent budgets, cancellation, atomic output/cache publication and compiler-free decoding/probing. No new acquisition, scheduler, evaluator invocation by the probe, dependencies or Git history changes.
- Keep all relevant safety/semantic regressions through current entry points. Do not lower quality thresholds or reuse measurements from older inputs.
- Work in the existing main checkout. Delegated workers, if any, must be `gpt-6-luna` and have disjoint file ownership.

## Review focus

1. Old magic/header cannot reach a successful current output, including CLI aliases and fuzz/benchmark adapters.
2. Host output replacement remains atomic after compilation, formatting, encoding, cancellation and input failures.
3. Current formatting preserves comments, expressions, templates/matrices, all-branch validity and complete logical meaning.
4. Shared contract/codec/type helpers retain validation when their historical public owners are removed; no private-source leaks or fabricated companions.
5. Cache generations cannot reuse old grammar or stale vocabulary/source evidence, and every active test/quality command targets the current implementation.

## Task 1: Current compiler formatting and host boundaries

**Files:** compiler expression formatter and exports; `neutral-cli/src/{host,command,constants}.rs` and owning tests; `neutral-probe/src/{main,expression}.rs` and owning tests.

**Interfaces:** Existing `CapturedExpressionProject`, `compile_expression_project`, `ValidatedExpressionProject::from_ir`, `encode_expression_project` and `decode_expression_project` stay the integration authority during migration. Add `format_expression_source(captured, module, cancel) -> Result<Vec<u8>, ExpressionCompileFailure>` to format one explicit module after whole-project validation. No new serialized selector.

- [x] Add CLI current arithmetic/template/deferred fixtures: compile emits `NEUP3`, validate publishes nothing, format is idempotent and meaning-preserving; old headers reject without overwriting existing output. Run owning tests and observe legacy dispatch fail these assertions.
- [x] Implement current capture/compile/format adapters, retaining host I/O and lock controls. Add current-only standalone probe dispatch, safe public summaries and explicit roots; old/unknown magic rejects atomically.
- [x] Migrate host/probe tests that assert old successful formats; retain their I/O, cancellation, redaction and bounds checks on current artifacts.
- [x] Run `cargo test -p neutral-cli -p neutral-probe` and current formatter/source pipeline tests; expected all pass.

## Task 2: Active tool and fixture migration

**Files:** `fuzz/fuzz_targets/`, owned seeds; `neutral-bench`; `neutral-test-support`; active test-suite fixtures; `config/{quality-gates,conformance,ir-encoding}.toml`; xtask consumers and tests.

**Interfaces:** Task 1's current capture/compile/format/reader/probe APIs replace every active legacy caller. Metric/report protocols remain stable unless explicitly migrated with their consumers; obsolete target paths are removed, not silently skipped.

- [x] Add current round-trip, cache, malformed-input, formatting and host-result assertions to every migrated harness. Demonstrate that an old source or wire producer fails the current assertions.
- [x] Migrate useful legacy property/security/limits/recovery tests to current owning modules; retire only old-success acceptance requirements. Preserve historical release records and literal rejection fixtures.
- [x] Compile all five fuzz targets without running campaigns; run benchmark smoke/PR profile, test-support and xtask checks. Expected: buildable current harnesses and no active dangling path.

## Task 3: Shared extraction and obsolete implementation removal

**Files:** compiler frontend/capture/cache; reader project/composition/consumer modules; encoding lib/project/composition helpers; IR models/identity modules; core profile catalogue; owning tests.

**Interfaces:** One complete captured project, compilation, reader authority, transport and cache remain public. Promote current entry points to ordinary project naming where no collision remains; remove old aliases. Shared scalar/type/value/contract/location/framing helpers remain in their existing responsible layer.

- [x] Add old header/magic/selector rejection cases and current public catalogue assertions before removing dispatch. Observe old successful entry points violate the latest-only expectation.
- [x] Extract current-reachable helpers, migrate their callers and retained unit tests, and remove old-only public APIs, authority types, complete codecs, caches and old-success test modules. Do not leave dormant compatibility implementations.
- [x] Remove historical cache branches; preserve current exact-byte/profile/control keys and failure recovery. Verify current literal identity/wire vectors remain unchanged.
- [x] Run workspace all-target checks and owning tests after each extraction batch. Expected: no production old-success path, no dead-code suppression and no lost validation.

## Task 4: Policy, documentation and final qualification

**Files:** active compatibility-policy amendment, implementation checklist, affected plans/READMEs, CLI examples, repository/configuration checks.

**Interfaces:** Tasks 1–3 establish the current-only code; documentation must describe actual availability, not stable readiness. Historical approvals remain historical and source-bound receipts become stale normally.

- [x] Amend current compatibility policy to supersede development-era historical acceptance clauses; keep original frozen contracts/vectors byte-identical.
- [x] Update commands/examples/catalogues and checklist with actual progress. Leave long campaigns, authoring and stable activation gates open without new evidence.
- [x] Run `cargo xtask test all`, `cargo xtask ci pr`, compiler-free probe dependency checks, current literal oracle verification, fuzz build and benchmark smoke. Expected: every active fast gate passes, unchanged current vectors, no dangling legacy consumer.
- [x] Obtain one final Luna review; fix Important/Critical findings with failing regression → fix → full green suite. Report any remaining long qualification explicitly. No commit/push/release.
