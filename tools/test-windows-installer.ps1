#requires -Version 7.0
param(
    [Parameter(Mandatory)] [string] $Installer,
    [string] $Iscc = (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# This smoke test installs a product and changes the real system PATH: CI only.
if (-not $IsWindows -or $env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') {
    throw '実インストール検査はGitHub Actionsの一時Windows runner専用です。'
}
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) -or -not [Environment]::Is64BitProcess) {
    throw '管理者権限のある64-bit PowerShellで実行してください。'
}
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$statePath = 'Software\Kagura\Installer'
$uninstallPath = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{72FD7323-E0F6-486F-9CD0-0C08A45BA16A}_is1'
$machineKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
$environmentKey = $null
$userEnvironmentKey = $null
$pathChanged = $false
try {
    foreach ($root in @($machineKey, [Microsoft.Win32.Registry]::CurrentUser)) {
        foreach ($keyPath in @($statePath, $uninstallPath)) {
            $existing = $root.OpenSubKey($keyPath)
            if ($existing) { $existing.Dispose(); throw "既存インストールの状態があります: $($root.Name)\$keyPath" }
        }
    }
    $app = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)) 'SUPER-SHRINE/Kagura'
    if (Test-Path -LiteralPath $app) { throw "既定の配置先が既に存在します: $app" }
    $work = Join-Path $env:RUNNER_TEMP ('Kagura installer 日本語 ' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $work | Out-Null
    # The old uninstall log must own the obsolete file; an untracked fixture would prove nothing.
    $repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
    $obsoleteSource = Join-Path $work 'obsolete.txt'
    $oldReadme = Join-Path $work 'old-readme.txt'
    Set-Content -LiteralPath $obsoleteSource -Value 'old managed file'
    Set-Content -LiteralPath $oldReadme -Value 'old payload marker'
    $oldScript = [IO.File]::ReadAllText((Join-Path $repo 'packaging/windows/kagura.iss'))
    $oldScript = $oldScript.Replace('[Files]', ('[Files]' + "`nSource: `"$obsoleteSource`"; DestDir: `"{app}`"; Flags: ignoreversion"))
    $oldScript = $oldScript.Replace('Source: "{#SourceRoot}\README.md";', "Source: `"$oldReadme`"; DestName: `"README.md`";")
    $oldScriptPath = Join-Path $work 'old-payload.iss'
    [IO.File]::WriteAllText($oldScriptPath, $oldScript, [Text.UTF8Encoding]::new($false))
    $binary = Join-Path $repo 'support/target/x86_64-pc-windows-msvc/release/kagura.exe'
    & $Iscc /Q '/DAppVersion=0.0.0-fixture' "/DBinaryPath=$binary" "/DSourceRoot=$repo" "/DOutputRoot=$work" $oldScriptPath
    if ($LASTEXITCODE -ne 0) { throw '旧ペイロード検査用インストーラーのcompileに失敗しました。' }
    $oldInstaller = Join-Path $work 'kagura-0.0.0-fixture-x86_64-pc-windows-msvc-setup.exe'
    $environmentKey = $machineKey.OpenSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment', $true)
    if (-not $environmentKey) { throw 'システム環境変数のレジストリを開けません。' }
    $raw = $environmentKey.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $kind = if ($null -ne $raw) { $environmentKey.GetValueKind('Path') } else { [Microsoft.Win32.RegistryValueKind]::ExpandString }
    $userEnvironmentKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
    $userRaw = if ($userEnvironmentKey) { $userEnvironmentKey.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { $null }
    $userKind = if ($null -ne $userRaw) { $userEnvironmentKey.GetValueKind('Path') } else { $null }
    function Assert-UserPath {
        # Reopen the key so a newly created user PATH is detected too.
        $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
        try {
            $current = if ($key) { $key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { $null }
            $currentKind = if ($null -ne $current) { $key.GetValueKind('Path') } else { $null }
            if ($current -cne $userRaw -or $currentKind -ne $userKind) { throw 'ユーザーPATHが変更されました。' }
        } finally { if ($key) { $key.Dispose() } }
    }
    function Read-Path { $environmentKey.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) }
    function Assert-Path([string] $Expected) {
        if ((Read-Path) -cne $Expected) { throw 'システムPATHの期待値と一致しません。' }
    }
    function Run-Installer {
        param(
            [AllowNull()] [string] $Task,
            [string] $Directory,
            [string] $SetupPath = $Installer,
            [switch] $ExpectFailure
        )
        $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
        if ($PSBoundParameters.ContainsKey('Task')) { $arguments += "/TASKS=`"$Task`"" }
        if ($Directory) { $arguments += "/DIR=`"$Directory`"" }
        $arguments += "/LOG=`"$work\install-$([Guid]::NewGuid().ToString('N')).log`""
        $process = Start-Process -FilePath $SetupPath -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
        Assert-UserPath
        if ($ExpectFailure) {
            if ($process.ExitCode -eq 0) { throw '旧版の削除失敗時に更新が成功しました。' }
            return
        }
        if ($process.ExitCode -ne 0) { throw "インストール失敗: $($process.ExitCode)" }
        if (-not (Test-Path -LiteralPath (Join-Path $app 'kagura.exe'))) { throw "配置先に実行ファイルがありません: $app" }
        $registration = $machineKey.OpenSubKey($uninstallPath)
        try {
            if (-not $registration -or $registration.GetValue('InstallLocation').TrimEnd('\') -ine $app) {
                throw '64-bitのマシン単位アンインストール登録がありません。'
            }
        } finally { if ($registration) { $registration.Dispose() } }
    }
    function Run-Uninstaller {
        param([switch] $ExpectFailure)
        $registration = $machineKey.OpenSubKey($uninstallPath)
        try {
            if (-not $registration) { throw '64-bitのマシン単位アンインストール登録がありません。' }
            $uninstallString = $registration.GetValue('UninstallString')
            if ([string]::IsNullOrWhiteSpace($uninstallString)) { throw 'アンインストール登録にUninstallStringがありません。' }
            if ($uninstallString -match '^\s*"(?<exe>[^"]+\.exe)"(?:\s|$)') {
                $uninstaller = $Matches.exe
            } elseif ($uninstallString -match '^\s*(?<exe>.+?\.exe)(?:\s|$)') {
                $uninstaller = $Matches.exe.Trim()
            } else {
                throw "UninstallStringから実行ファイルを取得できません: $uninstallString"
            }
        } finally { $registration.Dispose() }
        $process = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$work\uninstall.log`"") -Wait -PassThru -WindowStyle Hidden
        Assert-UserPath
        if ($ExpectFailure) {
            if ($process.ExitCode -eq 0 -or -not (Test-Path -LiteralPath (Join-Path $app 'kagura.exe'))) {
                throw 'PATH更新失敗時に非0で停止し、実行ファイルを保持できませんでした。'
            }
            return
        }
        if ($process.ExitCode -ne 0) { throw "アンインストール失敗: $($process.ExitCode)" }
        if (Test-Path -LiteralPath (Join-Path $app 'kagura.exe')) { throw '実行ファイルが削除されていません。' }
        $registration = $machineKey.OpenSubKey($uninstallPath)
        if ($registration) { $registration.Dispose(); throw 'マシン単位アンインストール登録が残っています。' }
    }
    $before = '%SystemRoot%\System32;C:\Other app\日本語'
    $pathChanged = $true
    $environmentKey.SetValue('Path', $before, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Run-Installer -SetupPath $oldInstaller
    Assert-Path "$before;$app"
    $ownership = $machineKey.OpenSubKey($statePath)
    try {
        if (-not $ownership -or $ownership.GetValue('PathEntry') -cne $app) { throw '64-bitのマシン単位PATH所有権がありません。' }
    } finally { if ($ownership) { $ownership.Dispose() } }
    $version = & (Join-Path $app 'kagura.exe') --version
    if ($LASTEXITCODE -ne 0 -or $version -cne 'kagura 0.2.0') { throw "version不一致: $version" }
    $conformance = & (Join-Path $app 'kagura.exe') conformance (Join-Path $app 'specs/kagura/v1/conformance')
    if ($LASTEXITCODE -ne 0 -or -not ($conformance -match '^181 cases: 181 passed, 0 failed, 0 errors$')) { throw 'インストール済み適合試験に失敗しました。' }
    $obsolete = Join-Path $app 'obsolete.txt'
    if (-not (Test-Path -LiteralPath $obsolete)) { throw '旧インストーラーの管理対象ファイルがありません。' }
    $sentinel = Join-Path $app 'user-added.txt'
    Set-Content -LiteralPath $sentinel -Value 'keep user file'
    $oldReadmeHash = (Get-FileHash -LiteralPath (Join-Path $app 'README.md')).Hash
    # A failed old uninstaller must block the new payload before any files are copied.
    $environmentKey.SetValue('Path', [byte[]] @(1, 2), [Microsoft.Win32.RegistryValueKind]::Binary)
    Run-Installer -ExpectFailure
    if (-not (Test-Path -LiteralPath $obsolete) -or
        (Get-FileHash -LiteralPath (Join-Path $app 'README.md')).Hash -cne $oldReadmeHash) {
        throw '旧版の削除失敗後にペイロードが変更されました。'
    }
    $registration = $machineKey.OpenSubKey($uninstallPath)
    try {
        if (-not $registration -or $registration.GetValue('DisplayVersion') -cne '0.0.0-fixture') {
            throw '旧版の削除失敗後に登録が変更されました。'
        }
    } finally { if ($registration) { $registration.Dispose() } }
    $environmentKey.SetValue('Path', "$before;$app;C:\Added later", [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Run-Installer
    Assert-Path "$before;C:\Added later;$app"
    if (Test-Path -LiteralPath $obsolete) { throw '旧版の管理対象ファイルが更新時に削除されていません。' }
    if ((Get-Content -LiteralPath $sentinel -Raw).Trim() -cne 'keep user file') { throw '利用者が追加したファイルが保持されていません。' }
    if ((Get-FileHash -LiteralPath (Join-Path $app 'README.md')).Hash -cne (Get-FileHash -LiteralPath (Join-Path $repo 'README.md')).Hash) {
        throw '新版のペイロードが配置されていません。'
    }
    # Force a helper failure and verify usUninstall aborts before deleting files.
    $environmentKey.SetValue('Path', [byte[]] @(1, 2), [Microsoft.Win32.RegistryValueKind]::Binary)
    Run-Uninstaller -ExpectFailure
    $environmentKey.SetValue('Path', "$before;$app;C:\Added later", [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Run-Uninstaller
    Assert-Path "$before;C:\Added later"
    $environmentKey.SetValue('Path', "$before;$app", [Microsoft.Win32.RegistryValueKind]::String)
    Run-Installer 'systempath'
    Run-Uninstaller
    Assert-Path "$before;$app"
    if ($environmentKey.GetValueKind('Path') -ne [Microsoft.Win32.RegistryValueKind]::String) { throw 'REG_SZが変更されました。' }
    $environmentKey.SetValue('Path', $before, [Microsoft.Win32.RegistryValueKind]::String)
    $app = Join-Path $work 'custom Kagura 日本語'
    Run-Installer -Task '' -Directory $app
    Run-Installer
    Assert-Path $before
    Run-Uninstaller
    Assert-Path $before
    $app = Join-Path ($env:SystemDrive + '\') ('KaguraInstallerTest-' + [Guid]::NewGuid().ToString('N'))
    if ($app.Contains(' ') -or (Test-Path -LiteralPath $app)) { throw "空白なし配置先の検査条件を満たしません: $app" }
    Run-Installer -Task '' -Directory $app
    Run-Installer
    Assert-Path $before
    Run-Uninstaller
    Assert-Path $before
    Write-Output 'Installer smoke passed: default/custom/no-space directory, machine registration/PATH, unchanged user PATH, conformance, upgrade cleanup/failure, preserved user files/directory/task, owned/manual/unchecked PATH, uninstall.'
} finally {
    try {
        if ($pathChanged) {
            if ($null -eq $raw) { $environmentKey.DeleteValue('Path', $false) }
            else { $environmentKey.SetValue('Path', $raw, $kind) }
            Assert-UserPath
        }
    } finally {
        if ($userEnvironmentKey) { $userEnvironmentKey.Dispose() }
        if ($environmentKey) { $environmentKey.Dispose() }
        $machineKey.Dispose()
    }
    # Keep test logs on failure; the runner discards its temporary workspace.
}
