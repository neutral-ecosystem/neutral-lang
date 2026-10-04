<!-- SPDX-License-Identifier: Apache-2.0 -->

# Troubleshooting

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

## Toolchain or host verification fails

Run `cargo xtask environment verify`. It reports the missing or incompatible
dependency and the expected installation command without modifying the system.

## Formatting fails

Run `cargo xtask fmt --write`, then repeat the original command.

## Nextest is missing

Run `cargo install cargo-nextest --locked`, then retry. For an explicit Cargo
compatibility run use `NEUTRAL_TEST_RUNNER=cargo cargo xtask test all`.
The repository never silently switches runners. Full quality gates run doctests
separately, since nextest executes test binaries rather than Rustdoc tests.

## Cargo discovery reports missing offline dependencies

Run `cargo fetch --locked`. Workspace discovery is read-only and does not update
the lockfile or download dependencies behind a check command.

## CI fails locally

Run the failed `cargo xtask` subcommand directly. Hosted CI delegates to the
same repository commands used during local development.

## Generated results appear stale

Inspect `test-results/`, then remove generated evidence when appropriate with
`cargo xtask clean`.

## API documentation is stale

Run `cargo xtask docs` and open `target/doc/index.html`. The generated tree is
ignored here; the published copy is the [Neutral API documentation
website](https://neutral-lang-doc.younesrabeh.workers.dev/).

## Release preparation fails

Confirm that `main` is checked out, `HEAD` is the intended release commit, the
worktree is clean, the workspace version is consistent, release quality
evaluation and approval are current, and distribution scope is valid. Then run
`cargo xtask release prepare` again.
