# SPDX-License-Identifier: Apache-2.0

param(
    [ValidateSet('prepare', 'publish', 'tag')]
    [string]$Action = 'prepare',
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
Set-Location (Join-Path $PSScriptRoot '../..')
if ($Action -eq 'prepare') {
    if ($Version) { & $neutralCargoCommand xtask release prepare $Version }
    else { & $neutralCargoCommand xtask release prepare }
} else {
    if ($Version) { throw '[error] version is only accepted with prepare' }
    & $neutralCargoCommand xtask release $Action
}
exit $LASTEXITCODE
