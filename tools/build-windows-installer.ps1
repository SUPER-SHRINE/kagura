#requires -Version 7.0
param([string] $Iscc)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Windowsで実行してください。' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $Iscc) { $Iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe' }
if (-not (Test-Path -LiteralPath $Iscc -PathType Leaf)) { throw "Inno Setup 6.3以降のISCC.exeが必要です: $Iscc" }
$binary = Join-Path $repo 'support/target/x86_64-pc-windows-msvc/release/kagura.exe'
foreach ($inputPath in @($binary, (Join-Path $repo 'LICENSE.md'), (Join-Path $repo 'README.md'), (Join-Path $repo 'specs/README.md'))) {
    if (-not (Test-Path -LiteralPath $inputPath -PathType Leaf)) { throw "必要な入力がありません: $inputPath" }
}
$metadata = & cargo metadata --manifest-path (Join-Path $repo 'support/Cargo.toml') --format-version 1 --no-deps --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'support workspaceのversionを取得できません。' }
$packages = ($metadata | ConvertFrom-Json).packages
$versions = @($packages.version | Sort-Object -Unique)
if ($versions.Count -ne 1) { throw 'support workspaceのversionが一致していません。' }
$version = $versions[0]
$binaryVersion = & $binary --version
if ($LASTEXITCODE -ne 0 -or $binaryVersion -cne "kagura $version") { throw "release binaryのversionが一致していません: $binaryVersion" }
$outputRoot = Join-Path $repo 'target/windows-installer/dist'
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
& $Iscc /Q "/DAppVersion=$version" "/DBinaryPath=$binary" "/DSourceRoot=$repo" "/DOutputRoot=$outputRoot" (Join-Path $repo 'packaging/windows/kagura.iss')
if ($LASTEXITCODE -ne 0) { throw "Inno Setupのcompileに失敗しました ($LASTEXITCODE)" }
$output = Join-Path $outputRoot "kagura-$version-x86_64-pc-windows-msvc-setup.exe"
if (-not (Test-Path -LiteralPath $output -PathType Leaf)) { throw "生成物がありません: $output" }
Write-Output $output
