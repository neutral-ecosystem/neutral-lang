<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral repository automation

`xtask` is the single platform-neutral command and policy layer for Neutral's
developer, CI, quality, documentation, package, and release-preparation flows.
It is a non-published workspace package and must remain outside every production
dependency graph.

Repository checks also enforce `quality/manifest.toml`, keeping durable quality
policy, maintained reviews, and versioned release evidence complete and
separate from ignored raw tool output.

The managed `quality status|evaluate|approve|render|verify` workflow binds a
release record to its evaluated commit and gate configuration, then generates
the human-readable status page. Evidence can evolve without per-edit digest
bookkeeping.

The active conformance stage is derived internally from required manifest
suites, not a second stage-status file. `cargo xtask dev`, `cargo xtask ci pr`,
and `cargo xtask release prepare` are ordered aggregate workflows. Every step
emits start/pass/fail events plus a
final summary beneath `test-results/workflows/`, including duration, source
commit, worktree cleanliness, package version, selected compiler, and project
license. A failing run
retains the exact failed step instead of losing all context behind a final exit
code.

## Responsibility in the ecosystem

This package owns stable command parsing, Cargo invocation, repository boundary
and traceability checks, durable test selection, configured quality thresholds,
generated-evidence paths, release-candidate validation, and fail-closed package
assembly. It orchestrates production crates but does not implement Neutral
syntax, semantics, IR, encoding, reading, formatting, or host acquisition.

The root `[workspace.package].version` is the sole package-release version.
`version` commands verify its inheritance and lock coherence, while release
commands derive tag `v<version>` instead of reading a duplicated version.
`portable install <directory>` atomically copies and verifies a reviewed package
without overwriting an existing one. `portable verify` enforces the
version-independent active-package boundary without prescribing contract or
checklist filenames. `portable snapshot` creates an ignored, SHA-256-addressed
archive candidate without network access.

Inputs are tracked repository configuration and explicit command arguments.
Outputs are terminal messages prefixed with a category and ignored generated
files beneath `target/` or `test-results/`. It does not tag, push, upload,
publish, mutate frozen contracts, or write into another repository.

## Structure

| Path | Ownership |
| --- | --- |
| `src/interface.rs` | Stable stage-free command grammar and help |
| `src/configuration.rs`, `src/configuration_models.rs` | Standard TOML access, typed schemas, and local automation defaults |
| `src/cargo_discovery.rs` | Typed Cargo workspace/target/graph and build-message discovery |
| `src/manifest_updates.rs`, `src/fixtures.rs` | Formatting-preserving version/lock and preflighted fixture/hash edits |
| `src/test_execution.rs` | Configured nextest/Cargo execution, exact inventory counts, and full-gate doctests |
| `src/environment.rs` | Toolchain and host checks; environment evidence |
| `src/results.rs` | Ignored generated-result paths and safe cleanup |
| `src/versioning.rs` | Package version, dependency lock, and inherited-contract checks |
| `src/portable_stage.rs` | Derived active conformance-suite stage for environment evidence |
| `src/release.rs` | Typed fail-closed release-scope parsing |
| `src/constants.rs` | Stable package, contract, and output names—not user defaults |
| `src/lib.rs` | Command composition and remaining repository checks |
| `tests/unit/` | Parser, safety, boundary, release-plan, and helper tests |

Use `cargo xtask --help` for the authoritative command list. The root README
documents contributor usage and migration from removed legacy names. Linux and
Windows scripts are thin adapters that delegate here.

Local defaults live in [`config/automation.toml`](../config/automation.toml).
The [developer command guide](../docs/workflow-optimization.md) maps other
changeable policies to their owning configuration files.

Install the latest normal test runner with `cargo install cargo-nextest --locked`.
`dev` runs the local loop without site generation; `ci pr` adds doctests and the
documentation build. `NEUTRAL_TEST_RUNNER=cargo` explicitly selects the standard
Cargo compatibility backend. Missing nextest never silently changes the backend.

Full coverage-guided fuzzing, LLVM coverage, and mutation analysis require their
documented external Cargo tools. Missing tools fail their command; bounded fuzz
regression tests are never reported as a full fuzz campaign.

`cargo xtask docs` builds the searchable workspace Rustdoc site under ignored
`target/doc/` from Cargo metadata. The published copy is the [Neutral API
documentation website](https://neutral-lang-doc.younesrabeh.workers.dev/). The
`docs/` directory owns hand-written, task-oriented guides. Package additions,
removals, descriptions, ownership, versions, and dependency relationships
therefore require no hand-maintained HTML package list.
