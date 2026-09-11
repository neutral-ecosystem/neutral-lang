````md
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral

Neutral is a portable declarative language for describing structured,
platform-independent data and behavior.

This repository contains the frozen **v0 language specification**, its Rust
implementation, conformance corpus, standalone reader and probe, formatter,
command-line tooling, quality infrastructure, and release automation.

The compiler currently supports:

- scalar values
- nominal records and defaults
- invariant lists
- immutable-value reuse
- typed identity references
- captured closed vocabularies
- validated logical IR
- versioned NIR-CBOR serialization

`neutral-probe` can inspect encoded Neutral artifacts independently, without
linking against the compiler implementation.

---

## Quick start

### Linux

Install the latest stable Rust toolchain with Rustfmt and Clippy, clone the
repository, then run:

```sh
./scripts/linux/bootstrap.sh
cargo xtask dev
````

### Windows

From PowerShell:

```powershell
.\scripts\win\bootstrap.ps1
cargo xtask dev
```

The host bootstrap scripts only verify prerequisites and record ignored
environment evidence.

They do **not**:

* install software
* elevate privileges
* modify shell configuration
* define compiler policy
* define test policy
* define release policy

Repository policy remains implemented through `cargo xtask` and tracked
configuration.

---

## Project commands

`cargo xtask` is the stable, platform-neutral interface used by contributors,
CI, quality checks, and release preparation.

| Purpose                        | Command                                                    |
| ------------------------------ | ---------------------------------------------------------- |
| Daily development              | `cargo xtask dev`                                          |
| Exact pull-request CI gate     | `cargo xtask ci pr`                                        |
| Format check                   | `cargo xtask fmt`                                          |
| Apply formatting               | `cargo xtask fmt --write`                                  |
| Lint                           | `cargo xtask lint`                                         |
| Compile and repository checks  | `cargo xtask check`                                        |
| Unit tests                     | `cargo xtask test unit`                                    |
| Smoke tests                    | `cargo xtask test smoke`                                   |
| Integration tests              | `cargo xtask test integration`                             |
| System tests                   | `cargo xtask test system`                                  |
| Conformance tests              | `cargo xtask test conformance`                             |
| Property tests                 | `cargo xtask test property`                                |
| Security tests                 | `cargo xtask test security`                                |
| Complete test suite            | `cargo xtask test all`                                     |
| Performance tests              | `cargo xtask test performance --profile pr\|release\|soak` |
| Normal quality gate            | `cargo xtask quality`                                      |
| Release quality gate           | `cargo xtask quality --profile release`                    |
| Quality status                 | `cargo xtask quality status`                               |
| Render quality evidence        | `cargo xtask quality render`                               |
| Verify quality evidence        | `cargo xtask quality verify`                               |
| Evaluate retained quality      | `cargo xtask quality evaluate --profile pr\|release`       |
| Approve release quality        | `cargo xtask quality approve --release <version>`          |
| Development build              | `cargo xtask build --profile dev`                          |
| Release build                  | `cargo xtask build --profile release`                      |
| Documentation                  | `cargo xtask docs`                                         |
| Coverage                       | `RUSTUP_TOOLCHAIN=nightly cargo xtask coverage`            |
| Fuzz smoke run                 | `RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke`          |
| Full fuzz campaign             | `RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz campaign`       |
| Validate an artifact           | `cargo xtask validate <artifact>`                          |
| Validate repository binaries   | `cargo xtask validate binaries`                            |
| Assemble distribution          | `cargo xtask package`                                      |
| Prepare a release candidate    | `cargo xtask release prepare`                              |
| Show current version           | `cargo xtask version show`                                 |
| Check version consistency      | `cargo xtask version check`                                |
| Prepare a new version          | `cargo xtask version prepare <version>`                    |
| Install active portable plan   | `cargo xtask portable install <directory>`                 |
| Verify active portable plan    | `cargo xtask portable verify`                              |
| Archive portable plan          | `cargo xtask portable snapshot`                            |
| Verify development environment | `cargo xtask environment verify`                           |
| Print environment manifest     | `cargo xtask environment manifest`                         |
| Remove generated evidence      | `cargo xtask clean`                                        |

Use `cargo docs` directly when standard Cargo-generated API documentation is
preferred.

---

## Development workflow

The normal development workflow is intentionally small.

### 1. Verify the environment

Run once after cloning, or whenever the toolchain changes:

```sh
cargo xtask bootstrap
```

### 2. Develop

Run the complete local development loop:

```sh
cargo xtask dev
```

This performs the normal formatting, compile checks, linting, tests, and
documentation checks.

### 3. Verify before pushing

Run:

```sh
cargo xtask ci pr
```

This executes the same non-mutating gate used by hosted pull-request CI.

The repository does not maintain a separate CI-only implementation of the test
or quality logic.

---

## Quality and evidence

Quality checks produce machine-readable evidence under `test-results/`.

Aggregate runs retain a run directory containing at least:

```text
events.jsonl
summary.json
```

Generated evidence is automation-owned and should not be edited manually.

This includes:

* quality summaries
* generated Markdown status
* test results
* environment manifests
* workflow records
* version plans
* package manifests
* checksums
* release preparation summaries

Repository quality policy is tracked under:

```text
config/
quality/
```

---

## Release model

Release qualification is performed from a clean checked-out `main` `HEAD`.

The release version is derived from the authoritative workspace version in the
root `Cargo.toml`.

The corresponding publication tag is:

```text
v<workspace-version>
```

For example:

```text
v0.1.0
```

Release preparation validates repository state and fails closed when required
conditions are not satisfied, including:

* the wrong branch is checked out
* the working tree is dirty
* the version is inconsistent
* required quality evaluation is missing
* release approval is missing
* generated evidence is stale
* distribution scope is invalid

Release tooling prepares and validates artifacts locally.

It does **not** automatically:

* push commits
* create remote tags
* upload artifacts
* publish packages
* create releases

Publication remains an explicit operation outside release preparation.

### Release workflow

A normal release qualification sequence is:

```sh
cargo xtask quality evaluate --profile release
cargo xtask quality approve --release <version>
cargo xtask release prepare
```

Release preparation produces candidate artifacts and retained evidence under:

```text
test-results/release/
```

---

## Versioning

The root workspace version is the authoritative release version.

Inspect it with:

```sh
cargo xtask version show
```

Validate repository-wide consistency with:

```sh
cargo xtask version check
```

Prepare a new version with:

```sh
cargo xtask version prepare <version>
```

For example:

```sh
cargo xtask version prepare 0.1.1
```

Version propagation is automated so release metadata does not need to be
updated manually across the repository.

---

## Repository layout

| Path                    | Responsibility                                                               |
| ----------------------- | ---------------------------------------------------------------------------- |
| `crates/`               | Versioned Rust implementation, binaries, and crate-owned tests               |
| `conformance/`          | Released contracts, fixtures, reference data, and verification oracles       |
| `portable/`             | Optional active portable-development plan                                    |
| `quality/`              | Quality policy, maintained reviews, approvals, and retained release evidence |
| `config/`               | Machine-readable repository policy                                           |
| `scripts/`              | Thin host-specific adapters                                                  |
| `fuzz/`                 | Fuzz harnesses and subsystem definitions                                     |
| `xtask/`                | Stable project commands and shared automation policy                         |
| `target/`               | Ignored Cargo build and Rustdoc output                                       |
| `test-results/`         | Ignored generated test, quality, analysis, and release evidence              |
| `test-results/release/` | Candidate packages, SBOMs, checksums, and release summaries                  |

Each tracked top-level project directory documents its own responsibility and
lifecycle.

The executable repository ownership inventory is defined in:

```text
config/repository-layout.toml
```

---

## Supported platforms

The primary supported system-test target is:

```text
x86_64-unknown-linux-gnu
```

The Windows target:

```text
x86_64-pc-windows-msvc
```

is currently experimental.

Normal development uses the repository-selected **stable Rust toolchain**.

Nightly is required only for specialized analysis such as coverage and
coverage-guided fuzzing.

---

## Optional analysis toolchain

Install the nightly toolchain without replacing stable as the repository
default:

```sh
rustup toolchain install nightly --profile minimal
rustup component add --toolchain nightly llvm-tools-preview
cargo install cargo-llvm-cov cargo-fuzz
```

Run coverage with:

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
```

Run the fuzz smoke suite with:

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke
```

Run complete fuzz campaigns with:

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz campaign
```

Full campaigns use the configured **900-second budget per subsystem**.

Mutable fuzz corpora, generated findings, and crash artifacts are not normative
conformance fixtures.

---

## Coverage

Coverage is a specialized nightly-only analysis command.

Run:

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
```

The generated report is written to:

```text
test-results/analysis/coverage/html/index.html
```

The machine-readable summary is written to:

```text
test-results/analysis/coverage/coverage.json
```

Current production coverage gates are:

| Metric            | Minimum |
| ----------------- | ------: |
| Line coverage     |     85% |
| Function coverage |     90% |
| Region coverage   |     80% |

Production coverage excludes only the repository automation implementation in
`xtask/`.

`xtask` command and policy behavior is verified separately.

The reviewed scope and thresholds are defined in:

```text
config/quality-gates.toml
```

Coverage exclusions are repository policy. They must not be introduced through
one-off local command-line filters.

---

## Environment verification

For normal development:

```sh
cargo xtask bootstrap
```

For a complete workstation audit:

```sh
cargo xtask environment verify
```

The environment verifier checks the supported host, Rust toolchains, analysis
tools, and packaging prerequisites.

When an optional or required tool is missing, it reports the appropriate
installation command instead of modifying the system automatically.

Generate a path-independent environment identity record with:

```sh
cargo xtask environment manifest
```

When both Rust toolchains are installed, environment verification should report
stable and nightly independently rather than changing the repository default.

---

## Generated outputs

Cargo build and Rustdoc output belongs under: `target/`

Generated repository evidence belongs under: `test-results/`

This includes:

```text
test-results/
├── analysis/
├── quality/
├── release/
└── ...
```

Both roots are ignored where appropriate and are treated as generated output,
not source-of-truth project state.

Released contracts, fixtures, conformance oracles, and maintained quality policy
remain tracked.

---

## Cleanup safety

Use:

```sh
cargo xtask clean
```

to remove generated repository evidence.

Cleanup is intentionally restricted to the validated relative result root.

It cannot target:

* the repository root
* a parent directory
* an absolute path
* released conformance data
* tracked quality policy
* installed portable plans

This prevents cleanup automation from becoming a destructive general-purpose
filesystem command.

---

## Portable development plans

`portable/` may contain an active development plan imported from the Neutral
roadmap.

It is a planning input, not part of released language conformance.

Replacing or removing the active portable plan does not modify already released
contracts, fixtures, or conformance behavior.

Archived development plans remain outside the normative language definition.

---

## Troubleshooting

### Toolchain or host verification fails

Run:

```sh
cargo xtask environment verify
```

It reports the missing or incompatible dependency and the expected installation
command.

### Formatting fails

Run:

```sh
cargo xtask fmt --write
```

Then rerun the original command.

### CI fails locally

Run the failing stable `cargo xtask` subcommand directly.

CI delegates to the same repository commands used during local development.

### Generated results appear stale

Inspect:

```text
test-results/
```

Remove them when appropriate with:

```sh
cargo xtask clean
```

### Release preparation fails

Check that:

* `main` is checked out
* `HEAD` is the intended release commit
* the working tree is clean
* the workspace version is correct
* release quality evaluation is current
* release approval exists
* distribution scope is valid

Then rerun:

```sh
cargo xtask release prepare
```

---

## Documentation

Additional repository documentation:

* [Quality system](quality/README.md)
* [Platform adapters](scripts/README.md)
* [Automation internals](xtask/README.md)

The completed v0 development plan is archived in the Neutral roadmap.

Released conformance is defined by the versioned specification, contracts,
fixtures, oracles, and validated external formats stored in this repository.

