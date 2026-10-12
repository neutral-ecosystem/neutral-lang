# SPDX-License-Identifier: Apache-2.0
# Provides shared Windows failure logging with terminal-aware red status labels.

$neutralLogFail = 'FAIL'
$neutralLogRed = "$([char]27)[31m"
$neutralLogReset = "$([char]27)[0m"

# Writes a failure to stderr using the same color policy as the Linux adapter.
function Write-NeutralFailure {
    param([string]$Message)

    $useColor = -not $env:NO_COLOR -and (
        $env:CARGO_TERM_COLOR -eq 'always' -or (
            $env:CARGO_TERM_COLOR -ne 'never' -and
            -not [Console]::IsErrorRedirected -and
            $env:TERM -ne 'dumb' -and
            $Host.UI.SupportsVirtualTerminal
        )
    )
    if ($useColor) {
        [Console]::Error.WriteLine("$neutralLogRed$neutralLogFail$neutralLogReset $Message")
    } else {
        [Console]::Error.WriteLine("$neutralLogFail $Message")
    }
}
