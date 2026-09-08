<!-- SPDX-License-Identifier: Apache-2.0 -->

# Platform adapters

This directory owns the smallest host-specific layer in Neutral's developer and
release workflow. The Rust `xtask` package owns repository, quality, packaging,
and release policy; these scripts only verify platform prerequisites and invoke
that platform-neutral interface.

| Directory | Supported host | Responsibility | Delegated command |
| --- | --- | --- | --- |
| [`linux/`](linux/README.md) | Linux | POSIX host checks and shell entry points | `cargo xtask ...` |
| [`win/`](win/README.md) | Windows PowerShell | Windows host checks and PowerShell entry points | `cargo xtask ...` |

Inputs are the checked-out repository, the selected Rust toolchain, and optional
`NEUTRAL_CARGO_COMMAND` / `NEUTRAL_RUSTC_COMMAND` executable overrides. Outputs
are only the ignored files created by `xtask` beneath `test-results/` and normal
Cargo build output beneath `target/`.

The scripts must never duplicate compiler behavior, test selection, quality
thresholds, package contents, release scope, version rules, or publication
policy. When that policy changes, update `xtask` and keep these adapters thin.
