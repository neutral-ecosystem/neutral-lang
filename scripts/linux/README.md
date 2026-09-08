<!-- SPDX-License-Identifier: Apache-2.0 -->

# Linux workflow adapter

This directory is the supported Linux entry layer for Neutral contributors and
release operators. It is owned by repository operations and has no authority
over language behavior or release policy.

| Script | Responsibility | Inputs | Outputs | Delegates to |
| --- | --- | --- | --- | --- |
| `bootstrap.sh` | Verify the Linux host, architecture, TLS/archive/checksum tools, Rust, and Cargo | Repository checkout and optional command overrides | Bootstrap environment evidence | `cargo xtask bootstrap` |
| `environment.sh` | Verify or print the resolved repository environment | Optional `verify` or `manifest` action | Terminal output only | `cargo xtask environment <action>` |
| `release.sh` | Enter the fail-closed release-preparation workflow | Approved candidate configuration and clean candidate checkout | Ignored package/release evidence | `cargo xtask release prepare` |

Start a new checkout with:

```sh
./scripts/linux/bootstrap.sh
cargo xtask --help
cargo xtask quality
```

`release.sh` never tags, pushes, uploads, or publishes. Missing approval,
candidate identity, evidence, tools, or a clean checkout causes `xtask` to fail.
