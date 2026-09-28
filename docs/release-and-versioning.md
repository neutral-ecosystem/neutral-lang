<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release and versioning

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

The root `Cargo.toml` workspace version is authoritative. Every package inherits
that value, and the release tag is derived as `v<workspace-version>`. A release
is deliberately a sequence of small, reviewable changes: first the version,
then its quality approval, then the locally assembled distribution, and finally
an explicit signed Git tag.

Language and artifact contract versions are separate from the package-release
version. Do not change frozen contract versions merely because the package moves
from (for example) `0.2.0` to `0.3.0`.

## Before starting

Release preparation must begin with the intended completed implementation on
`main`. If the work was developed on `dev`, fast-forward `main` only after its
normal review and CI have passed:

```sh
git checkout main
git merge --ff-only dev
git push origin main
```

Confirm that `main` is clean and that the current implementation passes the
ordinary integration gate:

```sh
git status --short --branch
cargo xtask version check
cargo xtask ci pr
```

`git status` must report no modified or untracked tracked files before a release
command that requires a clean worktree can run.

## Version commands

```sh
cargo xtask version show
cargo xtask version check
cargo xtask version prepare <version>
```

`cargo xtask version show` displays the package release version beside the
independent frozen contract versions. `cargo xtask version check` verifies that
every package and lockfile entry inherits the one workspace package version.

`cargo xtask version prepare <version>` accepts a plain SemVer value—such as
`0.3.0`, not `v0.3.0`. It requires a clean worktree and automatically updates:

- root `Cargo.toml` (`workspace.package.version`);
- matching workspace-package records in `Cargo.lock`; and
- `quality/evidence/v<version>/README.md`, the durable approval-evidence
  scaffold.

It does not tag, publish, upload, or approve a release. It changes tracked
files, so those changes need their own reviewable commit before any quality
evaluation.

## Full promotion procedure

The following example promotes the completed Stage 2 work as `v0.3.0`. Replace
`0.3.0` only with the approved next package version.

### 1. Prepare and commit the version

```sh
cargo xtask version prepare 0.3.0
cargo xtask version check
git add Cargo.toml Cargo.lock quality/evidence/v0.3.0
git tag -s v0.3.0 -m "[REL] release v0.3.0"
git verify-tag v0.3.0
git push origin v0.3.0

git push origin HEAD
git push origin main
```

This first commit is the exact source candidate evaluated by the release-quality
gate. Do not manually edit individual crate manifests: they inherit the workspace
version.

### 2. Evaluate the committed release candidate

```sh
cargo xtask quality evaluate --profile release
```

This runs the full release-quality composition on clean `main` and retains a
commit-bound, generated evaluation under `test-results/quality/evaluations/`.
It does not alter tracked project files. If it fails, fix the cause, commit the
fix, and run the evaluation again for the new `HEAD`.

### 3. Record the human release approval

```sh
cargo xtask quality approve --release 0.3.0
git add quality/evidence/v0.3.0 quality/STATUS.md
git commit -m "[REL] approve v0.3.0 quality"
git push origin main
```

Approval checks that the supplied version equals `workspace.package.version` and
that a passing release evaluation exists for the candidate commit. It then writes
the immutable `record.toml` approval record and regenerates `quality/STATUS.md`.
Those tracked evidence changes must be committed so the repository, rather than a
local machine, retains the decision.

### 4. Assemble and validate release files

## Release qualification

```sh
cargo xtask release prepare
```

Release preparation validates branch, `HEAD`, worktree cleanliness, version
consistency, quality evidence, approval, and distribution scope. It assembles
candidate binaries, checksums, manifests, and supporting evidence beneath
`test-results/release/`.

Inspect the generated package before publication. The local command does not
push commits, create tags, upload artifacts, publish packages, or create a
GitHub release.

### 5. Create the source tag and publish

After reviewing the assembled files, create the signed tag from the clean,
approved `main` commit and push it:

```sh
git tag -s v0.3.0 -m "Neutral v0.3.0"
git push origin v0.3.0
```

The tag-triggered GitHub release workflow is the only publication step. It can
build and attach the selected binary assets, but it must not replace the local
approval, quality evaluation, or release preparation above.

## Recovery and common mistakes

- **`invalid package SemVer: v0.3.0`** — pass `0.3.0` to `version prepare` and
  `quality approve`; only the Git tag has the `v` prefix.
- **`quality evaluation requires a clean worktree`** — commit or intentionally
  discard unrelated work, then rerun the evaluation for the new `HEAD`.
- **`release evidence directory is not prepared`** — run and commit `cargo
  xtask version prepare <version>` before approving quality.
- **A new implementation, dependency, contract, or release-configuration change
  after evaluation** — rerun release evaluation. Evaluations are bound to one
  exact source candidate. The approval-evidence commit produced immediately by
  a valid `quality approve` command is the documented exception.
- **`release prepare` rejects the branch or worktree** — check out `main`, push
  the relevant commits, and ensure `git status --short` has no output.
