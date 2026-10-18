; installer/ferrisfilter.iss -- Inno Setup 6 script for the per-user FerrisFilter installer.
;
; ASCII only: Inno 6 reads a file without a BOM as ANSI, so keep every character plain.
;
; Do not run ISCC on this file by hand. installer/package.sh passes the defines:
;   /DAppVersion=<X.Y.Z[-pre]>   shown in Windows, used in the setup file name
;   /DAppNumVersion=<X.Y.Z>      numeric only, for the setup.exe version resource
;   /DAppExe=<full Windows path of the ff.exe to package>
;
; The AppId below must never change. Inno keys the per-user uninstall entry on it, so a
; different value would make an upgrade install beside the old copy instead of over it.
; It differs on purpose from the instance name used by the overlay.
;
; There is deliberately no mutex-based "close the application" prompt: it would nag before
; PrepareToInstall can ask the running ff to quit by itself. CloseApplications stays as the
; Restart Manager backstop, with restarting of applications off so ff is never started twice (the
; Launch entry on the last page starts it).
;
; Uninstall removes the program and the shortcuts and keeps %APPDATA%\FerrisFilter.

#ifndef AppVersion
  #error Pass /DAppVersion=X.Y.Z[-pre]
#endif
#ifndef AppNumVersion
  #error Pass /DAppNumVersion=X.Y.Z
#endif
#ifndef AppExe
  #error Pass /DAppExe=<full path of ff.exe>
#endif

[Setup]
AppId={{1FDA92ED-8F71-4558-B482-3EC93A7AA550}
AppName=FerrisFilter
AppVersion={#AppVersion}
AppVerName=FerrisFilter {#AppVersion}
AppPublisher=Jonathas Conceicao
AppPublisherURL=https://github.com/Jonathas-Conceicao/ferris-filter
AppSupportURL=https://github.com/Jonathas-Conceicao/ferris-filter/issues
AppUpdatesURL=https://github.com/Jonathas-Conceicao/ferris-filter/releases
VersionInfoVersion={#AppNumVersion}
VersionInfoProductTextVersion={#AppVersion}
DefaultDirName={autopf}\FerrisFilter
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=FerrisFilter-{#AppVersion}-setup
SetupIconFile=..\assets\ferris\ferris.ico
UninstallDisplayIcon={app}\ff.exe
UninstallDisplayName=FerrisFilter
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Dirs]
Name: "{userappdata}\FerrisFilter"; Flags: uninsneveruninstall

[Files]
Source: "{#AppExe}"; DestDir: "{app}"; DestName: "ff.exe"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\FerrisFilter"; Filename: "{app}\ff.exe"
Name: "{autodesktop}\FerrisFilter"; Filename: "{app}\ff.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\ff.exe"; Description: "{cm:LaunchProgram,FerrisFilter}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\ff.exe"; Parameters: "--quit"; Flags: waituntilterminated runhidden; RunOnceId: "QuitFerrisFilter"

[Code]
{ Asks an installed, running ff to quit through its own quit path (the Unapplied changes
  popup if there are edits). Exit code 0 = quit or not running, 1 = still running after the
  wait, 2 = could not reach it (for example an elevated instance). }
function QuitRunningFerrisFilter(): Boolean;
var
  Exe: String;
  Code: Integer;
begin
  Result := True;
  Exe := ExpandConstant('{app}\ff.exe');
  { Fresh install: nothing of ours can be running from {app}. }
  if not FileExists(Exe) then exit;
  repeat
    { Exec failure: carry on, Restart Manager is the backstop. }
    if not Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, Code) then exit;
    { 0 = quit or not running, 2 = unreachable: both continue. }
    if Code <> 1 then exit;
  until SuppressibleMsgBox('FerrisFilter is still running. If it is asking about unapplied ' +
      'changes, answer it, then click Retry.', mbError, MB_RETRYCANCEL, IDCANCEL) <> IDRETRY;
  Result := False;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  if QuitRunningFerrisFilter() then
    Result := ''
  else
    Result := 'FerrisFilter is still running. Quit it from its tray icon, then run Setup again.';
end;

{ Uninstall asks a running ff to quit the same way before any file is removed, so ff.exe is
  never left running in a half-removed install. The [UninstallRun] entry then finds nothing
  running. }
function InitializeUninstall(): Boolean;
begin
  Result := QuitRunningFerrisFilter();
  if not Result then
    SuppressibleMsgBox('FerrisFilter is still running. Quit it from its tray icon, then ' +
      'uninstall again.', mbError, MB_OK, IDOK);
end;
