<!-- SPDX-License-Identifier: Apache-2.0 -->

# v0.1.0 entry-baseline validation

Status: validated on 2026-09-20

This record establishes the released `v0.1.0` implementation as the immutable
compatibility baseline for v1 work. It does not extend v0 behavior or claim
that any Stage 1 feature is implemented.

## Executed gates

| Gate | Stable command or test | Result |
| --- | --- | --- |
| Package and contract versions | `cargo xtask version show` and `cargo xtask version check` | Pass; package release and frozen contract versions report `0.1.0` where applicable. |
| Environment | `cargo xtask bootstrap` | Pass; the generated environment manifest is under `test-results/bootstrap/`. |
| Inherited conformance | `cargo xtask test conformance` | Pass; all selected compiler and vocabulary conformance tests passed. |
| Explicit profile boundary | `parser_matches_the_minimal_frozen_oracle`, `parser_matches_the_unsupported_version_frozen_oracle`, and `parser_rejects_the_unavailable_v1_profile` | Pass; `neu "0.1"` remains accepted while unavailable profiles, including `neu "1.0"`, fail with the unsupported-version diagnostic and no IR. |
| Bounded performance | `cargo xtask test performance --profile pr` | Pass across compile, reader, encoding, decoding, probe, growth, and concurrent-isolation phases. |
| Push CI composition | `cargo xtask ci pr` | Pass; generated workflow evidence is under `test-results/workflows/ci/pr/`. |

## Frozen comparison anchors

- Public API and language behavior: the released `v0.1.0` source tag, generated
  API documentation, and public contract tests.
- Dependencies: the locked `Cargo.lock` selected by `cargo xtask version check`.
- Diagnostics and conformance: the immutable
  [`v0.1.0` conformance bundle](../../../conformance/releases/v0.1.0/README.md),
  including its freeze, manifest, fixtures, and oracles.
- Performance and resource behavior: the reviewed
  [`v0.1.0` dynamic evidence](../../../quality/evidence/v0.1.0/stage9-dynamic-evidence.md)
  plus the current bounded PR performance run.

## Baseline decision

All five baseline checklist items are satisfied. Stage 1 may begin, but remains
independently gated by its contract, fixture, implementation, integration, and
validation requirements.
