<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 10 environment reproduction evidence

Evaluation date: 2026-09-08. Owner: maintainer. Scope: Stage 10, Step 5.

## Supported host and tools

The supported Linux bootstrap ran on Fedora Linux 44 Workstation, kernel
7.1.13-200.fc44.x86_64, target `x86_64-unknown-linux-gnu`. It generated the
path-independent ignored manifest at
`test-results/bootstrap/environment.json` and did not change the tracked or
untracked source status.

`cargo xtask environment verify` passed with:

| Tool | Verified identity |
| --- | --- |
| Stable Rust | rustc 1.98.1 |
| Cargo | cargo 1.98.1 |
| Rustfmt | rustfmt 1.9.0-stable |
| Clippy | clippy 0.1.98 |
| Isolated nightly | rustc 1.100.0-nightly (2026-09-07) |
| LLVM coverage | cargo-llvm-cov 0.9.1 |
| Fuzzing | cargo-fuzz 0.13.2 |
| Mutation | cargo-mutants 27.1.0 |
| Memory profiling | Valgrind 3.27.1 |
| Release shell | Git 2.55.0, POSIX `sh`, GNU tar 1.35, curl 8.18.0, sha256sum 9.10 |

Missing tools are aggregated into actionable diagnostics containing the exact
installation action. The ordinary bootstrap checks only the supported host,
stable Rust/Cargo, repository inputs, TLS/archive/checksum prerequisites, and
then delegates to `xtask`; normal CI does not require expensive analysis tools.

## Stable offline reproduction

A temporary clean Git source tree was assembled from the final Step 5 source
set with no `target/` or `test-results/`. The following sequence passed with
Cargo network access disabled:

```text
CARGO_NET_OFFLINE=true ./scripts/linux/bootstrap.sh
CARGO_NET_OFFLINE=true cargo xtask ci pr
git status --porcelain=v1
```

The final status was empty. Cargo rebuilt from the checked-in root lockfile,
all stable quality/tests/documentation passed, and generated products remained
ignored. The clean run also found and removed a portable-plan dependency on a
generated Rustdoc link, so `portable verify` no longer requires `target/doc/`.

## Isolated nightly and ignore policy

The default compiler reported stable Rust 1.98.1 before and after both
command-local analysis runs:

```text
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
```

Both passed. Nightly remains a named Rustup toolchain and does not replace the
repository-selected stable toolchain.

The executable repository check now requires ignores for Cargo/results, fuzz
corpora/findings/coverage, mutation output, Massif/Callgrind/perf/raw LLVM data,
local distribution roots, and editor state. It separately proves that the root
lockfile, toolchain manifest, release configuration, frozen contract manifest,
conformance manifest, and Linux bootstrap remain tracked.

Dependency/source checks reject ambient filesystem, environment, network, or
process APIs from the pure compiler source closure. Release configuration and
platform release adapters reject user-specific home paths. Source and
vocabulary acquisition continues through explicit captured host inputs; release
commands perform no ambient or network source lookup.

## Result

Stage 10 Step 5 passes. The documented clean Linux path reaches the exact local
PR CI composition offline, specialized nightly tooling stays isolated, and no
generated evidence or user-specific checkout state is required.
