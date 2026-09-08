<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 development evidence

This directory retains implementation-specific review records that support the
numbered lifecycle gates without becoming normative language specifications.

- [Contract question ledger](contract-question-ledger.md) records how every
  blocking freeze question was closed.
- [Stage 7 decoder allocation review](stage7-decoder-allocation-review.md)
  records the bounds-before-allocation audit for hostile artifacts.
- [Stage 10 candidate-preparation record](stage10-candidate-preparation.md)
  records the reproducible source, dependency, toolchain, fixture, and contract
  identities collected before a release-authority decision creates a candidate.
- [Stage 10 workflow-overhaul record](stage10-workflow-overhaul.md) records the
  stable command boundary, policy ownership, platform/CI adapters, fail-closed
  release behavior, and executed Step 2 validation.
- [Stage 10 version and portable lifecycle record](stage10-version-portable-lifecycle.md)
  records centralized package-version, lock/source, generated-output, snapshot,
  archive-boundary, and rollover evidence for Step 3.
- [Stage 10 repository and quality overhaul record](stage10-repository-quality-overhaul.md)
  records directory ownership, durable test levels, generated-file hygiene,
  coverage/fuzz policy, warning enforcement, and executed Step 4 validation.
- [Stage 10 environment reproduction record](stage10-environment-reproduction.md)
  records supported-host bootstrap, complete tool identities, offline clean-tree
  CI, stable/nightly isolation, ignore policy, and release-input independence.
- [Stage 10 candidate qualification record](stage10-candidate-qualification.md)
  records the exact `main` revision, release CI, coverage, campaign
  applicability, benchmarks, candidate reviews, and tool identities for Step 6.
- [Stage 10 consumer-deliverable record](stage10-consumer-deliverables.md)
  records release builds, CLI fail-closed behavior, reader/probe equivalence,
  clean-consumer execution, dependency isolation, and Rustdoc inspection.
- [Stage 10 distribution-assembly record](stage10-distribution-assembly.md)
  records selected artifacts, checksums, source archive, dependency/SBOM and
  provenance outputs, transient-output exclusion, and packaged-form testing.
- [Stage 10 release-publication record](stage10-release-publication.md) records
  final approval ownership, publication-workflow controls, completed local
  validation, and the explicit remote operations still required for Step 9.

New evidence belongs here only when no numbered lifecycle document owns it.
