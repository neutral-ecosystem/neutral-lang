# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = 'Stop'
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }
$neutralEnvironmentAction = if ($args.Count -eq 0) { 'verify' } else { $args[0] }

if ($neutralEnvironmentAction -notin @('verify', 'manifest')) {
    throw '[error] usage: .\scripts\win\environment.ps1 [verify|manifest]'
}

& $neutralCargoCommand xtask environment $neutralEnvironmentAction
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
