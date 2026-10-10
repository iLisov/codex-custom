param([Parameter(Mandatory = $true)][string]$Binary, [Parameter(Mandatory = $true)][string]$ProjectRoot)
$ErrorActionPreference = 'Stop'
$Binary = (Resolve-Path -LiteralPath $Binary).ProviderPath
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).ProviderPath
$VersionText = (& $Binary --version) -join "`n"
if ($LASTEXITCODE -ne 0) { throw 'Cannot read installed binary version.' }
$VersionMatch = [regex]::Match($VersionText, 'codex-cli (\d+\.\d+\.\d+)')
if (-not $VersionMatch.Success) { throw 'Unexpected binary version.' }
$Version = $VersionMatch.Groups[1].Value
$Directory = Split-Path -Parent $Binary
foreach ($Name in @('check-custom-update.ps1','custom-update-common.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $Name) -Destination (Join-Path $Directory $Name) -Force
}
$KnownDirectories = @((Join-Path $env:APPDATA 'npm'), (Join-Path $env:USERPROFILE '.local/bin'))
# Update existing commands together so PATH cannot select an older launcher.
$LauncherDirectories = @($KnownDirectories | Where-Object {
    Test-Path -LiteralPath (Join-Path $_ 'codex-custom.cmd')
})
if ($LauncherDirectories.Count -eq 0) {
    $LauncherDirectories = @(if (Test-Path -LiteralPath $KnownDirectories[0]) { $KnownDirectories[0] } else { $KnownDirectories[1] })
}
# Use this build's backend; the shared daemon can still run older or official code.
$Launcher = "@echo off`r`nset `"CODEX_CUSTOM_PROJECT_ROOT=$ProjectRoot`"`r`npowershell.exe -NoLogo -NoProfile -NonInteractive -File `"$Directory\check-custom-update.ps1`" -CurrentVersion `"$Version`" -ProjectRoot `"$ProjectRoot`" 1>&2`r`n`"$Binary`" --no-daemon %*`r`n"
foreach ($LauncherDirectory in $LauncherDirectories) {
    New-Item -ItemType Directory -Path $LauncherDirectory -Force | Out-Null
    $LauncherFile = Join-Path $LauncherDirectory 'codex-custom.cmd'
    [IO.File]::WriteAllText($LauncherFile, $Launcher, [Text.UTF8Encoding]::new($false))
    Write-Output "Installed command: $LauncherFile"
}
