<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development workflow

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Normal development uses the repository-selected latest stable Rust toolchain.
The `cargo xtask` interface owns the order and policy used locally and in CI.

## First setup

Install the latest test runner and fetch the locked workspace dependencies:

```sh
cargo install cargo-nextest --locked
cargo fetch --locked
```

Install Python 3 through your platform package manager as well. Project identity
conformance runs a retained independent standard-library encoder; it is a real
test prerequisite, not an optional skipped check. Discovery tries `python3`, then
`python`; `NEUTRAL_IDENTITY_REFERENCE_PYTHON` selects another interpreter path.
No Python packages or virtual environment are required.

Run the platform bootstrap adapter after cloning or when the toolchain changes:

```sh
./scripts/linux/bootstrap.sh
```

```powershell
.\scripts\win\bootstrap.ps1
```

For a complete workstation audit, run:

```sh
cargo xtask environment verify
```

## Daily loop

```sh
cargo xtask dev
cargo xtask ci pr
```

Developers select and review the next task in the active portable checklist.
`dev` runs formatting, compilation, lint, binary tests, doctests, and command smoke in their
maintained order, without building the documentation website. `ci pr` is the
non-mutating full pre-push gate, including doctests and documentation, and retains its
result under ignored `test-results/`; routine edits need no separate evidence
Markdown file. Mark the single portable checklist item complete after review.

Run `cargo xtask --help` for the authoritative command list. The root
[Development](../README.md#development) table provides the command-oriented
index. For command order, local defaults, and optional environment overrides,
see [developer command flow](workflow-optimization.md).

Use `cargo xtask test all` as the primary test command, including doctests.
Test binaries use Nextest with the execution profiles in `config/nextest.toml`. Focus a
run with `cargo xtask test security`, `test unit`, or another documented level.
`test unit` selects crate-local library/binary tests, excluding the cross-package
test suite and names containing explicit integration, system, conformance,
property, security, smoke, determinism, or fuzz categories. It is a focused
development check, not a replacement for `test all`. Inventory and execution use
the same selection; the explicit Cargo backend applies equivalent exclusions.
The runner/config/profile selection lives in `config/automation.toml`; a missing
runner produces installation guidance rather than silently skipping tests.

For project IR and public-view changes, run the focused boundaries first:

```sh
cargo test --package neutral-test-suite project_ir
cargo test --package neutral-test-suite project_identity
cargo xtask test all
cargo xtask fixtures check
cargo xtask ci pr
```

The standalone probe supports both document and complete project artifacts;
see its [commands and selection rules](../crates/neutral-probe/README.md).

## Documentation

Generate workspace Rustdoc and the searchable package landing page with:

```sh
cargo xtask docs
```

The command writes the complete generated website to `target/doc/`. This
repository does not track that output; it is published at the [Neutral API
documentation website](https://neutral-lang-doc.younesrabeh.workers.dev/).
Update Rust documentation, package metadata, or the templates under `xtask/templates/`,
then regenerate the local output.

The task-oriented Markdown guides in `docs/` are maintained source documents;
they are not overwritten by the generator.

For the complete path from an active portable plan through staged
implementation, conformance promotion, release qualification, and archival, see
the [development lifecycle](development-lifecycle.md).
