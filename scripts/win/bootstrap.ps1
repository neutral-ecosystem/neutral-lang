# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = 'Stop'
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
$neutralRustcCommand = if ($env:NEUTRAL_RUSTC_COMMAND) { $env:NEUTRAL_RUSTC_COMMAND } else { 'rustc' }

if (-not (Get-Command Get-FileHash -ErrorAction SilentlyContinue)) {
    throw '[error] Get-FileHash is required for verified bootstrap downloads.'
}

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    throw '[error] tar is required for verified bootstrap archives.'
}

if (-not (Get-Command $neutralCargoCommand -ErrorAction SilentlyContinue)) {
    throw '[error] cargo is required; install the latest stable Rust, then rerun this script.'
}

if (-not (Get-Command $neutralRustcCommand -ErrorAction SilentlyContinue)) {
    throw '[error] rustc is required; install the latest stable Rust, then rerun this script.'
}

$actualToolchain = (& $neutralRustcCommand --version).Split(' ')[1]
if ($actualToolchain -match '-(nightly|beta|dev)') {
    throw "[error] Latest stable Rust is required; found $actualToolchain. This script never installs software automatically."
}

& $neutralCargoCommand xtask bootstrap
