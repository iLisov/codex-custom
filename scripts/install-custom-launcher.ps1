param([Parameter(Mandatory = $true)][string]$Binary, [Parameter(Mandatory = $true)][string]$ProjectRoot)
$ErrorActionPreference = 'Stop'
$VersionText = (& $Binary --version) -join "`n"
if ($LASTEXITCODE -ne 0) { throw 'Cannot read installed binary version.' }
$VersionMatch = [regex]::Match($VersionText, 'codex-cli (\d+\.\d+\.\d+)')
if (-not $VersionMatch.Success) { throw 'Unexpected binary version.' }
$Version = $VersionMatch.Groups[1].Value
$Directory = Split-Path -Parent $Binary
foreach ($Name in @('check-custom-update.ps1','custom-update-common.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $Name) -Destination (Join-Path $Directory $Name) -Force
}
$LauncherDirectory = Join-Path $env:APPDATA 'npm'
if (-not (Test-Path -LiteralPath $LauncherDirectory)) { $LauncherDirectory = Join-Path $env:USERPROFILE '.local/bin' }
New-Item -ItemType Directory -Path $LauncherDirectory -Force | Out-Null
$Launcher = "@echo off`r`nset `"CODEX_CUSTOM_PROJECT_ROOT=$ProjectRoot`"`r`npowershell.exe -NoLogo -NoProfile -NonInteractive -File `"$Directory\check-custom-update.ps1`" -CurrentVersion `"$Version`" -ProjectRoot `"$ProjectRoot`" 1>&2`r`n`"$Binary`" %*`r`n"
$LauncherFile = Join-Path $LauncherDirectory 'codex-custom.cmd'
[IO.File]::WriteAllText($LauncherFile, $Launcher, [Text.UTF8Encoding]::new($false))
Write-Output "Installed command: $LauncherFile"
