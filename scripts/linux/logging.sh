#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0
# Provides shared Linux failure logging with terminal-aware red status labels.

NEUTRAL_LOG_FAIL='FAIL'
NEUTRAL_LOG_RED='\033[31m'
NEUTRAL_LOG_RESET='\033[0m'

neutral_log_fail() {
    if [ -n "${NO_COLOR:-}" ] || [ "${CARGO_TERM_COLOR:-}" = never ]; then
        printf '%s %s\n' "$NEUTRAL_LOG_FAIL" "$*" >&2
    elif [ "${CARGO_TERM_COLOR:-}" = always ] || { [ -t 2 ] && [ "${TERM:-}" != dumb ]; }; then
        printf '%b%s%b %s\n' "$NEUTRAL_LOG_RED" "$NEUTRAL_LOG_FAIL" "$NEUTRAL_LOG_RESET" "$*" >&2
    else
        printf '%s %s\n' "$NEUTRAL_LOG_FAIL" "$*" >&2
    fi
}
