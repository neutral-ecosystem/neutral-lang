<!-- SPDX-License-Identifier: Apache-2.0 -->

# Quality and analysis

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Repository quality policy is tracked under `config/` and `quality/`. Generated
machine-readable evidence belongs under ignored `test-results/` directories and
must not be edited manually.

Release measurements are produced automatically by the commands below. Each
successful run retains tool stdout/stderr, native reports, exact report digests,
compiler identity, duration, and an input fingerprint. Changing code, tests,
fixtures, lockfiles, or configuration invalidates earlier measurements;
documentation-only changes do not. New, untracked source files also participate.
Historical `status` and `observed_*` configuration fields are not passing evidence.

## Quality workflow

| Need | Command |
| --- | --- |
| Run the normal gate | `cargo xtask quality` |
| Run release qualification | `cargo xtask quality --profile release` |
| Inspect durable status | `cargo xtask quality status` |
| Render maintained status | `cargo xtask quality render` |
| Verify maintained evidence | `cargo xtask quality verify` |
| Evaluate retained evidence | `cargo xtask quality evaluate --profile pr\|release` |

Every aggregate workflow records `events.jsonl` and `summary.json` beneath its
run directory in `test-results/workflows/`.

## Coverage and fuzzing

Coverage-guided fuzzing and LLVM coverage use an isolated nightly toolchain;
stable remains the project default.

```sh
rustup toolchain install nightly --profile minimal
rustup component add --toolchain nightly llvm-tools-preview
cargo install cargo-llvm-cov cargo-fuzz
```

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz campaign
```

Full fuzz campaigns use the configured 900-second budget per subsystem. Mutable
fuzz corpora, findings, and crash artifacts are not normative conformance data.

Coverage HTML is written to
`test-results/analysis/coverage/html/index.html`; its machine-readable summary
is `test-results/analysis/coverage/coverage.json`. Thresholds and reviewed
exclusions are defined in [`config/quality-gates.toml`](../config/quality-gates.toml).

## Release measurements

Install the current analysis tools and the system's Valgrind package:

```sh
cargo install cargo-mutants cargo-audit --locked
```

After preparing the workspace version, run these on the final source inputs:

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
cargo xtask mutate
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz campaign
cargo xtask test performance --profile release
cargo xtask test performance --profile soak
```

The performance commands automatically capture phase/end-to-end, growth, and
concurrency timings, Massif heap snapshots, and Memcheck allocation/leak results.
The release soak uses the benchmark's extended-soak profile. Use an otherwise
idle controlled runner; comparing measurements from different machines is not a
performance regression test.

Commit source changes, then run `cargo xtask quality evaluate --profile release`.
It runs fresh RustSec checks for every declared dependency lock and verifies the
measured gates. Missing, stale, modified, incomplete, or failing reports block
release qualification; manual Markdown approval cannot substitute for them.

`cargo xtask quality approve --release <version>` automatically retains verified
reports under `quality/evidence/v<version>/gates/<input-fingerprint>/`. Commit
those snapshots so a fresh CI checkout can verify them. Existing historical
approval records remain unchanged when new source-bound reports are retained.
Advisory scans expire according to `config/automation.toml` and are rerun during
release qualification. No additional handwritten per-command evidence is needed.
