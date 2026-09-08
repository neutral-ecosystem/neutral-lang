# Neutral v0 release qualification

Status: mandatory Stage 10 gate

This document owns release artifacts, the release test matrix, final audits,
evidence, and approval. It cannot waive normative requirements from the
contract-freeze manifest.

## Entry criteria

- [x] Stages 1–9 are complete on the candidate revision.
- [x] Contract-freeze manifest identifies all governing versions.
- [x] No blocking specification, implementation, security, test, performance,
      dependency, or standards-applicability issue remains.
- [x] Release source tree is clean, protected, reviewed, and identified.
- [x] Lockfile, toolchain, dependencies, environment image, test profiles,
      fixture manifest, benchmark corpus, and quality gates are immutable.
- [x] Release credentials are unavailable to untrusted pull-request execution.

## Release artifacts

- [x] v0 lexical/layout/grammar specification.
- [x] v0 static semantic specification.
- [x] Logical IR, invalid-state, equality, identity/fingerprint, source-map,
      provenance, and derivation specifications.
- [x] Closed vocabulary schema and strict bundle-encoding specification.
- [x] One versioned noncanonical external IR encoding specification.
- [x] Compiler, capture/resolver, reader, diagnostic, limits, and probe APIs.
- [x] Compiler and reader libraries.
- [x] Compile/validate/format CLI.
- [x] Standalone reader-only generic probe binary/library.
- [x] Reference formatter.
- [x] Captured fixture vocabulary bundle and exact digest.
- [x] Normative conformance corpus, manifest, and expected results.
- [x] Compatibility, supported-host, resource-profile, security, and dependency
      notes.
- [x] SBOM, licenses/notices, dependency manifest, and build provenance.

## Release matrix

Run from an offline/prevalidated dependency environment:

- [x] environment verification and manifest;
- [x] repository/document/ID/traceability coherence;
- [x] formatting, linting, docs, dependency, license, advisory, source, and
      package-boundary checks;
- [x] smoke, unit, integration, system, and conformance suites;
- [x] property/metamorphic and deterministic repeated/concurrent suites;
- [x] security/adversarial and complete required fuzz campaign;
- [x] controlled performance, resource, stress, and soak profiles;
- [x] coverage and mutation gates;
- [x] formatter idempotence and parse/format/parse equality;
- [x] in-process reader/probe and external standalone probe comparison;
- [x] every positive, negative, ambiguity, numeric, vocabulary, invalid-IR,
      resource, cancellation, and explicit-exclusion fixture;
- [x] resolved latest-stable development/release toolchain and exact MSRV
      compatibility checks; and
- [x] every finite supported host/target combination.

## Artifact verification

- [x] Release packages contain only intended files and licenses.
- [x] Package digests use the accepted exact-byte digest contract.
- [x] Packaged logical/version identities match the freeze manifest.
- [x] The standalone probe package dependency graph contains no compiler/private
      frontend dependency.
- [x] Public examples compile and execute against packaged artifacts.
- [x] A clean consumer environment can validate and inspect encoded artifacts
      using explicit vocabulary contracts without network lookup.
- [x] Failed/cancelled commands never leave an apparently valid artifact.
- [x] Build and test execution do not modify normative fixtures/goldens.

## Final scope and boundary audit

- [x] Every master syntax-checklist item has normative prose, grammar, valid and
      invalid fixtures, stable diagnostics, lowering/provenance, limits,
      formatter behavior, and reader evidence.
- [x] Every accepted source form has one frozen logical meaning.
- [x] Every explicit exclusion has a passing rejection fixture.
- [x] No API promises canonical IR bytes, automatic migration, public AST/IR
      rewriting, ambient lookup, runtime authority/effects, or application
      semantics.
- [x] Source/vocabulary/IR inputs remain untrusted after digest match.
- [x] `compile_captured`, decoder/reader, and probe effect boundaries pass review.
- [x] No temporary milestone test/diagnostic entered release conformance.
- [x] All accepted documents use consistent terms and versions.

## Quality and test completion report

The retained report includes:

- candidate and artifact identities;
- environment/tool/dependency/fixture/limits/profile identities;
- planned, discovered, executed, passed, failed, skipped, quarantined, flaky,
  and missing test counts by level;
- requirements/diagnostics/exclusions coverage;
- code coverage and mutation results with exclusions;
- fuzz duration/corpus/findings;
- benchmark/resource/stress/soak results and baseline comparison;
- static review, security review, dependency review, and quality evaluation;
- deviations and unresolved defects;
- residual product/project risks and owners;
- standards applicability/edition review; and
- release recommendation and approvers.

Missing, corrupt, expired, contradictory, retried-into-pass, or indeterminate
required evidence fails release unless the governing contract explicitly permits
an indeterminate result and the release authority accepts the residual risk.

## Approval

- [x] Technical owner approves implementation/specification consistency.
- [x] Test/quality reviewer approves evidence and test completion.
- [x] Security reviewer approves threat-boundary and unresolved-risk treatment.
- [x] Release owner approves packaging, versions, compatibility, and publication.
- [x] Independent review is used for critical decoder/identity/security paths
      where staffing permits; a staffing exception and compensating review are
      recorded otherwise.
- [x] Standards alignment is described honestly; no unsupported ISO conformity,
      accreditation, or certification claim is published.

### Approval record

Approval date: 2026-09-08. Approver for the technical, test/quality, security,
release, and standards roles: Younes Rabeh, sole maintainer and release owner.

Independent role separation was not available. The approved staffing exception
is compensated by automated frozen-contract traceability, warning-free static
checks, full conformance and adversarial suites, unchanged coverage thresholds,
zero surviving viable mutants, five independent 900-second fuzz campaigns,
Valgrind allocation/leak review, reproducible packaging, checksum verification,
and a clean standalone consumer exercise. The limitation remains disclosed and
is not represented as independent review, accreditation, certification, or full
standards conformity.

The approved independent release identities are: language behavior `0.1.0`,
logical IR schema `0.1.0`, source map `0.1.0`, provenance `0.1.0`, derivation
`0.1.0`, vocabulary schema `0.1.0`, vocabulary bundle encoding `0.1.0`, digest
transcript profile `0.1.0`, external encoding `NIR-CBOR/0.1`, compiler API
`0.1.0`, reader API `0.1.0`, probe API `0.1.0`, fixture corpus manifest
`f05752b28c7e1bc1d58d5a2200c85c58837e93932ec07c9db457e01b564075fe`,
and structural limits profile `0.1.0`.

## Exit condition

Neutral v0 may be released only when every required item above passes and the
approval record names exact language, IR, source-map, provenance, derivation,
vocabulary schema/encoding, digest profile, external encoding, compiler, reader,
probe, fixture, and limits-profile versions independently.
