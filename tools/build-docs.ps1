#requires -Version 7.0

param(
    [string] $Python,
    [switch] $Serve
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$targetRoot = [IO.Path]::GetFullPath((Join-Path $repo 'target/website'))
$sources = Join-Path $targetRoot 'sources'
$public = Join-Path $targetRoot 'public'
if (-not $targetRoot.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "生成先がリポジトリ外です: $targetRoot"
}
$overrides = Join-Path $targetRoot 'overrides'
if (-not $overrides.StartsWith($targetRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "生成先がtarget/websiteの外です: $overrides"
}

if (-not $Python) {
    if ($IsWindows -or $env:OS -eq 'Windows_NT') {
        $Python = Join-Path $targetRoot 'venv/Scripts/python.exe'
    } else {
        $Python = Join-Path $targetRoot 'venv/bin/python'
    }
}
if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw "Pythonがありません: $Python`n先に python -m venv target/website/venv と python -m pip install -r website/requirements.txt を実行してください。"
}
$Python = (Resolve-Path -LiteralPath $Python).Path
$version = & $Python -m zensical --version 2>&1
if ($LASTEXITCODE -ne 0) { throw "Zensicalを実行できません: $version" }

foreach ($path in @($sources, $public)) {
    $resolved = [IO.Path]::GetFullPath($path)
    if (-not $resolved.StartsWith($targetRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "生成先がtarget/websiteの外です: $resolved"
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
if (Test-Path -LiteralPath $overrides) { Remove-Item -LiteralPath $overrides -Recurse -Force }
New-Item -ItemType Directory -Path $sources -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repo 'website/index.md') -Destination (Join-Path $sources 'index.md')
Copy-Item -LiteralPath (Join-Path $repo 'specs') -Destination (Join-Path $sources 'specs') -Recurse
$conformance = Join-Path $sources 'specs/kagura/v1/conformance.md'
# The source has both conformance.md and conformance/README.md; rename only the generated copy to avoid the shared URL.
Move-Item -LiteralPath $conformance -Destination (Join-Path (Split-Path $conformance) 'conformance-overview.md')
$specIndex = Join-Path $sources 'specs/kagura/v1/README.md'
$indexText = [IO.File]::ReadAllText($specIndex).Replace('./conformance.md', './conformance-overview.md')
[IO.File]::WriteAllText($specIndex, $indexText, [Text.UTF8Encoding]::new($false))
foreach ($suite in @('cpu', 'bus')) {
    $caseRoot = Join-Path $sources "specs/kagura/v1/conformance/cases/$suite"
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("# $suite cases")
    $lines.Add('')
    Get-ChildItem -LiteralPath $caseRoot -Recurse -File | Where-Object { $_.Name -in @('case.toml', 'program.bin') } | ForEach-Object {
        $relative = [IO.Path]::GetRelativePath($caseRoot, $_.FullName).Replace('\', '/')
        $lines.Add("- [$relative](./$relative)")
    }
    [IO.File]::WriteAllLines((Join-Path $caseRoot 'index.md'), $lines, [Text.UTF8Encoding]::new($false))
}
New-Item -ItemType Directory -Path (Join-Path $sources 'assets') -Force | Out-Null
Get-ChildItem -LiteralPath (Join-Path $repo 'assets') -Filter '*.png' -File | Copy-Item -Destination (Join-Path $sources 'assets')
New-Item -ItemType Directory -Path (Join-Path $sources 'assets/stylesheets') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repo 'website/assets/stylesheets/site.css') -Destination (Join-Path $sources 'assets/stylesheets/site.css') -Force
Copy-Item -LiteralPath (Join-Path $repo 'website/overrides') -Destination $overrides -Recurse
Copy-Item -LiteralPath (Join-Path $repo 'website/zensical.toml') -Destination (Join-Path $targetRoot 'zensical.toml')

Push-Location $repo
try {
    & $Python -m zensical build --config-file target/website/zensical.toml --clean --strict
    if ($LASTEXITCODE -ne 0) { throw "Zensical buildに失敗しました ($LASTEXITCODE)" }
    Get-ChildItem -LiteralPath (Join-Path $sources 'specs') -Recurse -File | Where-Object { $_.Extension -ne '.md' } | ForEach-Object {
        $relative = [IO.Path]::GetRelativePath($sources, $_.FullName)
        $destination = Join-Path $public $relative
        New-Item -ItemType Directory -Path (Split-Path $destination) -Force | Out-Null
        Copy-Item -LiteralPath $_.FullName -Destination $destination
    }
    & $Python tools/check-docs-links.py target/website/public
    if ($LASTEXITCODE -ne 0) { throw "生成HTMLの内部リンク検査に失敗しました ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

if ($Serve) {
    & $Python -m http.server 8012 --bind 127.0.0.1 --directory $public
    if ($LASTEXITCODE -ne 0) { throw "ローカル配信に失敗しました ($LASTEXITCODE)" }
}
