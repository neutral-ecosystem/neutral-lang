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
| `src/lib.rs`, `src/interface.rs` | Public entry point, stable command grammar, routing, and help |
| [`src/commands/`](src/commands/README.md) | Development, docs, tests, analysis, quality, distribution, and portable task composition |
| [`src/checks/`](src/checks/README.md) | Repository ownership, dependencies, traceability, test layout, and tracked-file hygiene |
| [`src/config/`](src/config/README.md) | Typed configuration, Cargo discovery, stable names, and formatting-preserving manifest edits |
| [`src/runtime/`](src/runtime/README.md) | Shared reporting, subprocess execution, progress, workflow logs, environment, and safe result paths |
| [`src/conformance/`](src/conformance/README.md) | Released bundle discovery, active suite selection, and preflighted fixture/hash edits |
| [`src/quality/`](src/quality/README.md) | Source-bound measurement evidence and approval ledger management |
| [`src/release/`](src/release/README.md) | Typed fail-closed release scope, artifact metadata, and approval validation |
| `src/versioning.rs` | Package version, dependency lock, and inherited-contract checks |
| [`templates/`](templates/README.md) | Workspace landing page and shared Rustdoc presentation templates |
| `tests/unit/` | Parser, safety, boundary, release-plan, and helper tests |
| `tests/command_output.rs` | Executable-level reporting and script-compatible stdout contracts |

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

Human output uses shared `[info]`, `[warn]`, and `[error]` reporting on stderr,
with aligned `START`, `RUN`, `PASS`, `FAIL`, `CMD`, and `FILE` labels. Capable
terminals color these statuses; redirected logs are plain by default.
`CARGO_TERM_COLOR=always|never|auto` overrides automatic color selection;
nonempty `NO_COLOR` disables color, including inherited Cargo/nextest output.
Captured measurement reports are always color-free. Every command, including
help and script-facing commands, prints lifecycle summaries on stderr. Statuses
are explicit at each call site, not guessed from message wording; every line in
a multiline diagnostic gets a category. Failures produce one final contextual
error instead of duplicate command-failure summaries. Workflows number their
steps, and measured tools
show elapsed-time heartbeats while retaining complete stdout/stderr reports.
Fuzz targets additionally show configured-budget progress and approximate ETA;
elapsed budget never substitutes for a successful tool exit or verified receipt.
Report paths are workspace-relative, commands quote arguments containing spaces,
and timings use readable units. Redirect stderr to retain the human log;
`release tag` and `environment manifest` keep their script-facing stdout format.

When adding a command, route it through `run`, keep task composition in
`commands/`, and place reusable policies in their owning boundary. Use
`output::start`, `pass`, `warn`, `error`, `invocation`, and `file` for human
reporting rather than direct `println!` calls or embedded ANSI escapes. Native
Cargo/nextest diagnostics retain their own format and inherit the shared color
policy; retained measurement files stay color-free.

Tests use compact reporting by default: progress and summaries remain visible,
along with slow tests and failures, without a line for every passing test.
Set `testing.verbose = true` in `config/automation.toml` or use
`NEUTRAL_TEST_VERBOSE=true cargo xtask test all` for detailed test statuses.
Reporting settings never change test selection or quality acceptance.

`cargo xtask docs` builds the searchable workspace Rustdoc site under ignored
`target/doc/` from Cargo metadata. The published copy is the [Neutral API
documentation website](https://neutral-lang-doc.younesrabeh.workers.dev/). The
`docs/` directory owns hand-written, task-oriented guides. Package additions,
removals, descriptions, ownership, versions, and dependency relationships
therefore require no hand-maintained HTML package list.
