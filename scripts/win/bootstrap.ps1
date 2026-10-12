# SPDX-License-Identifier: Apache-2.0
# Checks Windows bootstrap prerequisites and delegates environment setup to xtask.

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'logging.ps1')
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
$neutralRustcCommand = if ($env:NEUTRAL_RUSTC_COMMAND) { $env:NEUTRAL_RUSTC_COMMAND } else { 'rustc' }

if ($env:OS -ne 'Windows_NT') {
    Write-NeutralFailure "unsupported bootstrap host: $env:OS"
    exit 1
}
$neutralHostArchitecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
if ($neutralHostArchitecture -ne 'AMD64') {
    Write-NeutralFailure "unsupported bootstrap architecture: $neutralHostArchitecture"
    exit 1
}

if (-not (Get-Command Get-FileHash -ErrorAction SilentlyContinue)) {
    Write-NeutralFailure 'Get-FileHash is required for verified bootstrap downloads.'
    exit 1
}

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    Write-NeutralFailure 'tar is required for verified bootstrap archives.'
    exit 1
}

if (-not (Get-Command $neutralCargoCommand -ErrorAction SilentlyContinue)) {
    Write-NeutralFailure 'cargo is required; install the latest stable Rust, then rerun this script.'
    exit 1
}

if (-not (Get-Command $neutralRustcCommand -ErrorAction SilentlyContinue)) {
    Write-NeutralFailure 'rustc is required; install the latest stable Rust, then rerun this script.'
    exit 1
}

$actualToolchain = & $neutralRustcCommand --version
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
if ($actualToolchain -match '-(nightly|beta|dev)') {
    Write-NeutralFailure "latest stable Rust is required; found $actualToolchain."
    Write-NeutralFailure 'install or select the stable toolchain; this script never installs software automatically.'
    exit 1
}

& $neutralCargoCommand xtask bootstrap
exit $LASTEXITCODE
