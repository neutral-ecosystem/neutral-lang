# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = 'Stop'
$requiredToolchain = '1.97.1'

if (-not (Get-Command Get-FileHash -ErrorAction SilentlyContinue)) {
    throw '[error] Get-FileHash is required for verified bootstrap downloads.'
}

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    throw '[error] tar is required for verified bootstrap archives.'
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw '[error] cargo is required; install Rust 1.97.1 explicitly, then rerun this script.'
}

if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    throw '[error] rustc is required; install Rust 1.97.1 explicitly, then rerun this script.'
}

$actualToolchain = (rustc --version).Split(' ')[1]
if ($actualToolchain -ne $requiredToolchain) {
    throw "[error] Rust $requiredToolchain is required; found $actualToolchain. This script never installs software automatically."
}

cargo xtask bootstrap
