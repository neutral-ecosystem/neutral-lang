# Neutral v0 testing and quality plan

Status: required implementation and release policy

This document owns TDD, test levels, test locations, suite activation, quality
measurement, evidence, and standards alignment. Implementation stages link here
instead of duplicating test policy.

## Standards alignment and claim boundary

Neutral v0 aligns its project-specific process with:

- [ISO/IEC/IEEE 29119-1:2022](https://www.iso.org/standard/81291.html) for test
  concepts and terminology;
- [ISO/IEC/IEEE 29119-2:2021](https://www.iso.org/standard/79428.html) for test
  processes;
- [ISO/IEC/IEEE 29119-3:2021](https://www.iso.org/standard/79429.html) for test
  documentation;
- [ISO/IEC/IEEE 29119-4:2021](https://www.iso.org/standard/79430.html) for test
  design techniques;
- [ISO/IEC 25010:2023](https://www.iso.org/standard/78176.html) for product
  quality characteristics;
- [ISO/IEC 25023:2016](https://www.iso.org/standard/35747.html) for selected
  product-quality measures;
- [ISO/IEC 25040:2024](https://www.iso.org/standard/83467.html) for quality
  evaluation;
- [ISO/IEC 20246:2017](https://www.iso.org/standard/67407.html) for static work
  product reviews; and
- [ISO/IEC 5055:2021](https://www.iso.org/standard/80623.html) for selected
  automated source-code measures.

This is an alignment target, not an ISO conformity or certification claim. A
conformity claim requires the complete licensed standards, an applicability and
tailoring record, objective evidence, qualified review, and any required
independent assessment.

Create `quality/standards-register.md` with edition, ISO status, applicability,
tailoring, evidence, owner, and next review date. Recheck it before every release
candidate.

## Test ownership and source of truth

The future implementation repository uses one owner for each class of test
material:

```text
spec/v0/fixtures/
└── normative source and vocabulary fixtures copied/pinned from this v0 spec

conformance/
├── manifest.toml              # fixture → requirement → oracle → active stage
├── expected-ir/
├── expected-diagnostics/
├── expected-source-map/
├── expected-provenance/
├── expected-derivation/
└── encoded-ir/

crates/<production-crate>/src/
└── colocated private unit tests only

crates/neutral-test-support/
└── reusable builders, fixture loading, assertions, and test-only comparators

crates/neutral-test-suite/
├── tests/smoke/
├── tests/integration/
├── tests/system/
├── tests/conformance/
├── tests/determinism/
└── tests/security/

crates/neutral-bench/
├── corpora/
└── benches/

fuzz/
├── corpus/
└── fuzz_targets/
```

Rules:

- `spec/v0/fixtures` is the normative input source of truth.
- `conformance/manifest.toml` is the sole fixture discovery and oracle index.
- `neutral-test-support` contains no top-level integration, system, conformance,
  or benchmark tests.
- `neutral-test-suite` is non-published and owns cross-crate executable tests.
- `neutral-bench` is non-published and owns benchmark harnesses and immutable
  benchmark corpus identities.
- Fuzz targets live only under `fuzz/` and promote confirmed failures into
  deterministic conformance/security regression fixtures.
- Generated results live only under ignored `test-results/<run-id>/`.
- There is no duplicate root `tests/` tree outside its owning Cargo package.

## Independent probe proof

`neutral-probe` is both a library and standalone binary package.

- Its normal and development dependency graph may include only public core and
  reader contracts plus narrowly reviewed CLI/output dependencies.
- It may not depend on `neutral-compiler`, compiler test support, private AST,
  semantic types, source lexer/parser, or filesystem resolver implementation.
- The library accepts a public validated reader view.
- The binary accepts an externally encoded artifact plus explicit host-supplied
  vocabulary contracts and reader limits.
- Compiler-to-in-process-reader-to-probe behavior is tested only from
  `neutral-test-suite`, which may compose public packages.
- `neutral-cli` does not embed the probe to claim independence. Compiler commands
  and the standalone probe remain separate release artifacts.
- CI validates `cargo tree --package neutral-probe --edges all` against an
  allowlist and builds the package independently.

## TDD workflow

Every observable production behavior follows red → green → refactor:

1. Select one frozen requirement/decision and identify risk and test level.
2. Write the smallest test against the lowest stable owning interface.
3. Run it and prove it fails for the intended missing behavior.
4. Record the red command, test ID, and failure summary in the change record.
5. Implement only enough behavior to satisfy the test.
6. Run the narrow test, then affected unit/integration/conformance/smoke suites.
7. Refactor while all affected suites remain green.
8. Complete negative, boundary, misleading-lookalike, diagnostic, source-map,
   provenance, reader, and resource evidence for the vertical slice.
9. Run the local PR-equivalent gate before review.

Exceptions are limited to documentation-only changes, build metadata, or proven
behavior-preserving mechanical refactors. The exception still runs affected
gates and is recorded in the change.

### Enforcement

- [ ] Every behavior change links `NL-*`/`SYN-*` IDs and red-phase evidence.
- [ ] Every defect fix begins with a regression test that fails on the defective
      revision.
- [ ] New parser acceptance cannot merge before semantic, IR, reader, and
      conformance obligations are implemented in the same slice.
- [ ] Tests assert behavior/invariants, not incidental Rust layout or map order.
- [ ] Goldens update only through explicit `cargo xtask golden update`, and every
      semantic difference is reviewed.
- [ ] Flaky tests are defects; retries collect evidence but never convert a
      required failure into a passing gate.
- [ ] Ignoring, weakening, deleting, or quarantining a test requires owner,
      rationale, risk approval, and expiry/reinstatement condition.
- [ ] Randomized failures retain seeds and are minimized into stable fixtures.

## Stage-aware suite activation

The implementation records the active development stage in
`config/development-stage.toml`. The conformance manifest gives each case an
`active_from_stage` value.

Rules:

- Only behavior completed by the current stage is active.
- Future-stage cases are listed as planned, not compiled into intentionally
  failing test skeletons.
- Every active suite must discover at least its configured minimum test count.
- A required active case that is missing, skipped, ignored, or lacks an oracle
  fails suite discovery.
- A future valid v0 form is never committed as a normative negative fixture just
  because its implementation stage has not begun.
- Temporary milestone checks live under `tests/development/` and cannot use
  stable language diagnostic codes or enter release evidence.
- Advancing the stage is a reviewed change that activates the new suite cases in
  the same commit as their implementation and evidence.

### Stage 1 active tests

Stage 1 has no compiler behavior. Its passing suites are intentionally limited
to:

- automation command parsing and safe cleanup unit tests;
- environment verification and manifest tests;
- workspace metadata/dependency-boundary checks;
- package shell build and documentation tests;
- standalone probe dependency-allowlist check; and
- CLI shell `--help`, version, and no-argument behavior if the shell exists.

Compiler smoke, conformance, semantic integration, full system, fuzz, and
performance suites are planned but inactive. Stage 1 cannot require them to
pass, and no placeholder is allowed to fail on the main branch.

### Activation milestones

| Stage | Newly active evidence |
| --- | --- |
| 1 | Environment, automation, workspace, dependency, shell smoke |
| 2 | Minimal scalar unit/integration/system/conformance/probe smoke |
| 3 | Source text and remaining scalar vertical-slice cases |
| 4 | Record/default/list vertical-slice cases |
| 5 | Reuse/reference/alpha-equivalence cases |
| 6 | Vocabulary bundle/source/reader cases |
| 7 | External IR encoding/hostile decoder cases |
| 8 | Full formatter/CLI/standalone probe and traceability cases |
| 9 | Extended property, fuzz, security, performance, soak, mutation |
| 10 | Complete release matrix and retained quality evaluation |

## Test levels

### Smoke

Small release-facing happy and fail-closed paths. It runs on every local PR gate.
Each stage adds at most the smallest critical end-to-end path for newly active
behavior.

### Unit

Colocated pure-module invariants: spans, limits, diagnostics, exact numbers,
tokens, layout, parser productions, symbol/type logic, graph algorithms,
lowering, fingerprints, alpha-equivalence, schema validators, and decoder
checks. Unit tests perform no network, process, ambient filesystem, wall-clock,
locale, or sleep-based synchronization.

### Integration

Public boundary compositions inside one process:

- resolver → capture;
- captured input → compiler → logical IR → reader;
- logical IR → encoding → decoder/validator → reader;
- vocabulary contract across capture/compiler/reader;
- reader → probe library; and
- cancellation/limits at component handoffs.

Integration uses in-memory or isolated temporary adapters and no public network.

### System

Black-box built binaries in unique temporary roots. Cover files/stdin/stdout,
Unicode/spaced/long paths, permissions, missing/unwritable/existing output,
broken pipes, cancellation, interrupted atomic writes, safe diagnostics, exit
classes, and no partial authoritative output. Run on the finite declared host
matrix—not an unbounded claim about every filesystem.

### Conformance

Manifest-driven normative fixtures. Golden comparison includes logical graph
alpha-equivalence plus diagnostics, source maps, provenance, derivation, and
resource outcomes. It never freezes map order, `ElementId` spelling, pretty
printing, or noncanonical IR bytes.

### Property and metamorphic

Cover exact-number normalization, line-ending equivalence, comment nonsemantics,
formatter idempotence, parse/format/parse equality, declaration-order
independence, alpha-equivalence, diagnostic ordering, encode/decode equality,
and repeated/concurrent determinism. CI records seeds and minimizes failures.

### Security and fuzz

Deterministic hostile cases run on every PR. Coverage-guided fuzz smoke runs on
PRs; extended campaigns run nightly/release for source decoding, lexer/layout,
parser/recovery, vocabulary JSON, external IR decoder, formatter, and probe
traversal. No crash, hang, stack exhaustion, uncontrolled allocation, invalid
typed IR, or partial success is acceptable.

### Performance, stress, and soak

Use immutable representative/boundary/adversarial corpora. Measure phase and
end-to-end latency, throughput, peak memory, allocations where supported,
artifact size, decoder behavior, traversal, growth by structural dimension,
concurrent isolation, and cancellation responsiveness.

PR performance is informational except for gross complexity/time-budget
failures. Release regression gates run on a controlled dedicated runner with
pinned toolchain, power/CPU policy, warm-up, repeated samples, dispersion, and
reviewed absolute/relative thresholds.

### Mutation and static review

Mutation testing targets exact-number, layout, compatibility, graphs,
alpha-equivalence, limits, diagnostic ordering, and decoder validation. Surviving
mutants are classified as missing test, equivalent, tool limitation, or accepted
risk. Static review covers normative specs, grammar, public APIs, trust
boundaries, tests/oracles, identity transcripts, vocabulary schema, and goldens.

## Test design techniques

Use and record the applicable technique:

- equivalence partitioning;
- boundary value analysis;
- decision tables;
- state-transition testing;
- grammar/syntax-based testing;
- pairwise/combinatorial testing;
- property/metamorphic testing;
- fault injection; and
- static work-product review.

## Coverage and strength gates

- 100% accepted requirement-to-executable-evidence coverage;
- 100% stable diagnostic catalogue coverage;
- 100% explicit-exclusion rejection coverage;
- changed production behavior exercised unless a reviewed exception exists;
- line/function/region thresholds established after Stage 2 baseline and stored
  in `config/quality-gates.toml`;
- critical-module mutation target calibrated before Stage 9 and stored in the
  same file; and
- thresholds ratchet upward; reduction requires a reviewed quality decision.

Coverage percentage alone never proves correctness.

## Test metadata and evidence

Every normative/risk-significant test records:

- stable test ID and title;
- governing requirement/decision IDs;
- level and design technique;
- product/project risk and priority;
- preconditions, fixture/config/limit identities;
- input/action and independently reviewable oracle;
- expected typed output, diagnostic/span/resource/effect result;
- timeout/budget and cleanup;
- owner and review status; and
- first/current implementation result.

Every CI run records commit, clean-tree state, lockfile digest, toolchain, target,
environment image, profile, seed, fixture manifest digest, timing, and results.
Produce machine-readable dynamic-test results plus coverage, mutation, fuzz,
benchmark, dependency, static-analysis, and documentation reports. Missing or
internally inconsistent required evidence fails the gate.

## Product quality evaluation

Tailor ISO/IEC 25010 characteristics to Neutral:

- functional suitability: exact v0 conformance;
- performance efficiency: named resource profiles;
- compatibility: supported hosts and independent contract versions;
- interaction capability: safe CLI and source-linked diagnostics;
- reliability: determinism, cancellation, and no partial success;
- security: untrusted-input and effect boundaries;
- maintainability: boundaries, reviews, complexity, tests, documentation;
- flexibility: explicit host/resolver/encoding substitution without semantic
  drift; and
- safety: fail-closed behavior and no authority/external effects.

Each selected measure needs purpose, unit, method, tool/version, sample,
threshold, owner, and failure response. Release concludes each selected quality
characteristic as pass, fail, or indeterminate. Missing evidence is never a
pass.
