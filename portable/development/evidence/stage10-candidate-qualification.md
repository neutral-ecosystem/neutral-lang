<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 candidate qualification

Review date: 2026-09-08. Result: pass for Stage 10 Step 6.

## Candidate identity

Qualification ran from clean checked-out branch `main` at full revision
`47633a635deafa6d56260442a02e5aab14ff9441` (`[FIX] main candidate`). The
release candidate is always this exact `main` `HEAD`; the eventual `v0.1.0`
publication tag did not select or replace the qualified source.

| Input | SHA-256 |
| --- | --- |
| Root release lock `Cargo.lock` | `300bc9e55dd409d35f4b747da774c73200035aa37689102f0ae4c332b7058c49` |
| Isolated fuzz lock `fuzz/Cargo.lock` | `521e791b97feb62b4d82e8e44dc6788c670403487339806c21a5698575c96a6a` |
| Quality-gate configuration | `74fa7a48137a830a8122592592cf9cbea6b822fe6cd160794ebf7e108caa3ef2` |

## Release composition and coverage

`cargo xtask ci release` passed. Its ignored machine summary is retained at
`test-results/ci/release/run-1703-0/task-summary.json`; the corresponding
release-quality summary is at
`test-results/analysis/quality-report/release/run-1703-0/task-summary.json`.
The composition passed formatting, warning-free linting, locked compilation,
all ordinary tests, binary smoke and fuzz regressions, dependency and package
boundaries, test layout, traceability, version/portable/generated-output and
workflow checks, documentation, optimized builds, and released-binary
validation.

`RUSTUP_TOOLCHAIN=nightly cargo xtask coverage` passed without changing its
scope or thresholds. The machine-readable report is retained at
`test-results/analysis/coverage/coverage.json` and the HTML entry point at
`test-results/analysis/coverage/html/index.html`.

| Measure | Required | Observed | Result |
| --- | ---: | ---: | --- |
| Lines | 85% | 93.86% | pass |
| Functions | 90% | 93.66% | pass |
| Regions | 80% | 85.16% | pass |

## Expensive-campaign applicability

The diff from the approved Stage 9 completion through the qualified candidate
contains repository automation, metadata centralization, documentation, and
release-workflow changes. It contains no production Rust source change, no fuzz
target change, and no production dependency version or source change. The sole
lock addition assigns the already-reviewed `sha2` dependency to non-production
`xtask`. Therefore the exact completed Stage 9 critical/broader mutation runs
and five 900-second source, vocabulary, IR, formatter, and probe campaigns
remain applicable. No viable mutant or fuzz failure was accepted.

The same analysis found no allocation-affecting production change, so the
approved Stage 9 Valgrind Massif/Memcheck evidence remains applicable. The
candidate nevertheless reran both current benchmark profiles:

- release: 250 iterations for direct phase samples;
- soak: 5,000 iterations for direct phase samples;
- both included declaration-growth and concurrent-isolation measurements and
  completed successfully.

## Candidate reviews

The candidate reran the automated dependency, package-boundary, test-layout,
traceability, package-license inheritance, source-origin, generated-output,
workflow, and static repository checks. `cargo-audit 0.22.2` loaded 1,242
RustSec advisories and found no vulnerability in either the 22-dependency root
release lock or the 26-dependency isolated fuzz lock. The existing threat,
static-work-product, dependency, quality, standards, and residual-risk records
were reviewed against the candidate changes; none was invalidated.

## Tool identity

| Tool | Qualified version |
| --- | --- |
| Rust | `rustc 1.98.1 (48a229cea 2026-09-01)`, LLVM 22.1.8 |
| Cargo | `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| LLVM coverage | `cargo-llvm-cov 0.9.1` |
| Mutation | `cargo-mutants 27.1.0` |
| Fuzzing | `cargo-fuzz 0.13.2` |
| Memory analysis | `valgrind-3.27.1` |
| Advisory scan | `cargo-audit-audit 0.22.2` |

All required Step 6 results passed without lowered thresholds, reduced scope,
accepted viable mutants, or informational treatment of a failed required gate.
