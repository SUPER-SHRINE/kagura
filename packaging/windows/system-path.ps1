#requires -Version 5.1
param(
    [ValidateSet('Add', 'Remove')] [string] $Action,
    [string] $InstallDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ComparablePath([string] $Entry, [bool] $ExpandVariables = $false) {
    $expanded = $Entry.Trim().Trim('"')
    if ($ExpandVariables) { $expanded = [Environment]::ExpandEnvironmentVariables($expanded) }
    $expanded = $expanded.Replace('/', '\')
    if ($expanded -match '^(?:[A-Za-z]:\\|\\\\)') {
        try { $expanded = [IO.Path]::GetFullPath($expanded) }
        catch [ArgumentException] { } # Leave malformed unrelated entries unchanged.
    }
    $expanded.TrimEnd('\')
}

# RegistryKey arguments let the checks use isolated HKCU keys instead of the system Environment.
function Update-KaguraSystemPath {
    param(
        [Microsoft.Win32.RegistryKey] $EnvironmentKey,
        [Microsoft.Win32.RegistryKey] $StateKey,
        [ValidateSet('Add', 'Remove')] [string] $Operation,
        [string] $Entry
    )
    $owned = [string] $StateKey.GetValue('PathEntry', '')
    if ($Operation -eq 'Remove' -and (-not $owned -or (Get-ComparablePath $owned) -ine (Get-ComparablePath $Entry))) {
        return
    }
    if ($Operation -eq 'Add' -and (-not [IO.Path]::IsPathRooted($Entry) -or $Entry.Contains(';') -or $Entry.Contains('"') -or $Entry.Contains('%'))) {
        throw 'The installation path must be absolute and contain no semicolons, quotes, or percent signs.'
    }
    $raw = $EnvironmentKey.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $exists = $null -ne $raw
    $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
    if ($exists) {
        $kind = $EnvironmentKey.GetValueKind('Path')
        if ($kind -notin @([Microsoft.Win32.RegistryValueKind]::String, [Microsoft.Win32.RegistryValueKind]::ExpandString)) {
            throw 'System PATH must be REG_SZ or REG_EXPAND_SZ.'
        }
    }
    $original = [string] $raw
    $expandVariables = $kind -eq [Microsoft.Win32.RegistryValueKind]::ExpandString
    $created = $StateKey.GetValue('CreatedPath', 0) -eq 1
    $parts = [Collections.Generic.List[string]]::new()
    if ($original.Length -gt 0) { $parts.AddRange([string[]] $original.Split(';')) }

    if ($Operation -eq 'Add' -and $owned) {
        $ownedPresent = $false
        foreach ($part in $parts) {
            if ([string]::Equals($part, $owned, [StringComparison]::Ordinal)) { $ownedPresent = $true; break }
        }
        if (-not $ownedPresent) { $owned = ''; $created = $false }
    }

    if ($owned -and ($Operation -eq 'Remove' -or (Get-ComparablePath $owned) -ine (Get-ComparablePath $Entry))) {
        # Remove one entry only. Other additions, including duplicate entries, remain intact.
        for ($index = $parts.Count - 1; $index -ge 0; $index--) {
            if ([string]::Equals($parts[$index], $owned, [StringComparison]::Ordinal)) {
                $parts.RemoveAt($index)
                break
            }
        }
        $owned = ''
    }
    if ($Operation -eq 'Add') {
        $found = $false
        foreach ($part in $parts) {
            if ((Get-ComparablePath $part $expandVariables) -ieq (Get-ComparablePath $Entry)) { $found = $true; break }
        }
        if (-not $found) {
            $parts.Add($Entry)
            $owned = $Entry
            if (-not $exists) { $created = $true }
        }
    }
    $updated = $parts -join ';'
    $oldOwned = $StateKey.GetValue('PathEntry', $null)
    $oldCreated = $StateKey.GetValue('CreatedPath', $null)
    try {
        if ($updated -cne $original) {
            if ($created -and $parts.Count -eq 0) { $EnvironmentKey.DeleteValue('Path', $false) }
            else { $EnvironmentKey.SetValue('Path', $updated, $kind) }
        }
        if ($owned) {
            $StateKey.SetValue('PathEntry', $owned, [Microsoft.Win32.RegistryValueKind]::String)
            $StateKey.SetValue('CreatedPath', [int] $created, [Microsoft.Win32.RegistryValueKind]::DWord)
        } else {
            $StateKey.DeleteValue('PathEntry', $false)
            $StateKey.DeleteValue('CreatedPath', $false)
        }
    } catch {
        # A failed ownership write must not leave an untracked PATH addition behind.
        if ($exists) { $EnvironmentKey.SetValue('Path', $original, $kind) }
        else { $EnvironmentKey.DeleteValue('Path', $false) }
        if ($null -ne $oldOwned) { $StateKey.SetValue('PathEntry', $oldOwned) }
        else { $StateKey.DeleteValue('PathEntry', $false) }
        if ($null -ne $oldCreated) { $StateKey.SetValue('CreatedPath', $oldCreated, [Microsoft.Win32.RegistryValueKind]::DWord) }
        else { $StateKey.DeleteValue('CreatedPath', $false) }
        throw
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    if (-not $Action) { throw 'Action is required.' }
    $machineKey = $null
    $environmentKey = $null
    $stateKey = $null
    try {
        $machineKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64)
        $environmentKey = $machineKey.CreateSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment')
        $stateKey = $machineKey.CreateSubKey('Software\Kagura\Installer')
        Update-KaguraSystemPath $environmentKey $stateKey $Action $InstallDir
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class KaguraEnvironment {
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, UIntPtr wParam,
        string lParam, uint flags, uint timeout, out UIntPtr result);
}
'@
        $result = [UIntPtr]::Zero
        [void] [KaguraEnvironment]::SendMessageTimeout([IntPtr] 0xffff, 0x1a, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref] $result)
    } catch {
        [Console]::Error.WriteLine($_.Exception.Message)
        exit 1
    } finally {
        if ($stateKey) { $stateKey.Dispose() }
        if ($environmentKey) { $environmentKey.Dispose() }
        if ($machineKey) { $machineKey.Dispose() }
    }
}
