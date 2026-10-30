#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef BinaryPath
  #error BinaryPath is required
#endif
#ifndef SourceRoot
  #error SourceRoot is required
#endif
#ifndef OutputRoot
  #error OutputRoot is required
#endif

[Setup]
AppId={{72FD7323-E0F6-486F-9CD0-0C08A45BA16A}
AppName=Kagura
AppVersion={#AppVersion}
AppPublisher=SUPER SHRINE
DefaultDirName={autopf}\SUPER-SHRINE\Kagura
UsePreviousAppDir=yes
UsePreviousTasks=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\kagura.exe
OutputDir={#OutputRoot}
OutputBaseFilename=kagura-{#AppVersion}-x86_64-pc-windows-msvc-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
LicenseFile={#SourceRoot}\LICENSE.md

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
japanese.AddToPath=KaguraをシステムのPATHに追加する
english.AddToPath=Add Kagura to the system PATH
japanese.PathFailed=システムのPATHを更新できませんでした。ログを確認してください。
english.PathFailed=Could not update the system PATH. Check the log for details.
japanese.PathInvalid=PATHに追加する配置先にはセミコロン、二重引用符、パーセント記号を使用できません。
english.PathInvalid=An installation path added to PATH cannot contain semicolons, double quotes, or percent signs.
japanese.UpgradeFailed=以前のKaguraを削除できないため、更新を中止しました。ログを確認し、削除を完了してから再実行してください。
english.UpgradeFailed=The update stopped because the previous Kagura installation could not be removed. Check the log and complete its removal before trying again.
japanese.UpgradeRestart=以前のKaguraの削除を完了するには、Windowsを再起動してからインストーラーを再実行してください。
english.UpgradeRestart=Restart Windows to complete removal of the previous Kagura installation, then run Setup again.

[Tasks]
Name: "systempath"; Description: "{cm:AddToPath}"

[Files]
Source: "{#BinaryPath}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceRoot}\LICENSE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceRoot}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceRoot}\specs\*"; DestDir: "{app}\specs"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#SourceRoot}\packaging\windows\system-path.ps1"; DestDir: "{app}\.installer"; Flags: ignoreversion

[Code]
var
  PathUpdateFailed: Boolean;
  PreviousUninstallComplete: Boolean;
  PreviousUninstallError: String;

function RemovePreviousInstallation(var NeedsRestart: Boolean): String;
var
  UninstallKey, UninstallCommand, Uninstaller: String;
  ExitCode: Integer;
begin
  Result := PreviousUninstallError;
  if PreviousUninstallComplete or (Result <> '') then Exit;
  UninstallKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\' +
    '{72FD7323-E0F6-486F-9CD0-0C08A45BA16A}_is1';
  if not RegKeyExists(HKLM64, UninstallKey) then begin
    if FileExists(ExpandConstant('{app}\kagura.exe')) or
      FileExists(ExpandConstant('{app}\.installer\system-path.ps1')) then
      Result := CustomMessage('UpgradeFailed');
    Exit;
  end;
  Result := CustomMessage('UpgradeFailed');
  if not RegQueryStringValue(HKLM64, UninstallKey, 'UninstallString', UninstallCommand) then Exit;
  Uninstaller := RemoveQuotes(UninstallCommand);
  if ((UninstallCommand <> Uninstaller) and (UninstallCommand <> '"' + Uninstaller + '"')) or
    (Pos('"', Uninstaller) > 0) or (CompareText(ExpandFileName(Uninstaller), Uninstaller) <> 0) or
    (CompareText(ExtractFileExt(Uninstaller), '.exe') <> 0) or not FileExists(Uninstaller) then begin
    Log('Previous uninstall registration is invalid: ' + UninstallCommand);
    Exit;
  end;
  PreviousUninstallError := Result;
  if not Exec(Uninstaller, '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /RESTARTEXITCODE=3010',
    '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then begin
    Log('Could not launch previous uninstaller: ' + IntToStr(ExitCode));
    Exit;
  end;
  Log('Previous uninstaller exit=' + IntToStr(ExitCode));
  if ExitCode = 3010 then begin
    NeedsRestart := True;
    PreviousUninstallError := CustomMessage('UpgradeRestart');
    Result := PreviousUninstallError;
    Exit;
  end;
  if (ExitCode <> 0) or RegKeyExists(HKLM64, UninstallKey) then Exit;
  PreviousUninstallComplete := True;
  PreviousUninstallError := '';
  Result := '';
end;

function UpdateSystemPath(Action: String): Boolean;
var
  ExitCode: Integer;
  Parameters: String;
begin
  Parameters := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File ' +
    AddQuotes(ExpandConstant('{app}\.installer\system-path.ps1')) +
    ' -Action ' + Action + ' -InstallDir ' + AddQuotes(ExpandConstant('{app}'));
  Result := Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    Parameters, '', SW_HIDE, ewWaitUntilTerminated, ExitCode);
  if Result then Result := ExitCode = 0;
  Log('PATH helper: action=' + Action + ', exit=' + IntToStr(ExitCode));
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('systempath') then
    if not UpdateSystemPath('Add') then begin
      PathUpdateFailed := True;
      RaiseException(CustomMessage('PathFailed'));
    end;
end;

function GetCustomSetupExitCode(): Integer;
begin
  if PathUpdateFailed then Result := 1 else Result := 0;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if WizardIsTaskSelected('systempath') and
    ((Pos(';', ExpandConstant('{app}')) > 0) or
     (Pos('"', ExpandConstant('{app}')) > 0) or
     (Pos('%', ExpandConstant('{app}')) > 0)) then begin
    Result := CustomMessage('PathInvalid');
    Exit;
  end;
  Result := RemovePreviousInstallation(NeedsRestart);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usUninstall) and not UpdateSystemPath('Remove') then begin
    SuppressibleMsgBox(CustomMessage('PathFailed'), mbError, MB_OK, IDOK);
    Abort;
  end;
end;
