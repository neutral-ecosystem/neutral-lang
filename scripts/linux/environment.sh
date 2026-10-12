#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu
. "$(dirname "$0")/logging.sh"

neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"
neutral_environment_action="${1:-verify}"

case "$neutral_environment_action" in
    verify|manifest) ;;
    *)
        neutral_log_fail "usage: $0 [verify|manifest]"
        exit 2
        ;;
esac

exec "$neutral_cargo_command" xtask environment "$neutral_environment_action"
