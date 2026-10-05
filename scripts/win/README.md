<!-- SPDX-License-Identifier: Apache-2.0 -->

# Windows workflow adapter

This directory is the supported Windows PowerShell entry layer for Neutral
contributors and release operators. It is owned by repository operations and
has no authority over language behavior or release policy.

| Script | Responsibility | Inputs | Outputs | Delegates to |
| --- | --- | --- | --- | --- |
| `bootstrap.ps1` | Verify PowerShell archive/checksum tools, Rust, and Cargo | Repository checkout and optional command overrides | Bootstrap environment evidence | `cargo xtask bootstrap` |
| `environment.ps1` | Verify or print the resolved repository environment | Optional `verify` or `manifest` action | Terminal output only | `cargo xtask environment <action>` |
| `release.ps1` | Test/prepare, then approve and atomically push main + signed tag | `prepare [version]` or `publish`; clean `main` | Local package/evidence; publication triggers a draft release | `cargo xtask release prepare`, `cargo xtask release publish` |

Start a new checkout with:

```powershell
.\scripts\win\bootstrap.ps1
cargo xtask --help
cargo xtask quality
```

`release.ps1` never tags, pushes, uploads, or publishes. Missing approval,
candidate identity, evidence, tools, or a clean checkout causes `xtask` to fail.
