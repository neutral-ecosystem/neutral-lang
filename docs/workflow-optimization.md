<!-- SPDX-License-Identifier: Apache-2.0 -->

# Developer command flow and configuration

[< Documentation hub](README.md)

The normal loop is deliberately short:

1. Run `cargo xtask dev` while implementing. It formats source and runs the
   ordinary compile, lint, test, command-smoke, and documentation checks.
2. Run `cargo xtask ci pr` before pushing. This read-only composition checks
   formatting and the same repository policies used in CI.
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
Cargo's own `CARGO_TARGET_DIR` override is respected for documentation and
release binaries; it does not redirect the independent test-results root.

Some identifiers are contracts rather than setup choices: release approval
still binds to `main` HEAD, frozen conformance inputs are selected by exact
release, and binary/protocol names change only with their owning contract.
