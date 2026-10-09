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
    $PreviousProtocol = [Net.ServicePointManager]::SecurityProtocol
    try {
        [Net.ServicePointManager]::SecurityProtocol = $PreviousProtocol -bor [Net.SecurityProtocolType]::Tls12
        $Response = Invoke-WebRequest -Uri 'https://github.com/openai/codex/releases/latest' -Method Head -UseBasicParsing -Headers @{ 'User-Agent' = 'codex-custom-update-check' } -TimeoutSec 4
    } finally {
        [Net.ServicePointManager]::SecurityProtocol = $PreviousProtocol
    }
    $ReleaseUri = $Response.BaseResponse.ResponseUri
    if (-not $ReleaseUri) { $ReleaseUri = $Response.BaseResponse.RequestMessage.RequestUri }
    $Match = [regex]::Match([string]$ReleaseUri.AbsolutePath, '^/openai/codex/releases/tag/(rust-v\d+\.\d+\.\d+)$')
    if (-not $Match.Success) { throw 'Cannot determine the latest stable Codex release from the GitHub redirect.' }
    [pscustomobject]@{ tag_name = $Match.Groups[1].Value; draft = $false; prerelease = $false }
}
