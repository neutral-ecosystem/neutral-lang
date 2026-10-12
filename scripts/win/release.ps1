# SPDX-License-Identifier: Apache-2.0
# Delegates release preparation, publication, or tagging to xtask from the repository root.

param(
    [string]$Action = 'prepare',
    [string]$Version
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'logging.ps1')
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
Set-Location (Join-Path $PSScriptRoot '../..')
$neutralReleaseArguments = @()
if ($Version) { $neutralReleaseArguments += $Version }
$neutralReleaseArguments += $args
switch ($Action) {
    'prepare' { & $neutralCargoCommand xtask release prepare @neutralReleaseArguments }
    'publish' { & $neutralCargoCommand xtask release publish @neutralReleaseArguments }
    'tag' { & $neutralCargoCommand xtask release tag @neutralReleaseArguments }
    default {
        Write-NeutralFailure 'usage: .\scripts\win\release.ps1 prepare [version] | publish | tag'
        exit 2
    }
}
exit $LASTEXITCODE
