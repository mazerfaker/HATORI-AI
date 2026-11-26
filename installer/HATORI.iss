#ifndef AppVersion
#define AppVersion "0.2.5"
#endif
[Setup]
AppId=HATORI-AI-Desktop
AppName=HATORI AI
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\HATORI AI
DefaultGroupName=HATORI AI
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=HATORI-AI-{#AppVersion}-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\hatori_ai.exe
CloseApplications=yes
[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked
Name: "autostart"; Description: "Start HATORI with Windows"; Flags: unchecked
[Files]
Source: "..\target\release\hatori_ai.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\runtime\*"; DestDir: "{app}\runtime"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
[Icons]
Name: "{userprograms}\HATORI AI"; Filename: "{app}\hatori_ai.exe"
Name: "{userdesktop}\HATORI AI"; Filename: "{app}\hatori_ai.exe"; Tasks: desktopicon
Name: "{userstartup}\HATORI AI"; Filename: "{app}\hatori_ai.exe"; Tasks: autostart
[Run]
Filename: "{app}\hatori_ai.exe"; Description: "Launch HATORI AI"; Flags: nowait postinstall skipifsilent
[UninstallDelete]
Type: files; Name: "{userstartup}\HATORI AI.lnk"
