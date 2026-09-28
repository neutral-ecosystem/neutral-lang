#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu

neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"
neutral_release_action="${1:-prepare}"

if [ "$#" -gt 1 ]; then
    printf '%s\n' '[error] usage: scripts/linux/release.sh [tag|prepare|publish]' >&2
    exit 2
fi

neutral_repository_root=$(CDPATH='' cd "$(dirname "$0")/../.." && pwd)
cd "$neutral_repository_root"

case "$neutral_release_action" in
    tag)
        exec "$neutral_cargo_command" xtask release tag
        ;;
    prepare)
        printf '%s\n' '[info] qualifying and packaging the current main HEAD without publication'
        exec "$neutral_cargo_command" xtask release prepare
        ;;
    publish)
        ;;
    *)
        printf '%s\n' '[error] usage: scripts/linux/release.sh [tag|prepare|publish]' >&2
        exit 2
        ;;
esac

neutral_release_tag=$("$neutral_cargo_command" xtask release tag)
neutral_branch=$(git branch --show-current)
if [ "$neutral_branch" != main ]; then
    printf '%s\n' "[error] release publication requires checked-out main; found $neutral_branch" >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    printf '%s\n' '[error] release publication requires a clean worktree' >&2
    exit 1
fi
neutral_head=$(git rev-parse HEAD)
neutral_remote_main=$(git ls-remote --exit-code --heads origin main | awk 'NR == 1 { print $1 }')
if [ -z "$neutral_remote_main" ] || [ "$neutral_remote_main" != "$neutral_head" ]; then
    printf '%s\n' '[error] local main must equal origin/main before pushing the release tag' >&2
    exit 1
fi
if git ls-remote --exit-code --refs --tags origin "refs/tags/$neutral_release_tag" >/dev/null; then
    printf '%s\n' "[error] $neutral_release_tag already exists on origin; release tags are immutable" >&2
    exit 1
else
    neutral_lookup_status=$?
    if [ "$neutral_lookup_status" -ne 2 ]; then
        printf '%s\n' '[error] could not verify whether the release tag exists on origin' >&2
        exit 1
    fi
fi
if git show-ref --verify --quiet "refs/tags/$neutral_release_tag"; then
    neutral_tag_commit=$(git rev-list -n 1 "$neutral_release_tag")
    if [ "$neutral_tag_commit" != "$neutral_head" ]; then
        printf '%s\n' "[error] existing local $neutral_release_tag points to another commit" >&2
        exit 1
    fi
    git verify-tag "$neutral_release_tag"
fi

printf '%s\n' "[info] preparing $neutral_release_tag from main HEAD $neutral_head"
"$neutral_cargo_command" xtask version check
"$neutral_cargo_command" xtask release prepare

if ! git show-ref --verify --quiet "refs/tags/$neutral_release_tag"; then
    git tag -s "$neutral_release_tag" -m "neutral-lang $neutral_release_tag"
fi
git verify-tag "$neutral_release_tag"
git push origin "refs/tags/$neutral_release_tag:refs/tags/$neutral_release_tag"
printf '%s\n' "[info] pushed signed $neutral_release_tag; tag-triggered workflow will create a draft with verified assets"
