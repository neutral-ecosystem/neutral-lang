<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral

Neutral is a portable declarative language. This repository contains the frozen
v0 specification, its Rust implementation, conformance corpus, standalone
reader/probe, formatter, command-line tools, and the automation used to verify
and package them. Stage 9 hardening is approved; Stage 10 is replacing the old
stage-dependent workflow before final v0 qualification.

The compiler supports scalar values, nominal records and defaults, invariant
lists, immutable-value reuse, typed identity references, captured closed
vocabularies, validated logical IR, and the versioned NIR-CBOR external format.
`neutral-probe` inspects encoded artifacts without linking the compiler.

## Start here

Install the latest stable Rust toolchain with Rustfmt and Clippy, clone the
repository, and run the one adapter for your host:

```sh
./scripts/linux/bootstrap.sh
cargo xtask --help
cargo xtask quality
```

On Windows PowerShell:

```powershell
.\scripts\win\bootstrap.ps1
cargo xtask --help
cargo xtask quality
```

Bootstrap verifies prerequisites and writes only ignored environment evidence.
It does not install software, elevate privileges, change shell configuration,
or define compiler/test/release policy.

## Stable project commands

`cargo xtask` is the platform-neutral interface used by contributors, CI, and
release preparation:

| Purpose | Command |
| --- | --- |
| Format | `cargo xtask fmt [--write]` |
| Lint | `cargo xtask lint` |
| Compile and repository checks | `cargo xtask check` |
| Tests | `cargo xtask test unit\|smoke\|integration\|system\|conformance\|property\|security\|all` |
| Performance | `cargo xtask test performance --profile pr\|release\|soak` |
| Normal quality composition | `cargo xtask quality` |
| Release quality composition | `cargo xtask quality --profile release` |
| Developer/release build | `cargo xtask build --profile dev\|release` |
| Documentation | `cargo xtask docs` or `cargo docs` |
| Coverage | `RUSTUP_TOOLCHAIN=nightly cargo xtask coverage` |
| Fuzzing | `RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke\|campaign` |
| Artifact validation | `cargo xtask validate <artifact>` or `cargo xtask validate binaries` |
| Distribution assembly | `cargo xtask package` |
| Local release preparation | `cargo xtask release prepare` |
| Versioning | `cargo xtask version show\|check\|prepare <version>` |
| Active portable lifecycle | `cargo xtask portable verify\|snapshot` |
| Generated-evidence cleanup | `cargo xtask clean` |

The release commands derive annotated tag `v<workspace package version>` from
the root `Cargo.toml`. They fail closed unless that tag resolves to the clean
checked-out `HEAD` and the Stage 9 approval and distribution scope agree. They
never push, upload, publish, or create tags.

## Workflow migration

The stable interface replaces milestone-specific and duplicate wrappers:

| Previous entry point | Stable replacement | Status |
| --- | --- | --- |
| `cargo xtask format [--write]` | `cargo xtask fmt [--write]` | removed |
| Direct long `cargo clippy` flags | `cargo xtask lint` | replaced |
| Separate boundary/test-layout/traceability commands | `cargo xtask check` | composed |
| `cargo xtask clean-results` | `cargo xtask clean` | removed |
| `cargo xtask ci stage1` / `ci nightly` | stable commands selected by CI | removed |
| Handwritten release command sequences | `cargo xtask release prepare` | replaced |
| Platform scripts containing project policy | thin `scripts/linux` and `scripts/win` adapters | replaced |

The internal `cargo xtask ci pr|release` aliases remain available for CI
compatibility, but they call the same stable compositions and do not define a
second workflow.

## Repository map

| Path | Owner and lifecycle |
| --- | --- |
| `crates/` | Versioned implementation, binaries, and crate-owned tests |
| `portable/` | Active version plan/contracts/fixtures; snapshot to the roadmap before rollover |
| `quality/` | Tracked reviewed quality, security, dependency, and residual-risk conclusions |
| `config/` | Tracked machine-readable repository policy |
| `scripts/` | Thin Linux and Windows host adapters |
| `fuzz/` | Tracked subsystem harnesses; mutable corpora/findings are ignored |
| `xtask/` | Stable project commands and reusable policy implementation |
| `target/` | Ignored Cargo builds and Rustdoc |
| `test-results/` | Ignored generated quality, version, snapshot, and release evidence |
| `test-results/release/` | Ignored candidate packages, SBOMs, and preparation summaries |

Every tracked top-level directory has a README describing its responsibility.
`config/repository-layout.toml` is the executable ownership inventory.

## Supported hosts and specialized tools

The supported and primary system-test host is `x86_64-unknown-linux-gnu`.
`x86_64-pc-windows-msvc` is experimental. Normal work uses the selected stable
toolchain. Coverage and coverage-guided fuzzing additionally require nightly:

```sh
rustup toolchain install nightly --profile minimal
rustup component add --toolchain nightly llvm-tools-preview
cargo install cargo-llvm-cov cargo-fuzz
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke
```

Full fuzz campaigns use the configured 900-second budget per subsystem. Their
mutable corpora and crashes are never normative fixtures.

Coverage is a nightly-only analysis command. It preserves a browsable report at
`test-results/analysis/coverage/html/index.html` and a machine-readable summary
at `test-results/analysis/coverage/coverage.json`, while enforcing the configured
85% line, 90% function, and 80% region gates. Production coverage excludes only
the repository-automation `xtask`, whose command and policy paths are tested
separately. That reviewed scope is declared in `config/quality-gates.toml`; any
future exclusion requires another explicit policy review, never a one-off
command-line filter.

## Outputs and safety

Cargo build and rustdoc output belongs under ignored `target/`. Generated test,
quality, version-plan, package, and release evidence belongs under ignored
`test-results/`. `cargo xtask clean` removes only the validated relative result
root; it cannot target the repository, a parent path, or an absolute path.
Normative fixtures, oracles, contracts, and the portable plan are never cleanup
targets.

## Troubleshooting

- Run `cargo xtask environment verify` when a toolchain or host check fails.
- Run `cargo xtask fmt --write`, then rerun the command after formatting errors.
- Run the failing stable subcommand locally; CI contains no separate test logic.
- Inspect `test-results/` for generated summaries. Run `cargo xtask clean` only
  when those ignored results should be discarded.
- Release preparation requires a clean checkout at the derived signed tag. It
  will reject a dirty tree, the wrong `HEAD`, missing approval, or stale scope.

For the language and implementation lifecycle, start with
[the active v0 plan](portable/PLAN.md). Quality evidence and residual risks are
indexed in [quality/README.md](quality/README.md). Platform adapter ownership is
documented in [scripts/README.md](scripts/README.md), and automation internals in
[xtask/README.md](xtask/README.md).
