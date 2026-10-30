#requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '../packaging/windows/system-path.ps1')

function Assert-Equal($Actual, $Expected, [string] $Context) {
    if ($Actual -cne $Expected) { throw "$Context : expected <$Expected>, actual <$Actual>" }
}
function Read-Path {
    $environmentKey.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
}

$testRoot = 'Software\KaguraInstallerTests\' + [Guid]::NewGuid().ToString('N')
$environmentKey = $null
$stateKey = $null
$entry = Join-Path $env:TEMP 'Kagura 日本語 test'
try {
    $environmentKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey("$testRoot\Environment")
    $stateKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey("$testRoot\State")
    foreach ($kind in @([Microsoft.Win32.RegistryValueKind]::String, [Microsoft.Win32.RegistryValueKind]::ExpandString)) {
        $before = '%SystemRoot%\System32;C:\Other app\日本語;'
        $environmentKey.SetValue('Path', $before, $kind)
        Update-KaguraSystemPath $environmentKey $stateKey Add $entry
        Assert-Equal (Read-Path) "$before;$entry" "add $kind"
        Assert-Equal ($environmentKey.GetValueKind('Path')) $kind 'registry kind'
        Update-KaguraSystemPath $environmentKey $stateKey Add $entry
        Assert-Equal (Read-Path) "$before;$entry" 'reinstall ownership'
        $environmentKey.SetValue('Path', (Read-Path) + ';C:\After install', $kind)
        Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
        Assert-Equal (Read-Path) "$before;C:\After install" 'preserve later edits'
        Assert-Equal ($environmentKey.GetValueKind('Path')) $kind 'remove kind'
        Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
        Assert-Equal (Read-Path) "$before;C:\After install" 'repeat removal'
    }
    $manual = '"' + $entry.ToUpperInvariant() + '\"'
    $environmentKey.SetValue('Path', "C:\Before;$manual", [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'manual entry is not owned'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) "C:\Before;$manual" 'manual entry survives'
    $manual = '%TEMP%/./Kagura 日本語 test/'
    $environmentKey.SetValue('Path', $manual, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) $manual 'raw variable and normalized manual entry survive'

    $manual = '%TEMP%\Kagura 日本語 test'
    $environmentKey.SetValue('Path', $manual, [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Assert-Equal (Read-Path) "$manual;$entry" 'REG_SZ variables remain literal during comparison'
    Assert-Equal ($stateKey.GetValue('PathEntry')) $entry 'REG_SZ literal does not prevent ownership'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) $manual 'REG_SZ literal survives removal'
    Assert-Equal ($environmentKey.GetValueKind('Path')) ([Microsoft.Win32.RegistryValueKind]::String) 'REG_SZ kind survives removal'
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'REG_SZ ownership cleared'

    $environmentKey.SetValue('Path', $manual, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Assert-Equal (Read-Path) $manual 'REG_EXPAND_SZ variables identify a preexisting path'
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'REG_EXPAND_SZ manual path is not owned'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) $manual 'REG_EXPAND_SZ manual variable survives removal'
    Assert-Equal ($environmentKey.GetValueKind('Path')) ([Microsoft.Win32.RegistryValueKind]::ExpandString) 'REG_EXPAND_SZ kind survives removal'

    # An unselected task never calls Add; uninstall still runs Remove.
    $environmentKey.SetValue('Path', '%TEMP%\unrelated', [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) '%TEMP%\unrelated' 'no PATH selection'

    $environmentKey.DeleteValue('Path')
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) $null 'normal reinstall restores absent PATH'

    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    $environmentKey.SetValue('Path', '', [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Assert-Equal ($stateKey.GetValue('CreatedPath')) 0 'reinstall does not inherit ownership of a replaced PATH value'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) '' 'reinstall preserves user replacement with empty PATH'
    Assert-Equal ($environmentKey.GetValueKind('Path')) ([Microsoft.Win32.RegistryValueKind]::String) 'reinstall preserves empty REG_SZ kind'

    $environmentKey.DeleteValue('Path')
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    $manual = '%TEMP%\Kagura 日本語 test'
    $environmentKey.SetValue('Path', $manual, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'replacement variable entry becomes manual'
    Assert-Equal ($stateKey.GetValue('CreatedPath', $null)) $null 'replacement variable clears former CreatedPath'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) $manual 'replacement manual variable entry survives uninstall'

    foreach ($kind in @([Microsoft.Win32.RegistryValueKind]::String, [Microsoft.Win32.RegistryValueKind]::ExpandString)) {
        $environmentKey.DeleteValue('Path')
        Update-KaguraSystemPath $environmentKey $stateKey Add $entry
        $environmentKey.SetValue('Path', "$entry;", $kind)
        Update-KaguraSystemPath $environmentKey $stateKey Add $entry
        Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
        Assert-Equal (Read-Path) '' "user-added empty token survives reinstall and removal $kind"
        Assert-Equal ($environmentKey.GetValueKind('Path')) $kind 'user-added empty token preserves value kind'
    }

    $environmentKey.SetValue('Path', '', [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    $environmentKey.SetValue('Path', (Read-Path) + ";$entry", [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) $entry 'remove one duplicate only'

    $manualBefore = $entry.ToUpperInvariant() + '\'
    $manualAfter = '"' + $entry.ToLowerInvariant() + '\"'
    $environmentKey.SetValue('Path', 'C:\Base', [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    $environmentKey.SetValue('Path', "$manualBefore;C:\Base;$entry;$manualAfter", [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) "$manualBefore;C:\Base;$manualAfter" 'manual equivalents before and after owned raw entry survive'
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'exact owned removal clears ownership'

    $environmentKey.SetValue('Path', 'C:\Base', [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    $environmentKey.SetValue('Path', "C:\Base;$manualAfter", [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) "C:\Base;$manualAfter" 'manual equivalent survives when original raw entry is absent'
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'missing owned raw entry clears ownership'

    $environmentKey.SetValue('Path', 'C:\Base', [Microsoft.Win32.RegistryValueKind]::String)
    Update-KaguraSystemPath $environmentKey $stateKey Add $entry
    $moved = "$entry moved"
    Update-KaguraSystemPath $environmentKey $stateKey Add $moved
    Assert-Equal (Read-Path) "C:\Base;$moved" 'changed installation directory'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $entry
    Assert-Equal (Read-Path) "C:\Base;$moved" 'old uninstaller preserves active PATH'
    Assert-Equal ($stateKey.GetValue('PathEntry')) $moved 'old uninstaller preserves ownership'
    Update-KaguraSystemPath $environmentKey $stateKey Remove $moved
    Assert-Equal (Read-Path) 'C:\Base' 'remove changed directory'

    $rejected = $false
    try { Update-KaguraSystemPath $environmentKey $stateKey Add 'C:\invalid;entry' } catch { $rejected = $true }
    Assert-Equal $rejected $true 'reject ambiguous entry'
    Assert-Equal (Read-Path) 'C:\Base' 'rejected entry preserves PATH'
    $environmentKey.SetValue('Path', '%SystemRoot%\System32', [Microsoft.Win32.RegistryValueKind]::ExpandString)
    $rejected = $false
    try { Update-KaguraSystemPath $environmentKey $stateKey Add 'C:\Kagura %TEMP%' } catch { $rejected = $true }
    Assert-Equal $rejected $true 'reject percent in installation path'
    Assert-Equal (Read-Path) '%SystemRoot%\System32' 'rejected percent preserves raw PATH'
    Assert-Equal ($environmentKey.GetValueKind('Path')) ([Microsoft.Win32.RegistryValueKind]::ExpandString) 'rejected percent preserves PATH kind'
    Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'rejected percent does not acquire ownership'
    $environmentKey.SetValue('Path', [byte[]] @(1, 2), [Microsoft.Win32.RegistryValueKind]::Binary)
    $rejected = $false
    try { Update-KaguraSystemPath $environmentKey $stateKey Add $entry } catch { $rejected = $true }
    Assert-Equal $rejected $true 'reject unsupported registry kind'

    $environmentKey.SetValue('Path', 'C:\Rollback', [Microsoft.Win32.RegistryValueKind]::String)
    $readOnlyState = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("$testRoot\State")
    try {
        $rejected = $false
        try { Update-KaguraSystemPath $environmentKey $readOnlyState Add $entry } catch { $rejected = $true }
        Assert-Equal $rejected $true 'ownership write failure is reported'
        Assert-Equal (Read-Path) 'C:\Rollback' 'ownership write failure rolls back PATH text'
        Assert-Equal ($environmentKey.GetValueKind('Path')) ([Microsoft.Win32.RegistryValueKind]::String) 'rollback preserves registry kind'
        Assert-Equal ($stateKey.GetValue('PathEntry', $null)) $null 'failed ownership write leaves no ownership'
    } finally { $readOnlyState.Dispose() }
    Write-Output 'PATH checks passed (isolated HKCU keys only).'
} finally {
    if ($stateKey) { $stateKey.Dispose() }
    if ($environmentKey) { $environmentKey.Dispose() }
    # Only the GUID-owned test key is removed; never HKCU\Environment.
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($testRoot, $false)
    if ($null -ne [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($testRoot)) {
        throw "Test registry cleanup failed: $testRoot"
    }
}
