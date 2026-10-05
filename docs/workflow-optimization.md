<!-- SPDX-License-Identifier: Apache-2.0 -->

# Developer command flow and configuration

[< Documentation hub](README.md)

The normal loop is deliberately short:

1. Run `cargo xtask dev` while implementing. It formats source and runs the
   ordinary compile, lint, nextest, and command-smoke checks. It does not build
   the documentation site.
2. Run `cargo xtask ci pr` before pushing. This read-only composition checks
   formatting and the same repository policies used in CI, then runs the full
   test inventory, doctests, and documentation build.
3. For a release, follow [release and versioning](release-and-versioning.md).
   A release evaluation requires a clean `main` checkout; do not use it as a
   daily edit check.

Individual commands (`cargo xtask check`, `test all`, `docs`, `coverage`, and
others listed by `cargo xtask --help`) are useful while narrowing a failure.
Generated run summaries stay beneath the ignored result root; ordinary edits
do not need hand-written evidence files.

## Where to change an assumption

| Need | Source of truth |
| --- | --- |
| Cargo/Rust compiler command and generated-results default | [`config/automation.toml`](../config/automation.toml) |
| Test backend, configuration path, and local/CI profile selection | [`config/automation.toml`](../config/automation.toml) |
| Test parallelism, retries, timeouts, and JUnit reporting | [`config/nextest.toml`](../config/nextest.toml) |
| Package release version and license | root [`Cargo.toml`](../Cargo.toml) |
| Selected inherited language corpus | [`config/conformance.toml`](../config/conformance.toml) |
| Owned top-level directories and READMEs | [`config/repository-layout.toml`](../config/repository-layout.toml) |
| Test discovery minima and quality thresholds | [`config/test-suites.toml`](../config/test-suites.toml), [`config/quality-gates.toml`](../config/quality-gates.toml) |
| Generated-output inventory | [`config/generated-outputs.toml`](../config/generated-outputs.toml) |

`NEUTRAL_CARGO_COMMAND`, `NEUTRAL_RUSTC_COMMAND`, and
`NEUTRAL_TEST_RESULTS` override the corresponding local defaults for one
invocation. An alternative results root must be workspace-relative, ignored
by Git, and outside Cargo's `target/`; `cargo xtask clean` refuses a symlinked root. Keep the configured
default and the generated-output inventory aligned when changing the layout.
Cargo metadata resolves workspace members (including globs and excludes),
package names, dependency IDs, and the actual target directory. Cargo's own
configuration and `CARGO_TARGET_DIR` override are respected for documentation
and release binaries; they do not redirect the independent test-results root.

## Test execution

Output is compact by default. Nextest keeps its live progress display, summary,
slow-test warnings, and failure diagnostics; successful per-test rows are hidden.
The explicit Cargo backend uses libtest's compact reporter. Inventory checks,
JUnit reporting, exit status, and all quality requirements are unchanged.
Configure `testing.verbose` in `config/automation.toml` or override it for one run:

```sh
NEUTRAL_TEST_VERBOSE=true cargo xtask test all
```

Xtask status labels share semantic terminal colors with Cargo/nextest:
cyan for start/running, green for success, yellow for warnings, and red for
failure. Status text remains meaningful without color. Automatic color disables
ANSI styling in redirected logs and dumb terminals; `CARGO_TERM_COLOR=always`
explicitly forces color, `CARGO_TERM_COLOR=never` disables it, and nonempty
`NO_COLOR` takes precedence over either. Tool reports retained as quality evidence
never receive forced color. Machine-readable stdout remains unchanged.

Install once with `cargo install cargo-nextest --locked`. Container rebuilds
and CI install the latest runner automatically. Run a focused
`cargo xtask test <level>` while editing, `cargo xtask dev` to validate the local
change, then `cargo xtask ci pr` before pushing. No additional tracking document
is required for routine runs.

The nextest backend runs each test in its own process, rejects empty runs, and
retains a pretty-printed, structured inventory beneath `test-results/tests/`.
Category minima count actual runnable test names, not occurrences in console
output. Full gates ignore nextest default filters, use the configured CI profile,
and run `cargo test --workspace --doc --locked` separately. They retain JUnit
output under Cargo's nextest store (`target/nextest/ci/junit.xml` by default).
Retries are disabled in the repository profiles; CI completes the run after
failures to collect diagnostics. [Nextest configuration reference](https://nexte.st/docs/configuration/reference/).

For an explicit compatibility run without nextest:

```sh
NEUTRAL_TEST_RUNNER=cargo cargo xtask test all
```

In PowerShell use `$env:NEUTRAL_TEST_RUNNER = 'cargo'` before the command;
remove the override afterward with `Remove-Item Env:NEUTRAL_TEST_RUNNER`.
There is no silent backend fallback. Coverage, fuzzing, and mutation continue
to use their dedicated tools and unchanged thresholds.

## Configuration and manifest safety

Automation, layout, release authority, and test-minimum settings deserialize
through Serde and the standard TOML parser. Closed configuration schemas reject
unknown keys, missing fields, and wrong types; extensible conformance inputs
retain their contract-specific fields. Quoted strings, inline comments, and
multiline arrays no longer depend on line order or whitespace.

Version and fixture edits use `toml_edit` to preserve comments, spacing, and
unrelated fields. Lock edits select source-free workspace records and reject
missing/duplicate/stale records; equal-valued registry packages are unchanged.
Fixture synchronization matches paired fields by key, rejects escaping input
paths, and preflights both manifests before writing. Synchronizing hashes never
approves a language-contract change. Review the diff and run
`cargo xtask fixtures check` afterward.

Some identifiers are contracts rather than setup choices: release approval
still binds to `main` HEAD, frozen conformance inputs are selected by exact
release, and binary/protocol names change only with their owning contract.

Quality measurements and evidence validation share typed settings loaded once
per invocation. Gate identifiers are checked enums rather than unchecked strings;
receipts retain their existing serialized names. Policy changes during a
measurement invalidate it rather than silently accepting a different threshold.
These internal refactors require no new developer commands or manual logs.
