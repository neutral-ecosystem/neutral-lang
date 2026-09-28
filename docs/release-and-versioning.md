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

`cargo xtask version prepare <version>` accepts plain SemVer, without the `v`
prefix. It requires a clean worktree and automatically updates:

- root `Cargo.toml` (`workspace.package.version`);
- matching workspace-package records in `Cargo.lock`; and
- `quality/evidence/v<version>/README.md`, the durable approval-evidence
  scaffold.

It does not tag, publish, upload, or approve a release. It changes tracked
files, so those changes need their own reviewable commit before any quality
evaluation.

## Full promotion procedure

Use the next approved package version for `<version>` below. Once prepared, the
root `Cargo.toml` is authoritative; neither the shell script nor the workflow
hardcodes a release version. `cargo xtask release tag` prints the corresponding
`v<version>` directly from the workspace version and `config/release.toml`.

### 1. Prepare and commit the version

```sh
cargo xtask version prepare <version>
cargo xtask version check
git add Cargo.toml Cargo.lock quality/evidence/v<version>
git commit -m "[REL] prepare <version>"
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
cargo xtask quality approve --release <version>
git add quality/evidence/v<version> quality/STATUS.md
git commit -m "[REL] approve <version> quality"
git push origin main
```

Approval checks that the supplied version equals `workspace.package.version` and
that a passing release evaluation exists for the candidate commit. It then writes
the immutable `record.toml` approval record and regenerates `quality/STATUS.md`.
Those tracked evidence changes must be committed so the repository, rather than a
local machine, retains the decision.

### 4. Assemble and validate release files

```sh
scripts/linux/release.sh tag
scripts/linux/release.sh prepare
```

Release preparation validates branch, `HEAD`, worktree cleanliness, version
consistency, quality evidence, approval, and distribution scope. It assembles
candidate binaries, checksums, manifests, and supporting evidence beneath
`test-results/release/`.

Inspect the generated package before publication. `prepare` does not push
commits, create tags, upload artifacts, or create a GitHub release. The exact
package directory is
`test-results/release/package/<tag>/<main-commit>/<host-target>/`.

### 5. Create the source tag and publish

After reviewing the assembled files, run the explicit publication action from
clean `main` whose `HEAD` has already been pushed to `origin/main`:

```sh
scripts/linux/release.sh publish
```

The script derives the tag from the release plan, re-runs release qualification,
creates and verifies a signed tag only if it does not already exist, and pushes
that tag without force. It refuses to move an existing local or remote tag.
The tag-triggered GitHub workflow checks out that exact tag commit, confirms it
is still `main` HEAD, rebuilds and verifies the selected package, and creates
a draft GitHub Release with the title `neutral-lang <tag>`. Review the draft
and its assets before any separate publication decision. A manual
workflow dispatch qualifies current `main` without publishing. Neither path
replaces the human approval step.

## Recovery and common mistakes

- **`invalid package SemVer: v<version>`** — pass plain `<version>` to
  `version prepare` and `quality approve`; only the Git tag has the `v` prefix.
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
- **The derived tag already exists** — release tags are immutable. Do not
  retarget or force-push it. Check whether the corresponding release already
  contains the expected assets; use a new approved workspace version for a
  new source candidate.
