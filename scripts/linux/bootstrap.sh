#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0

set -eu

required_toolchain='1.97.1'

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
command -v cargo >/dev/null || {
    printf '%s\n' '[error] cargo is required; install Rust 1.97.1 explicitly, then rerun this script.' >&2
    exit 1
}
command -v rustc >/dev/null || {
    printf '%s\n' '[error] rustc is required; install Rust 1.97.1 explicitly, then rerun this script.' >&2
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

actual_toolchain="$(rustc --version | awk '{print $2}')"
if [ "$actual_toolchain" != "$required_toolchain" ]; then
    printf '%s\n' "[error] Rust $required_toolchain is required; found $actual_toolchain." >&2
    printf '%s\n' '[error] Install the pinned toolchain explicitly; this script never installs software automatically.' >&2
    exit 1
fi

cargo xtask bootstrap
