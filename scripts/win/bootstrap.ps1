# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = 'Stop'
$requiredToolchain = '1.97.1'
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
$neutralRustcCommand = if ($env:NEUTRAL_RUSTC_COMMAND) { $env:NEUTRAL_RUSTC_COMMAND } else { 'rustc' }

if (-not (Get-Command Get-FileHash -ErrorAction SilentlyContinue)) {
    throw '[error] Get-FileHash is required for verified bootstrap downloads.'
}

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    throw '[error] tar is required for verified bootstrap archives.'
}

if (-not (Get-Command $neutralCargoCommand -ErrorAction SilentlyContinue)) {
    throw '[error] cargo is required; install Rust 1.97.1 explicitly, then rerun this script.'
}

if (-not (Get-Command $neutralRustcCommand -ErrorAction SilentlyContinue)) {
    throw '[error] rustc is required; install Rust 1.97.1 explicitly, then rerun this script.'
}

$actualToolchain = (& $neutralRustcCommand --version).Split(' ')[1]
if ($actualToolchain -ne $requiredToolchain) {
    throw "[error] Rust $requiredToolchain is required; found $actualToolchain. This script never installs software automatically."
}

& $neutralCargoCommand xtask bootstrap
