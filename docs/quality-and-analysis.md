<!-- SPDX-License-Identifier: Apache-2.0 -->

# Quality and analysis

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Repository quality policy is tracked under `config/` and `quality/`. Generated
machine-readable evidence belongs under ignored `test-results/` directories and
must not be edited manually.

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
