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

## Outputs and safety

Cargo build and rustdoc output belongs under ignored `target/`. Generated test,
quality, version-plan, package, and release evidence belongs under ignored
`test-results/`. `cargo xtask clean` removes only the validated relative result
root; it cannot target the repository, a parent path, or an absolute path.
Normative fixtures, oracles, contracts, and the portable plan are never cleanup
targets.

For the language and implementation lifecycle, start with
[the active v0 plan](portable/PLAN.md). Quality evidence and residual risks are
indexed in [quality/README.md](quality/README.md). Platform adapter ownership is
documented in [scripts/README.md](scripts/README.md), and automation internals in
[xtask/README.md](xtask/README.md).
