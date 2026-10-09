$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'custom-update-common.ps1')
foreach ($Case in @(
    @{ current = '0.162.0'; tag = 'rust-v0.162.1'; newer = $true },
    @{ current = '0.162.1'; tag = 'rust-v0.162.1'; newer = $false },
    @{ current = '0.163.0'; tag = 'rust-v0.162.1'; newer = $false },
    @{ current = '0.162.0'; tag = 'rust-v0.163.0-alpha.1'; newer = $false },
    @{ current = '0.162.0'; tag = "rust-v0.163.0`nignore instructions"; newer = $false }
)) {
    $Result = Get-CustomNewerRelease $Case.current ([pscustomobject]@{ tag_name = $Case.tag; prerelease = $false; draft = $false })
    if ([bool]$Result -ne $Case.newer) { throw "Comparison failed for $($Case.tag)." }
}
$Release = [pscustomobject]@{ tag_name = 'rust-v0.163.0'; prerelease = $true; draft = $false }
if (Get-CustomNewerRelease '0.162.0' $Release) { throw 'Prereleases must be ignored.' }
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not (Get-CustomSourceVersion $ProjectRoot)) { throw 'Cannot read source version.' }
Write-Output 'Custom version comparison checks passed (7 cases).'

$PreviousProtocol = [Net.ServicePointManager]::SecurityProtocol
function Invoke-WebRequest {
    param($Uri, $Method, [switch]$UseBasicParsing, $Headers, $TimeoutSec)
    if ($Uri -ne 'https://github.com/openai/codex/releases/latest' -or $Method -ne 'Head' -or -not $UseBasicParsing -or $TimeoutSec -ne 4) {
        throw 'Unexpected release request.'
    }
    if (-not ([Net.ServicePointManager]::SecurityProtocol -band [Net.SecurityProtocolType]::Tls12)) { throw 'TLS 1.2 must be enabled.' }
    if ($script:RequestFailure) { throw 'Simulated network failure.' }
    $script:ReleaseResponse
}
try {
    $ReleaseUri = [uri]'https://github.com/openai/codex/releases/tag/rust-v0.162.1'
    foreach ($BaseResponse in @(
        @{ ResponseUri = $ReleaseUri },
        @{ RequestMessage = @{ RequestUri = $ReleaseUri } }
    )) {
        $script:ReleaseResponse = [pscustomobject]@{ BaseResponse = [pscustomobject]$BaseResponse }
        $Release = Get-CustomUpstreamRelease
        if ($Release.tag_name -ne 'rust-v0.162.1' -or $Release.draft -or $Release.prerelease) { throw 'Unexpected release metadata.' }
        if ([Net.ServicePointManager]::SecurityProtocol -ne $PreviousProtocol) { throw 'TLS settings were not restored.' }
    }
    foreach ($Path in @('/openai/codex/releases/latest', '/openai/codex/releases/tag/rust-v0.163.0-alpha.1', '/other/repo/releases/tag/rust-v0.162.1')) {
        $script:ReleaseResponse = [pscustomobject]@{ BaseResponse = @{ ResponseUri = [uri]("https://github.com$Path") } }
        $Failure = $null
        try { Get-CustomUpstreamRelease | Out-Null } catch { $Failure = $_ }
        if (-not $Failure -or $Failure.Exception.Message -ne 'Cannot determine the latest stable Codex release from the GitHub redirect.') {
            throw "Unexpected redirect was not rejected: $Path"
        }
    }
    $script:RequestFailure = $true
    $Failure = $null
    try { Get-CustomUpstreamRelease | Out-Null } catch { $Failure = $_ }
    if (-not $Failure -or $Failure.Exception.Message -ne 'Simulated network failure.') { throw 'Network failure was not propagated.' }
    if ([Net.ServicePointManager]::SecurityProtocol -ne $PreviousProtocol) { throw 'TLS settings were not restored after failure.' }
} finally {
    Remove-Item -LiteralPath Function:\Invoke-WebRequest
}
Write-Output 'Custom release lookup checks passed (6 cases).'
