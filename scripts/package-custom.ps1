param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryDirectory,
    [string]$Version = '0.162.0-lisov.1',
    [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^[A-Za-z0-9._-]+$') { throw 'Invalid package version' }
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $ProjectRoot 'codex-rs/target/custom-packages' }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$PackageName = "codex-custom-$Version-windows-x64"
$Stage = Join-Path $OutputDirectory ($PackageName + '-' + (Get-Date -Format 'yyyyMMdd-HHmmssfff'))
New-Item -ItemType Directory -Path $Stage | Out-Null
$Names = @('codex', 'codex-code-mode-host', 'codex-app-server', 'codex-command-runner', 'codex-windows-sandbox-setup', 'codex-windows-sandbox-service')
foreach ($Name in $Names) {
    $Source = Join-Path $BinaryDirectory "$Name.exe"
    if (-not (Test-Path -LiteralPath $Source)) { throw "Missing executable: $Name" }
    Copy-Item -LiteralPath $Source -Destination (Join-Path $Stage "$Name.exe")
}
foreach ($Name in @('LICENSE', 'NOTICE', 'README.md')) {
    Copy-Item -LiteralPath (Join-Path $ProjectRoot $Name) -Destination (Join-Path $Stage $Name)
}
$Launcher = '@echo off' + "`r`n" + '"%~dp0codex.exe" %*' + "`r`n"
[System.IO.File]::WriteAllText((Join-Path $Stage 'codex-custom.cmd'), $Launcher, [System.Text.UTF8Encoding]::new($false))
$Commit = (& git -C $ProjectRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot determine source commit' }
$Info = @{ version = $Version; base = 'rust-v0.162.0'; source_commit = $Commit; profile = 'dev-small'; target = 'x86_64-pc-windows-msvc' } | ConvertTo-Json
[System.IO.File]::WriteAllText((Join-Path $Stage 'BUILD-INFO.json'), $Info, [System.Text.UTF8Encoding]::new($false))
$Hashes = Get-ChildItem -LiteralPath $Stage -File | Sort-Object Name | ForEach-Object { (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $_.Name }
[System.IO.File]::WriteAllLines((Join-Path $Stage 'SHA256SUMS.txt'), $Hashes, [System.Text.UTF8Encoding]::new($false))
$Archive = Join-Path $OutputDirectory ($PackageName + '.zip')
if (Test-Path -LiteralPath $Archive) { throw 'Archive already exists; use a new version or output directory' }
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($Stage, $Archive, [System.IO.Compression.CompressionLevel]::Optimal, $false)
$ArchiveHash = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText(($Archive + '.sha256'), "$ArchiveHash  $PackageName.zip`n", [System.Text.UTF8Encoding]::new($false))
Write-Output "Package: $Archive"
Write-Output "SHA256: $ArchiveHash"
