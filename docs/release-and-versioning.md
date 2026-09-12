<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release and versioning

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

The root `Cargo.toml` workspace version is authoritative. Release qualification
uses a clean checked-out `main` `HEAD`, and the publication tag is derived as
`v<workspace-version>`.

## Version commands

```sh
cargo xtask version show
cargo xtask version check
cargo xtask version prepare <version>
```

Version preparation propagates the workspace version and associated metadata so
they do not need to be maintained manually across packages.

## Release qualification

```sh
cargo xtask quality evaluate --profile release
cargo xtask quality approve --release <version>
cargo xtask release prepare
```

Release preparation validates branch, `HEAD`, worktree cleanliness, version
consistency, quality evidence, approval, and distribution scope. It assembles
candidate binaries, checksums, manifests, and supporting evidence beneath
`test-results/release/`.

The local command does not push commits, create remote tags, upload artifacts,
publish packages, or create releases. GitHub publication remains an explicit
tag-triggered workflow after the source state is approved.
