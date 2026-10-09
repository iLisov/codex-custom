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
