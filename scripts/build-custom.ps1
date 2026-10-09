param(
    [ValidateSet('All', 'Test', 'Build')]
    [string]$Mode = 'All',
    [string]$BuildToolsPath,
    [switch]$NoInstall
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$RustRoot = Join-Path $ProjectRoot 'codex-rs'
$CargoPath = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
$BuildTools = $BuildToolsPath
if (-not $BuildTools) {
    $VsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $VsWhere)) { throw 'Visual Studio Build Tools is required (C++, LLVM and CMake).' }
    $BuildTools = (& $VsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
    if (-not $BuildTools) { throw 'No Visual Studio C++ installation found. Pass -BuildToolsPath explicitly.' }
}
Import-Module (Join-Path $BuildTools 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $BuildTools -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
$LlvmBin = Join-Path $BuildTools 'VC/Tools/Llvm/x64/bin'
$CmakeBin = Join-Path $BuildTools 'Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin'
$env:PATH = "$LlvmBin;$CmakeBin;$(Split-Path -Parent $CargoPath);$env:PATH"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path $LlvmBin 'lld-link.exe'
$env:LIBSQLITE3_FLAGS = 'SQLITE_DISABLE_INTRINSIC'
$env:CARGO_BUILD_JOBS = '8'
$env:RUST_MIN_STACK = '8388608'
$env:RUST_TEST_THREADS = '4'

Push-Location $RustRoot
try {
    $V8Version = (& py -3 -X utf8 (Join-Path $ProjectRoot '.github/scripts/rusty_v8_bazel.py') resolved-v8-crate-version).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve V8 version' }
    $V8Dir = Join-Path $RustRoot 'target/custom-v8'
    New-Item -ItemType Directory -Path $V8Dir -Force | Out-Null
    $V8Prefix = 'rusty_v8_ptrcomp_sandbox_release_x86_64-pc-windows-msvc'
    $ChecksumName = "$V8Prefix.sha256"
    $ArchiveName = "$V8Prefix.lib.gz"
    $BindingName = 'src_binding_ptrcomp_sandbox_release_x86_64-pc-windows-msvc.rs'
    $BaseUrl = "https://github.com/openai/codex/releases/download/rusty-v8-v$V8Version"
    $ManifestPath = Join-Path $V8Dir $ChecksumName
    $TrustedManifest = Join-Path $ProjectRoot ("third_party/v8/rusty_v8_{0}_release_manifests.sha256" -f $V8Version.Replace('.', '_'))
    $ManifestLine = Get-Content -LiteralPath $TrustedManifest -Encoding UTF8 | Where-Object { $_.EndsWith("  $ChecksumName") }
    if (-not $ManifestLine) { throw 'Missing trusted V8 manifest hash' }
    $ManifestHash = $ManifestLine.Split(' ')[0]
    if (-not (Test-Path -LiteralPath $ManifestPath)) {
        Invoke-WebRequest -Uri "$BaseUrl/$ChecksumName" -OutFile $ManifestPath -UseBasicParsing
    }
    if ((Get-FileHash -LiteralPath $ManifestPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $ManifestHash) {
        throw 'V8 manifest checksum mismatch'
    }
    foreach ($Name in @($BindingName, $ArchiveName)) {
        $Artifact = Join-Path $V8Dir $Name
        $HashLine = Get-Content -LiteralPath $ManifestPath -Encoding UTF8 | Where-Object { $_.EndsWith("  $Name") }
        if (-not $HashLine) { throw "Missing artifact hash: $Name" }
        if (-not (Test-Path -LiteralPath $Artifact)) {
            Write-Output "Downloading verified V8 artifact: $Name"
            Invoke-WebRequest -Uri "$BaseUrl/$Name" -OutFile $Artifact -UseBasicParsing
        }
        if ((Get-FileHash -LiteralPath $Artifact -Algorithm SHA256).Hash.ToLowerInvariant() -ne $HashLine.Split(' ')[0]) {
            throw "V8 artifact checksum mismatch: $Name"
        }
    }
    $env:RUSTY_V8_ARCHIVE = Join-Path $V8Dir $ArchiveName
    $env:RUSTY_V8_SRC_BINDING_PATH = Join-Path $V8Dir $BindingName

    if ($Mode -in @('All', 'Test')) {
        foreach ($Filter in @('custom_settings', 'mouse', 'live_activity', 'progress_messages', 'config_schema_matches_fixture', 'custom_update_notice', 'question_mouse', 'async_questions', 'questions_tests')) {
            & $CargoPath test --locked --target x86_64-pc-windows-msvc --profile dev-small -p codex-tui -p codex-core --lib $Filter -- --nocapture
            if ($LASTEXITCODE -ne 0) { throw "TUI tests failed: $Filter" }
        }
    }
    if ($Mode -in @('All', 'Build')) {
        & $CargoPath build --locked --target x86_64-pc-windows-msvc --profile dev-small --bin codex --bin codex-code-mode-host --bin codex-app-server --bin codex-command-runner --bin codex-windows-sandbox-setup --bin codex-windows-sandbox-service
        if ($LASTEXITCODE -ne 0) { throw 'Custom CLI build failed' }
        $Output = Join-Path $RustRoot 'target/x86_64-pc-windows-msvc/dev-small'
        $InstallDir = Join-Path $RustRoot ('target/codex-custom-releases/' + (Get-Date -Format 'yyyyMMdd-HHmmssfff'))
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        foreach ($Name in @('codex', 'codex-code-mode-host', 'codex-app-server', 'codex-command-runner', 'codex-windows-sandbox-setup', 'codex-windows-sandbox-service')) {
            Copy-Item -LiteralPath (Join-Path $Output "$Name.exe") -Destination (Join-Path $InstallDir "$Name.exe") -Force
        }
        $Binary = Join-Path $InstallDir 'codex.exe'
        & $Binary --version
        if ($LASTEXITCODE -ne 0) { throw 'Custom CLI version check failed' }
        if (-not $NoInstall) {
            & (Join-Path $PSScriptRoot 'install-custom-launcher.ps1') -Binary $Binary -ProjectRoot $ProjectRoot
        } else { Write-Output "Verified candidate (not installed): $Binary" }
    }
} finally {
    Pop-Location
}
