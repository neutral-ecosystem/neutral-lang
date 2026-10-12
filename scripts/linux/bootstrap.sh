#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0
# Checks Linux bootstrap prerequisites and delegates environment setup to xtask.

set -eu
. "$(dirname "$0")/logging.sh"

neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"
neutral_rustc_command="${NEUTRAL_RUSTC_COMMAND:-rustc}"

host_os="$(uname -s)"
host_architecture="$(uname -m)"
case "$host_os" in
    Linux) ;;
    *)
        neutral_log_fail "unsupported bootstrap host: $host_os"
        exit 1
        ;;
esac
case "$host_architecture" in
    x86_64) ;;
    *)
        neutral_log_fail "unsupported bootstrap architecture: $host_architecture"
        exit 1
        ;;
esac

command -v sh >/dev/null
command -v curl >/dev/null || {
    neutral_log_fail 'curl with TLS and system certificates is required for verified bootstrap downloads.'
    exit 1
}
command -v tar >/dev/null || {
    neutral_log_fail 'tar is required for verified bootstrap archives.'
    exit 1
}
command -v "$neutral_cargo_command" >/dev/null || {
    neutral_log_fail 'cargo is required; install the latest stable Rust, then rerun this script.'
    exit 1
}
command -v "$neutral_rustc_command" >/dev/null || {
    neutral_log_fail 'rustc is required; install the latest stable Rust, then rerun this script.'
    exit 1
}
command -v sha256sum >/dev/null || {
    neutral_log_fail 'sha256sum is required for verified bootstrap downloads.'
    exit 1
}

curl_capabilities="$(curl --version)"
case "$curl_capabilities" in
    *Protocols:*https*) ;;
    *)
        neutral_log_fail 'curl must support HTTPS.'
        exit 1
        ;;
esac

actual_toolchain="$("$neutral_rustc_command" --version)"
case "$actual_toolchain" in
    *-nightly*|*-beta*|*-dev*)
        neutral_log_fail "latest stable Rust is required; found $actual_toolchain."
        neutral_log_fail 'install or select the stable toolchain; this script never installs software automatically.'
        exit 1
        ;;
esac

"$neutral_cargo_command" xtask bootstrap
