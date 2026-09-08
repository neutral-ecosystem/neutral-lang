# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = 'Stop'
$neutralCargoCommand = if ($env:NEUTRAL_CARGO_COMMAND) { $env:NEUTRAL_CARGO_COMMAND } else { 'cargo' }

Write-Output '[info] starting fail-closed local release preparation; no publication is performed'
& $neutralCargoCommand xtask release prepare
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
