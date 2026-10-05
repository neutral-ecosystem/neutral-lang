<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release and versioning

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Release in two commands, from clean `main` with your implementation committed:

```sh
cargo xtask release prepare <version>
cargo xtask release publish
```

Use plain SemVer for `<version>`, without a `v` prefix. If the root
`Cargo.toml` already contains the intended version, omit the argument:

```sh
cargo xtask release prepare
cargo xtask release publish
```

No separate setup, approval, package, tag, or `git push` command is needed for
each release. Install the [analysis tools](quality-and-analysis.md#release-measurements)
once; preparation does not install software or weaken checks when a tool is missing.

## 1. Test and prepare

`cargo xtask release prepare [version]` performs the local work:

- Checks clean `main` and rejects an already existing local release tag.
- When a new version is supplied, updates the workspace version, inherited
  lockfile entries, and compact evidence scaffold; commits only those files
  with the message `[REL] v<version>`.
- Runs the ordinary quality gate, then coverage, mutation, fuzzing, performance,
  soak, and advisory checks. Valid evidence for the exact current inputs is
  reused; missing or stale measurements are run automatically.
- Selects nightly only for coverage/fuzz child commands, leaving the global
  stable toolchain unchanged.
- Records a passing evaluation for the exact candidate and assembles binaries,
  source archive, manifests, and checksums under ignored `test-results/release/`.

Preparation may take time when the expensive measurements need to run.
Watch the labeled progress and inspect the package path printed at completion.
It creates no approval, tag, remote push, or GitHub release.

Review the package before continuing. Do not edit source or release policy
between the two commands; if you do, commit the change and rerun preparation.

## 2. Publish to a draft

`cargo xtask release publish` is the explicit approval decision. It:

- Checks that remote `main` is an ancestor of local `main`, and the derived
  release tag does not already exist remotely.
- Records approval of the passing candidate, retains raw evidence locally, and
  commits only the compact approval record/status with `[REL] v<version>`.
- Requalifies and assembles artifacts for the final release commit.
- Creates and verifies a signed `v<workspace-version>` tag at that commit.
- Pushes `main` and **only that release tag** in one atomic, non-force push.

You need your usual Git remote credentials and a working Git tag-signing setup.
Local `main` does not need to be pushed separately first. If either ref is
rejected, the atomic push changes neither remote ref; there is no fallback to
separate pushes. This uses [Git's atomic push guarantee](https://git-scm.com/docs/git-push).

The tag-triggered workflow rechecks that exact source, regenerates missing
measurements on its runner, builds and verifies the assets, and creates a
**draft** GitHub release titled `neutral-lang v<version>`. It does not publish
a public release automatically. Review the draft and its assets separately;
[GitHub recommends assembling release assets while the release is a draft](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases).

## Version authority and advanced commands

The root `Cargo.toml` workspace package version is authoritative; all packages
inherit it. The tag is derived from that value, not supplied again to publication.
Language and artifact contract versions are independent: a package release must
not change frozen contracts merely to match its version.

These lower-level commands remain available for inspection or troubleshooting,
not as extra required release steps:

| Need | Command |
| --- | --- |
| Show package and frozen contract versions | `cargo xtask version show` |
| Verify inherited versions | `cargo xtask version check` |
| Print the derived tag | `cargo xtask release tag` |
| Change version metadata without committing/testing | `cargo xtask version prepare <version>` |
| Requalify an already approved release without changing Git refs | `cargo xtask release qualify` |

Linux `scripts/linux/release.sh prepare [version]` / `publish` and Windows
`scripts/win/release.ps1 prepare [version]` / `publish` are optional adapters
for the same commands. CI uses `release qualify`: it never records human approval,
creates local release commits, or pushes source refs.

## If something fails

- **Dirty worktree or wrong branch:** commit/review implementation changes and
  check out `main`. Release commands never stage arbitrary implementation files.
- **A test, measurement, or tool fails:** fix the cause, commit the fix if needed,
  and rerun `release prepare`. Valid unchanged measurements are reused.
- **Signing or pushing fails:** local release commits and any signed tag are
  preserved. Retry `release publish` on the same clean source; do not move the tag.
- **Remote main advanced:** integrate it, then rerun preparation. Publication
  will not force-push or rewrite history.
- **Tag already exists remotely:** inspect the GitHub draft/workflow. Never move
  a released tag; select a new workspace version for different source.
- **Raw evidence is missing on a fresh machine:** preparation/qualification
  regenerates it using the installed tools. Do not add reports to Git; they may
  contain personal paths. Only compact approval metadata is tracked.
