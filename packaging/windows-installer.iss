; Inno Setup 6 script for Canalis's Windows installer, canalis-setup-v<version>.exe:
;   iscc /DAppVersion=0.4.0 /DSourceDir=<folder with canalis.exe and the scripts> /DIconFile=<canalis.ico> /DOutputDir=<folder> windows-installer.iss
; It installs for the current user without administrator rights, then runs install-windows.ps1
; for the rest: adb, LibreHardwareMonitor, start at login, and turning off KANALI's autostart.

[Setup]
AppId={{8F3B6C2E-5D4A-4C1B-9E7F-2A6D0C9B1E43}
AppName=Canalis
AppVersion={#AppVersion}
AppPublisherURL=https://github.com/sallen2/canalis
DefaultDirName={localappdata}\Programs\Canalis
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename=canalis-setup-v{#AppVersion}
SetupIconFile={#IconFile}
UninstallDisplayIcon={app}\canalis.exe
UninstallDisplayName=Canalis
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
CloseApplications=force
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "{#SourceDir}\canalis.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\install-windows.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\uninstall-windows.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\packaging\*"; DestDir: "{app}\packaging"; Flags: ignoreversion

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\install-windows.ps1"""; StatusMsg: "Setting up adb, LibreHardwareMonitor and start at login…"; Flags: waituntilterminated
Filename: "{app}\canalis.exe"; Description: "Open Canalis"; Flags: postinstall nowait skipifsilent

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\uninstall-windows.ps1"" -FromUninstaller"; RunOnceId: "RemoveCanalis"; Flags: waituntilterminated
