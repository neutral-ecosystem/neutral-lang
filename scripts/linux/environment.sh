#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu

neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"
neutral_environment_action="${1:-verify}"

case "$neutral_environment_action" in
    verify|manifest) ;;
    *)
        printf '%s\n' "[error] usage: $0 [verify|manifest]" >&2
        exit 2
        ;;
esac

exec "$neutral_cargo_command" xtask environment "$neutral_environment_action"
