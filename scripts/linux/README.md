<!-- SPDX-License-Identifier: Apache-2.0 -->

# Linux workflow adapter

This directory is the supported Linux entry layer for Neutral contributors and
release operators. It is owned by repository operations and has no authority
over language behavior or release policy.

| Script | Responsibility | Inputs | Outputs | Delegates to |
| --- | --- | --- | --- | --- |
| `bootstrap.sh` | Verify the Linux host, architecture, TLS/archive/checksum tools, Rust, and Cargo | Repository checkout and optional command overrides | Bootstrap environment evidence | `cargo xtask bootstrap` |
| `environment.sh` | Verify or print the resolved repository environment | Optional `verify` or `manifest` action | Terminal output only | `cargo xtask environment <action>` |
| `release.sh` | Test/prepare, then approve and atomically push main + signed tag | `prepare [version]` (default), `publish`, or `tag`; clean `main` | Local evidence/packages; publication triggers a draft release | `cargo xtask release prepare`, `cargo xtask release publish` |

Start a new checkout with:

```sh
./scripts/linux/bootstrap.sh
cargo xtask --help
cargo xtask quality
```

The bootstrap is intentionally sufficient for normal stable development and
does not install software. For a release/quality workstation, install Rustup's
nightly toolchain and LLVM component plus `cargo-llvm-cov`, `cargo-fuzz`,
`cargo-mutants`, Valgrind, Git, a POSIX shell, `tar`, TLS-enabled `curl`, and
`sha256sum`, then run:

```sh
cargo xtask environment verify
cargo xtask environment manifest
CARGO_NET_OFFLINE=true cargo xtask ci pr
```

`environment verify` prints an actionable installation command for each absent
tool. The stable toolchain remains selected by `rust-toolchain.toml`; use the
command-local `RUSTUP_TOOLCHAIN=nightly` prefix only for coverage and fuzzing.
The bootstrap manifest under `test-results/bootstrap/environment.json` records
the host image, kernel, stable Rust/Cargo, isolated nightly, and tool versions
without embedding the checkout's user-specific absolute path.

`release.sh prepare [version]` automates version metadata, tests, measurements,
evaluation, and packaging without tagging or pushing. `release.sh publish`
records approval, requalifies, signs, and pushes main plus exactly the derived
tag atomically. Remote main may be behind, but cannot diverge. The tag-triggered
workflow creates a draft release. See the [two-command release procedure](../../docs/release-and-versioning.md).
