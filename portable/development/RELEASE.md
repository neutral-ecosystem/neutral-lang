# Neutral v0 release qualification

Status: mandatory Stage 10 gate

This document owns release artifacts, the release test matrix, final audits,
evidence, and approval. It cannot waive normative requirements from the
contract-freeze manifest.

## Entry criteria

- [ ] Stages 1–9 are complete on the candidate revision.
- [ ] Contract-freeze manifest identifies all governing versions.
- [ ] No blocking specification, implementation, security, test, performance,
      dependency, or standards-applicability issue remains.
- [ ] Release source tree is clean, protected, reviewed, and identified.
- [ ] Lockfile, toolchain, dependencies, environment image, test profiles,
      fixture manifest, benchmark corpus, and quality gates are immutable.
- [ ] Release credentials are unavailable to untrusted pull-request execution.

## Release artifacts

- [ ] v0 lexical/layout/grammar specification.
- [ ] v0 static semantic specification.
- [ ] Logical IR, invalid-state, equality, identity/fingerprint, source-map,
      provenance, and derivation specifications.
- [ ] Closed vocabulary schema and strict bundle-encoding specification.
- [ ] One versioned noncanonical external IR encoding specification.
- [ ] Compiler, capture/resolver, reader, diagnostic, limits, and probe APIs.
- [ ] Compiler and reader libraries.
- [ ] Compile/validate/format CLI.
- [ ] Standalone reader-only generic probe binary/library.
- [ ] Reference formatter.
- [ ] Captured fixture vocabulary bundle and exact digest.
- [ ] Normative conformance corpus, manifest, and expected results.
- [ ] Compatibility, supported-host, resource-profile, security, and dependency
      notes.
- [ ] SBOM, licenses/notices, dependency manifest, and build provenance.

## Release matrix

Run from an offline/prevalidated dependency environment:

- [ ] environment verification and manifest;
- [ ] repository/document/ID/traceability coherence;
- [ ] formatting, linting, docs, dependency, license, advisory, source, and
      package-boundary checks;
- [ ] smoke, unit, integration, system, and conformance suites;
- [ ] property/metamorphic and deterministic repeated/concurrent suites;
- [ ] security/adversarial and complete required fuzz campaign;
- [ ] controlled performance, resource, stress, and soak profiles;
- [ ] coverage and mutation gates;
- [ ] formatter idempotence and parse/format/parse equality;
- [ ] in-process reader/probe and external standalone probe comparison;
- [ ] every positive, negative, ambiguity, numeric, vocabulary, invalid-IR,
      resource, cancellation, and explicit-exclusion fixture;
- [ ] pinned development/release toolchain and MSRV jobs; and
- [ ] every finite supported host/target combination.

## Artifact verification

- [ ] Release packages contain only intended files and licenses.
- [ ] Package digests use the accepted exact-byte digest contract.
- [ ] Published logical/version identities match the freeze manifest.
- [ ] The standalone probe package dependency graph contains no compiler/private
      frontend dependency.
- [ ] Public examples compile and execute against packaged artifacts.
- [ ] A clean consumer environment can validate and inspect encoded artifacts
      using explicit vocabulary contracts without network lookup.
- [ ] Failed/cancelled commands never leave an apparently valid artifact.
- [ ] Build and test execution do not modify normative fixtures/goldens.

## Final scope and boundary audit

- [ ] Every master syntax-checklist item has normative prose, grammar, valid and
      invalid fixtures, stable diagnostics, lowering/provenance, limits,
      formatter behavior, and reader evidence.
- [ ] Every accepted source form has one frozen logical meaning.
- [ ] Every explicit exclusion has a passing rejection fixture.
- [ ] No API promises canonical IR bytes, automatic migration, public AST/IR
      rewriting, ambient lookup, runtime authority/effects, or application
      semantics.
- [ ] Source/vocabulary/IR inputs remain untrusted after digest match.
- [ ] `compile_captured`, decoder/reader, and probe effect boundaries pass review.
- [ ] No temporary milestone test/diagnostic entered release conformance.
- [ ] All accepted documents use consistent terms and versions.

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

- [ ] Technical owner approves implementation/specification consistency.
- [ ] Test/quality reviewer approves evidence and test completion.
- [ ] Security reviewer approves threat-boundary and unresolved-risk treatment.
- [ ] Release owner approves packaging, versions, compatibility, and publication.
- [ ] Independent review is used for critical decoder/identity/security paths
      where staffing permits; a staffing exception and compensating review are
      recorded otherwise.
- [ ] Standards alignment is described honestly; no unsupported ISO conformity,
      accreditation, or certification claim is published.

## Exit condition

Neutral v0 may be released only when every required item above passes and the
approval record names exact language, IR, source-map, provenance, derivation,
vocabulary schema/encoding, digest profile, external encoding, compiler, reader,
probe, fixture, and limits-profile versions independently.
