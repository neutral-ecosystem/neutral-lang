# SPDX-License-Identifier: Apache-2.0
# Delegates environment verification or manifest reporting to xtask.

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'logging.ps1')
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
$neutralEnvironmentAction = if ($args.Count -eq 0) { 'verify' } else { $args[0] }

if ($neutralEnvironmentAction -notin @('verify', 'manifest')) {
    Write-NeutralFailure 'usage: .\scripts\win\environment.ps1 [verify|manifest]'
    exit 2
}

& $neutralCargoCommand xtask environment $neutralEnvironmentAction
exit $LASTEXITCODE
