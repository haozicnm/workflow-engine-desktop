; Inno Setup script for Workflow Engine Windows installer
; Usage: iscc installer.iss

#define MyAppName "Workflow Engine"
#define MyAppVersion GetEnv("APP_VERSION")
#define MyAppPublisher "Workflow Engine"
#define MyAppURL "https://github.com/haozicnm/workflow-engine-desktop"
#define MyAppExeName "workflow-engine.exe"

[Setup]
AppId={{4A8F3D2E-1B5C-4D9A-8E2F-7C6B5A3D1E0F}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=LICENSE
OutputDir=.
OutputBaseFilename=WorkflowEngine-{#MyAppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "chinese"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Main executables
Source: "{src}\workflow-engine.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{src}\wf-cli.exe"; DestDir: "{app}"; Flags: ignoreversion

; Frontend dist
Source: "{src}\dist\*"; DestDir: "{app}\dist"; Flags: ignoreversion recursesubdirs createallsubdirs

; Sidecars (Python runtime, Playwright browsers, etc.)
Source: "{src}\sidecars\*"; DestDir: "{app}\sidecars"; Flags: ignoreversion recursesubdirs createallsubdirs

; Node schema
Source: "{src}\node-schema.json"; DestDir: "{app}"; Flags: ignoreversion

; Python runtime
Source: "{src}\python-runtime\*"; DestDir: "{app}\python-runtime"; Flags: ignoreversion recursesubdirs createallsubdirs

; Start script
Source: "{src}\start.bat"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\start.bat"; WorkingDir: "{app}"
Name: "{group}\{cm:ProgramOnTheWeb,{#MyAppName}}"; Filename: "{#MyAppURL}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\start.bat"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\start.bat"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
