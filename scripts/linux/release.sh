#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu

neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"

printf '%s\n' '[info] starting fail-closed local release preparation; no publication is performed'
exec "$neutral_cargo_command" xtask release prepare
