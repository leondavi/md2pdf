; Inno Setup script for the md2pdf Windows installer.
;
; Build with:
;   ISCC.exe /DMyAppVersion=0.1.0 ^
;            /DBinDir=target\x86_64-pc-windows-msvc\release ^
;            /DRepoDir=. ^
;            /DIconFile=assets\logo.ico ^
;            /DOutDir=dist ^
;            packaging\windows\md2pdf.iss
;
; Produces:  dist\md2pdf-<version>-windows-x64-setup.exe

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef BinDir
  #define BinDir "..\..\target\x86_64-pc-windows-msvc\release"
#endif
#ifndef RepoDir
  #define RepoDir "..\.."
#endif
#ifndef IconFile
  #define IconFile "..\..\assets\logo.ico"
#endif
#ifndef OutDir
  #define OutDir "..\..\dist"
#endif

#define MyAppName "md2pdf"
#define MyAppPublisher "David Leon"
#define MyAppURL "https://github.com/leondavi/md2pdf"
#define MyGuiExe "md2pdf-gui.exe"
#define MyCliExe "md2pdf.exe"

[Setup]
AppId={{6F2C9A1E-4B7A-4C2E-9C2B-2D9F0B7E2A10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#MyGuiExe}
LicenseFile={#RepoDir}\LICENSE
OutputDir={#OutDir}
OutputBaseFilename=md2pdf-{#MyAppVersion}-windows-x64-setup
SetupIconFile={#IconFile}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"
Name: "addtopath"; Description: "Add the md2pdf command-line tool to PATH"; GroupDescription: "Command line:"

[Files]
Source: "{#BinDir}\{#MyGuiExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\{#MyCliExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#RepoDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyGuiExe}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyGuiExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyGuiExe}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Registry]
; Optionally add the install dir to the user PATH (for the CLI).
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; \
    ValueData: "{olddata};{app}"; Tasks: addtopath; \
    Check: NeedsAddPath('{app}')

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  // Only add if not already present.
  Result := Pos(';' + ExpandConstant(Param) + ';', ';' + OrigPath + ';') = 0;
end;
