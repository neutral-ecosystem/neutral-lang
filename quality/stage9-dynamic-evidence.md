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

The wider 271-mutant review caught 178, missed 66, and classified 27 as
unviable. Follow-up exact-number boundary tests reduced the focused
`ExactNumber` subset to 32 caught, 2 missed, and 3 unviable out of 37. The
broader mutation gate remains open until every viable selected mutant is either
caught or reviewed as an accepted equivalent-risk decision.

## Coverage-guided fuzzing

All five targets (`source`, `vocabulary`, `ir`, `formatter`, and `probe`) built
under the isolated nightly toolchain and completed their full 900-second
libFuzzer campaigns on an untraced runner. No target reported a Neutral panic,
hang, crash, timeout, or sanitizer finding.

| Target | Executions | Duration | Peak RSS |
| --- | ---: | ---: | ---: |
| `source` | 7,517,511 | 901s | 509 MiB |
| `vocabulary` | 10,720,881 | 901s | 576 MiB |
| `ir` | 85,143,833 | 901s | 484 MiB |
| `formatter` | 9,337,700 | 901s | 471 MiB |
| `probe` | 86,633,280 | 901s | 472 MiB |

The first readiness run established that LeakSanitizer cannot run under the
ptrace-managed development executor. The completed campaigns therefore used an
untraced runner with normal sanitizer configuration.

## Performance

The controlled runner was Fedora Linux 7.1.13 on an Intel Core i3-1115G4
(4 logical CPUs, 7.3 GiB RAM), using stable Rust 1.98.0. `/usr/bin/time -v`
recorded whole-command peak resident memory; it includes Cargo/build activity,
so it is a conservative process baseline rather than a component allocation
profile.

| Profile | Peak RSS | Result |
| --- | ---: | --- |
| `release` (250 iterations) | 446,476 KiB | pass |
| `extended-soak` (50,000 iterations) | 463,128 KiB | pass |

The extended soak completed compilation, reader validation, encoding, decoding,
probe traversal, declaration growth, and eight-worker isolation without a
failure or swap. Its slowest phase, artifact decoding, took 2,535,879,293 ns.
No supported allocation profiler (`valgrind` or `heaptrack`) is installed on
this runner, and the project forbids an unsafe replacement global allocator;
component-level allocation accounting therefore remains an explicit open risk.
