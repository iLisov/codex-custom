param([string]$TargetVersion, [switch]$PrintPrompt)
$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'custom-update-common.ps1')
$CurrentVersion = Get-CustomSourceVersion $ProjectRoot
if ($TargetVersion) {
    if ($TargetVersion -notmatch '^\d+\.\d+\.\d+$') { throw 'Use a stable numeric release version, for example 0.162.1.' }
    $Release = [pscustomobject]@{ tag_name = "rust-v$TargetVersion"; prerelease = $false; draft = $false }
} else { $Release = Get-CustomUpstreamRelease }
$Newer = Get-CustomNewerRelease -CurrentVersion $CurrentVersion -Release $Release
if (-not $Newer) { Write-Output "Custom CLI base $CurrentVersion is up to date."; exit 0 }
$Prompt = @"
Update my Codex fork from rust-v$CurrentVersion to the official OpenAI release $($Newer.Tag).
The repository is $ProjectRoot, its shared patch branch is custom, and upstream must point to https://github.com/openai/codex.git.
Preserve all custom settings, mouse controls, live activity labels, version notifications, and the user's concise README.
First inspect Git status and project instructions. If unrelated user changes exist, preserve them and do not overwrite them.
Fetch exactly the official release tag. Work on an isolated candidate branch/worktree based on custom, inspect upstream changes and resolve conflicts while retaining our features.
Use scripts/build-custom.ps1 -Mode All -NoInstall to test and compile the candidate before switching the installed CLI.
When the candidate passes, verify custom has not changed concurrently, merge the tested result into custom, and run scripts/build-custom.ps1 -Mode All in the original repository to install the verified version.
This maintenance task authorizes the required update commits and a normal non-force push to origin/custom after successful validation.
Do not force-push, delete user changes, publish ZIP releases, or change main. On failure, keep the installed CLI unchanged and report what is blocked.
Update only the checked base-version value in README if needed; retain the user's other text. Reply in Russian with the result and checks.
"@
if ($PrintPrompt) { Write-Output $Prompt; exit 0 }
$Status = & git -C $ProjectRoot status --porcelain
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect repository.' }
if ($Status) { throw 'Commit or preserve the current changes before starting the update session.' }
$Branch = (& git -C $ProjectRoot branch --show-current).Trim()
if ($Branch -ne 'custom') { throw 'Run the updater on the custom branch.' }
$Command = Get-Command codex-custom -ErrorAction SilentlyContinue
if (-not $Command) { throw 'Install codex-custom first using scripts/build-custom.ps1.' }
$Binary = $Command.Source
if ([IO.Path]::GetExtension($Binary) -eq '.cmd') {
    $LauncherText = Get-Content -LiteralPath $Binary -Raw -Encoding UTF8
    $BinaryMatch = [regex]::Match($LauncherText, '(?m)^"([^"\r\n]+codex\.exe)"\s+%\*\s*$')
    if (-not $BinaryMatch.Success) { throw 'Cannot resolve the custom executable from its launcher.' }
    $Binary = $BinaryMatch.Groups[1].Value
}
$PreviousCheckFlag = $env:CODEX_CUSTOM_SKIP_VERSION_CHECK
try {
    $env:CODEX_CUSTOM_SKIP_VERSION_CHECK = '1'
    & $Binary -C $ProjectRoot $Prompt
    $ExitCode = $LASTEXITCODE
} finally {
    $env:CODEX_CUSTOM_SKIP_VERSION_CHECK = $PreviousCheckFlag
}
exit $ExitCode
