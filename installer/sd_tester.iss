; Inno Setup script for sd_tester
; Generates a Windows installer EXE that installs sd_tester.exe with start menu shortcut.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

#ifndef SourceExe
  #define SourceExe "..\target\release\sd_tester.exe"
#endif

#ifndef OutputDir
  #define OutputDir "..\dist"
#endif

#ifndef OutputBaseName
  #define OutputBaseName "sd_tester-installer"
#endif

[Setup]
AppId={{F2B8E248-6BC7-4D9F-AB44-5A9F6C8F5A9F}
AppName=SD/USB Native Tester
AppVersion={#MyAppVersion}
AppPublisher=sd_tester contributors
AppPublisherURL=https://github.com/hibyemy/sd_tester
DefaultDirName={autopf}\sd_tester
DefaultGroupName=SD/USB Native Tester
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
UninstallDisplayName=SD/USB Native Tester

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "sd_tester.exe"; Flags: ignoreversion

[Icons]
Name: "{group}\SD/USB Native Tester"; Filename: "{app}\sd_tester.exe"
Name: "{group}\Uninstall SD/USB Native Tester"; Filename: "{uninstallexe}"
Name: "{userdesktop}\SD/USB Native Tester"; Filename: "{app}\sd_tester.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\sd_tester.exe"; Description: "Launch SD/USB Native Tester"; Flags: nowait postinstall skipifsilent
