param([Parameter(Mandatory = $true)][string]$CurrentVersion, [Parameter(Mandatory = $true)][string]$ProjectRoot)
$ErrorActionPreference = 'Stop'
if ($env:CODEX_CUSTOM_SKIP_VERSION_CHECK -eq '1') { exit 0 }
. (Join-Path $PSScriptRoot 'custom-update-common.ps1')
try {
    $StateDirectory = Join-Path $ProjectRoot 'codex-rs/target/custom-update'
    $StateFile = Join-Path $StateDirectory 'upstream.json'
    $Release = $null
    if (Test-Path -LiteralPath $StateFile) {
        try {
            $Cached = Get-Content -LiteralPath $StateFile -Raw -Encoding UTF8 | ConvertFrom-Json
            if ([datetime]$Cached.checked_at -gt [datetime]::UtcNow.AddMinutes(-10)) { $Release = $Cached.release }
        } catch {}
    }
    if (-not $Release) {
        $Release = Get-CustomUpstreamRelease
        New-Item -ItemType Directory -Path $StateDirectory -Force | Out-Null
        $State = @{ checked_at = [datetime]::UtcNow.ToString('o'); release = @{ tag_name = $Release.tag_name; draft = [bool]$Release.draft; prerelease = [bool]$Release.prerelease } } | ConvertTo-Json -Depth 4
        [IO.File]::WriteAllText($StateFile, $State, [Text.UTF8Encoding]::new($false))
    }
    # The TUI reads this cache and owns rendering the update notice.
    # Printing here would be cleared when the terminal enters fullscreen mode.
} catch {
    # A failed check must not prevent the user's CLI session from starting.
}
exit 0
