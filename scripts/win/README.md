<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Describes the Windows scripts, prerequisites, logging, and development and release commands. -->

# Windows workflow adapter

This directory is the supported Windows PowerShell entry layer for Neutral
contributors and release operators. It is owned by repository operations and
has no authority over language behavior or release policy.

Failures use the shared `logging.ps1` helper and a red `FAIL` label on supported
terminals. `NO_COLOR`, `CARGO_TERM_COLOR=never`, redirected output, and dumb
terminals keep it plain; `CARGO_TERM_COLOR=always` forces color unless `NO_COLOR`
is set. Successful output and workflow progress come from the delegated
`cargo xtask` command using its standard status labels.

| Script | Responsibility | Inputs | Outputs | Delegates to |
| --- | --- | --- | --- | --- |
| `bootstrap.ps1` | Verify the Windows host, architecture, archive/checksum tools, Rust, and Cargo | Repository checkout and optional command overrides | Bootstrap environment evidence | `cargo xtask bootstrap` |
| `environment.ps1` | Verify or print the resolved repository environment | Optional `verify` or `manifest` action | Terminal output only | `cargo xtask environment <action>` |
| `release.ps1` | Test/prepare, then approve and atomically push main + signed tag | `prepare [version]` (default), `publish`, or `tag`; clean `main` | Local evidence/packages; publication triggers a draft release | `cargo xtask release prepare`, `cargo xtask release publish` |

Start a new checkout with:

```powershell
.\scripts\win\bootstrap.ps1
cargo xtask --help
cargo xtask quality
```

The bootstrap is sufficient for normal stable development and does not install
software. Both command overrides, `NEUTRAL_CARGO_COMMAND` and
`NEUTRAL_RUSTC_COMMAND`, work the same way as in the Linux scripts. Native command
exit codes are preserved; usage failures exit with code 2 and bootstrap
prerequisite failures with code 1.

For a release/quality workstation, install the tools reported by
`cargo xtask environment verify`, then run:

```powershell
cargo xtask environment verify
cargo xtask environment manifest
cargo xtask ci pr
```

The stable toolchain remains selected by `rust-toolchain.toml`. Coverage and
fuzzing require the nightly toolchain; platform-specific profiling tools require
a supported runner, as reported by environment verification.

`release.ps1 prepare [version]` automates version metadata, tests, measurements,
evaluation, and packaging. `release.ps1 publish` records approval, requalifies,
signs, and pushes main plus exactly the derived tag atomically. Remote main may
be behind, but cannot diverge. The tag-triggered workflow creates a draft release.
See the [two-command release procedure](../../docs/release-and-versioning.md).
