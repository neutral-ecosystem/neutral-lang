<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 dynamic quality evidence

Evaluation date: 2026-09-07. Owner: maintainer. Candidate state: modified
development tree, not a release candidate.

## Toolchain

- Repository compiler: `rustc 1.98.0` stable.
- Isolated campaign compiler: `rustc 1.100.0-nightly (5a2be9f5f
  2026-09-06)` with `llvm-tools-preview`.
- `cargo-llvm-cov 0.9.1`.
- `cargo-mutants 27.1.0`.
- `cargo-fuzz 0.13.2`.

The nightly toolchain and Cargo tools were installed beneath `/tmp` and did not
change the repository's stable toolchain contract or the user's global Cargo
installation.

## Coverage

`cargo xtask coverage` executed the complete workspace test selection. The
retained machine-readable summary is generated at
`test-results/analysis/coverage/summary.json`.

| Measure | Required | Observed | Result |
| --- | ---: | ---: | --- |
| Lines | 85% | 84.91% | Fail |
| Functions | 90% | 80.46% | Fail |
| Regions | 80% | 75.55% | Fail |

No threshold was reduced. Missing coverage is concentrated in automation,
parser/decoder failure paths, and host command handling. The coverage gate
remains open until tests meet every configured threshold.

## Mutation

`cargo xtask mutate` tested the configured critical target,
`crates/neutral-ir/src/language.rs`, in cargo-mutants' isolated scratch tree.
All 38 generated mutants were caught, meeting the configured 100% target.

## Coverage-guided fuzz readiness

All five targets (`source`, `vocabulary`, `ir`, `formatter`, and `probe`) built
under nightly and completed 256 bounded libFuzzer executions without a Neutral
panic, hang, or sanitizer finding. This proves campaign readiness only; it does
not satisfy the required 900 seconds per target.

LeakSanitizer cannot run under the ptrace-managed development executor. The
readiness rerun therefore disabled leak detection for this environment only.
The full campaign must run on an untraced runner with normal sanitizer settings.

## Performance

The local release profile completed 250 iterations and the local soak profile
completed 5,000 iterations for compilation, reader validation, artifact
encoding/decoding, and probe traversal. Declaration-growth and eight-worker
concurrent-isolation profiles also completed. These informational runs do not
replace the required controlled-runner baseline, allocation measurement, or
extended retained soak evidence.
