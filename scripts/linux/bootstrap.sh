#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu

neutral_cargo_command="${NEUTRAL_CARGO_COMMAND:-cargo}"
neutral_rustc_command="${NEUTRAL_RUSTC_COMMAND:-rustc}"

host_os="$(uname -s)"
host_architecture="$(uname -m)"
case "$host_os" in
    Linux|Darwin) ;;
    *)
        printf '%s\n' "[error] unsupported bootstrap host: $host_os" >&2
        exit 1
        ;;
esac
case "$host_architecture" in
    x86_64|aarch64|arm64) ;;
    *)
        printf '%s\n' "[error] unsupported bootstrap architecture: $host_architecture" >&2
        exit 1
        ;;
esac

command -v sh >/dev/null
command -v curl >/dev/null || {
    printf '%s\n' '[error] curl with TLS and system certificates is required for verified bootstrap downloads.' >&2
    exit 1
}
command -v tar >/dev/null || {
    printf '%s\n' '[error] tar is required for verified bootstrap archives.' >&2
    exit 1
}
command -v "$neutral_cargo_command" >/dev/null || {
    printf '%s\n' '[error] cargo is required; install the latest stable Rust, then rerun this script.' >&2
    exit 1
}
command -v "$neutral_rustc_command" >/dev/null || {
    printf '%s\n' '[error] rustc is required; install the latest stable Rust, then rerun this script.' >&2
    exit 1
}
command -v sha256sum >/dev/null || {
    printf '%s\n' '[error] sha256sum is required for verified bootstrap downloads.' >&2
    exit 1
}

curl --version | grep -q 'Protocols:.*https' || {
    printf '%s\n' '[error] curl must support HTTPS.' >&2
    exit 1
}

actual_toolchain="$("$neutral_rustc_command" --version | awk '{print $2}')"
case "$actual_toolchain" in
    *-nightly*|*-beta*|*-dev*)
        printf '%s\n' "[error] Latest stable Rust is required; found $actual_toolchain." >&2
        printf '%s\n' '[error] Install or select the stable toolchain; this script never installs software automatically.' >&2
        exit 1
        ;;
esac

"$neutral_cargo_command" xtask bootstrap
