<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 dynamic quality evidence

Evaluation date: 2026-09-08. Owner: maintainer. Candidate state: development
quality evaluation, not a release candidate.

## Toolchain

- Repository compiler: stable Rust selected by `rust-toolchain.toml`.
- Coverage compiler: `rustc 1.100.0-nightly` from 2026-09-07 with
  `llvm-tools-preview`.
- `cargo-llvm-cov 0.9.1`.
- `cargo-mutants 27.1.0`.
- `cargo-fuzz 0.13.2`.

The stable repository toolchain contract remains unchanged. Nightly is selected
only for LLVM coverage and coverage-guided fuzz commands.

## Coverage

`cargo xtask coverage` executed the complete workspace test selection. The
retained machine-readable summary is generated at
`test-results/analysis/coverage/summary.json`.

| Measure | Required | Observed | Result |
| --- | ---: | ---: | --- |
| Lines | 85% | 90.57% | Pass |
| Functions | 90% | 90.71% | Pass |
| Regions | 80% | 81.72% | Pass |

No threshold or coverage scope was reduced. The pass comes from the configured
`RUSTUP_TOOLCHAIN=nightly cargo xtask coverage` workspace/all-targets command.

## Mutation

`cargo xtask mutate` tested the configured critical target,
`crates/neutral-ir/src/language.rs`, in cargo-mutants' isolated scratch tree.
All 38 generated mutants were caught, meeting the configured 100% target.

The final wider 271-mutant review caught 244 mutants, classified 27 as
unviable, and left no missed viable mutant. Follow-up exact-number, decoder,
diagnostic, logical-equality, parser-fault, CLI-fault, and automation tests
closed the broader review without accepting an equivalent-risk exception.

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
(4 logical CPUs, 7.3 GiB RAM). Direct optimized benchmark binaries were used
so Cargo build activity did not affect allocation or peak-memory measurements.

| Profile | Peak RSS | Result |
| --- | ---: | --- |
| `release` (250 iterations) | 446,476 KiB | pass |
| `extended-soak` (50,000 iterations) | 463,128 KiB | pass |

Five direct release samples each completed in 0.02 seconds as measured by
`/usr/bin/time`; peak RSS ranged from 3,616 KiB to 3,848 KiB. The timer's
hundredth-second resolution cannot distinguish the five samples further, so
the benchmark's phase timings remain the more precise latency evidence.

Valgrind 3.27.1 supplied the component-level allocation review:

| Profile | Massif useful heap peak | Massif total peak | Result |
| --- | ---: | ---: | --- |
| `release` | 453,799 B | 524,640 B | pass |
| `extended-soak` (50,000 iterations) | 453,805 B | 524,640 B | pass |

The extended-soak peak differs by six useful-heap bytes from release and shows
no retained-growth trend. Release Memcheck exercised 322,503 allocations and
322,502 frees (40,042,839 bytes allocated in total), with zero reported memory
errors and zero definite, indirect, or possible leaks. One 544-byte
still-reachable runtime block remains at process exit.

The extended soak completed compilation, reader validation, encoding, decoding,
probe traversal, declaration growth, and eight-worker isolation without a
failure or swap. Its slowest phase, artifact decoding, took 2,535,879,293 ns.
This closes the allocation-accounting evidence gap without adding an unsafe
replacement global allocator.
