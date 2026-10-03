<!-- SPDX-License-Identifier: Apache-2.0 -->

# 06 — Validation ledger

Status: active release-train ledger

## Rules

The portable implementation checklist is the completion authority. Developers
select and review work themselves; workflow logs under ignored `test-results/`
retain commit-bound toolchain and gate outcomes.
This ledger preserves milestone decisions and links to exceptional reviews,
not a second per-edit status report. A stage is released only after its
promotion and release-quality evidence exist.

| Stage | Transition | Current status | Validation required to advance |
| --- | --- | --- | --- |
| 1 | `v0.1.0 -> v0.2.0` | released | [profile matrix, inherited v0 corpus, v1 exclusion/audit evidence](evidence/stage1.md); release promotion complete |
| 2 | `v0.2.0 -> v0.3.0` | released | [Stage 2.1 contract gate](evidence/stage2-1-contract-gate.md); [Stage 2.2 core capture](evidence/stage2-2-core-capture.md); [Stage 2.3 public integration](evidence/stage2-3-public-integration.md); [Stage 2.4 validation](evidence/stage2-4-validation.md); `v0.3.0` release and quality approval complete |
| 3 | `v0.3.0 -> v0.4.0` | released | [Stage 3.1 contract](evidence/stage3-1-contract-gate.md), [Stage 3.2 core graph](evidence/stage3-2-core-graph.md), [Stage 3.3 public integration](evidence/stage3-3-public-integration.md), and [Stage 3.4 validation](evidence/stage3-4-validation.md) passed; [v0.4.0 quality approval](../../quality/evidence/v0.4.0/record.toml) and signed tag retained |
| 4 | `v0.4.0 -> v0.5.0` | released | [public semantics contract](../specs/contracts/PUBLIC-SEMANTICS.md), [pinned fixtures](../conformance/manifest.toml), [Stage 4 integration and validation](evidence/stage4-validation.md), and [v0.5.0 quality approval](../../quality/evidence/v0.5.0/record.toml); signed tag and GitHub release assets verified |
| 5 | `v0.5.0 -> v0.6.0` | released | [Stage 5 contract, core, integration, and validation evidence](evidence/stage5-contract-core.md); [v0.6.0 quality approval](../../quality/evidence/v0.6.0/record.toml) and release tag retained; canonical reader catalogue, executable fixtures, inert-location review, hostile limits, and vocabulary fuzz evidence reviewed; presentation-only metadata projection is assigned to Stage 8 |
| 6 | `v0.6.0 -> v0.7.0` | not started | project IR/reader/view public probe |
| 7 | `v0.7.0 -> v0.8.0` | not started | reviewed transcript and identity vectors |
| 8 | `v0.8.0 -> v0.9.0` | not started | dynamic catalogue and generic Editor no-op probe |
| 9 | `v0.9.0 -> v1.0.0` | not started | full manifest, all probes, hardening, and release review |

## Entry baseline

| Baseline | Status | Evidence |
| --- | --- | --- |
| Released `v0.1.0` implementation and environment | validated | [baseline validation](evidence/v0-baseline.md) |

## Release evidence

The generated PR and release workflows record toolchain, source commit,
inherited and active suites, quality steps, and their results. Release approval
is retained in `quality/evidence/<version>/record.toml`. A milestone row above
only needs an authored note when a limitation, deferral, or normative decision
cannot be expressed by those machine-checked sources. The final `v1.0.0`
promotion also requires [the v1 checklist](../specs/contracts/v1-checklist.md).
