<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development workflow

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Normal development uses the repository-selected latest stable Rust toolchain.
The `cargo xtask` interface owns the order and policy used locally and in CI.

## First setup

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
```

This runs the normal formatting, compilation, lint, test, and documentation
checks in their maintained order. Before pushing to `main`, run the same
non-mutating composition as hosted CI:

```sh
cargo xtask ci pr
```

Run `cargo xtask --help` for the authoritative command list. The root
[Development](../README.md#development) table provides the command-oriented
index.

## Documentation

Generate workspace Rustdoc and the searchable package landing page with:

```sh
cargo xtask docs
```

The command writes the complete generated website to `target/doc/`. This
repository does not track that output; it is published at the [Neutral API
documentation website](https://neutral-lang-doc.younesrabeh.workers.dev/).
Update Rust documentation, package metadata, or the templates under `xtask/src/`,
then regenerate the local output.

The task-oriented Markdown guides in `docs/` are maintained source documents;
they are not overwritten by the generator.

For the complete path from an active portable plan through staged
implementation, conformance promotion, release qualification, and archival, see
the [development lifecycle](development-lifecycle.md).
