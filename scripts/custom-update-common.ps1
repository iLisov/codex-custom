function Get-CustomSourceVersion {
    param([string]$ProjectRoot)
    $Text = Get-Content -LiteralPath (Join-Path $ProjectRoot 'codex-rs/Cargo.toml') -Raw -Encoding UTF8
    $Section = [regex]::Match($Text, '(?ms)^\[workspace\.package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
    $Match = [regex]::Match($Section, '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"')
    if (-not $Match.Success) { throw 'Cannot read the custom CLI base version.' }
    $Match.Groups[1].Value
}

function Get-CustomNewerRelease {
    param([string]$CurrentVersion, [object]$Release)
    if ($Release.draft -or $Release.prerelease) { return $null }
    $Match = [regex]::Match([string]$Release.tag_name, '^rust-v(\d+\.\d+\.\d+)$')
    if (-not $Match.Success) { return $null }
    $Version = $Match.Groups[1].Value
    if ([version]$Version -le [version]$CurrentVersion) { return $null }
    [pscustomobject]@{ Version = $Version; Tag = [string]$Release.tag_name }
}

function Get-CustomUpstreamRelease {
    Invoke-RestMethod -Uri 'https://api.github.com/repos/openai/codex/releases/latest' -Headers @{ 'User-Agent' = 'codex-custom-update-check' } -TimeoutSec 4
}
