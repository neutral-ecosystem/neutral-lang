#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu
. "$(dirname "$0")/logging.sh"
neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"
neutral_repository_root=$(CDPATH='' cd "$(dirname "$0")/../.." && pwd)
cd "$neutral_repository_root"

neutral_release_action="${1:-prepare}"
if [ "$#" -gt 0 ]; then shift; fi
case "$neutral_release_action" in
    prepare) exec "$neutral_cargo_command" xtask release prepare "$@" ;;
    publish) exec "$neutral_cargo_command" xtask release publish "$@" ;;
    tag) exec "$neutral_cargo_command" xtask release tag "$@" ;;
    *) neutral_log_fail 'usage: scripts/linux/release.sh prepare [version] | publish | tag'; exit 2 ;;
esac
