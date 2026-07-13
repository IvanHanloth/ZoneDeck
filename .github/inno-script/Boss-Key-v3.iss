; Boss Key v3（Rust 重写版）安装脚本
; 由 scripts/package.ps1 -Installer 或 CI 调用：
;   ISCC.exe /DMyAppVersion=3.0.0.0 .github\inno-script\Boss-Key-v3.iss
; 安装内容为 package\Boss-Key\ 下的便携产物（先跑 package.ps1）。

#ifndef MyAppVersion
  #define MyAppVersion "3.0.0.0"
#endif

#define MyAppName "Boss-Key"
#define MyAppPublisher "Ivan Hanloth"
#define MyAppURL "https://github.com/IvanHanloth/Boss-Key"
#define CoreExe "bosskey-core.exe"
#define ConfigExe "bosskey-config.exe"
#define SourceDir "..\..\package\Boss-Key"

[Setup]
; v3 使用新的 AppId，避免与 v2（Python 版）安装记录混淆
AppId={{7D6E2A41-9C93-4B7A-B0E3-52D6A4C1F8B2}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=static\LICENSE.txt
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\..\package\installer
OutputBaseFilename=Boss-Key-{#MyAppVersion}-Setup
SetupIconFile=static\icon.ico
UninstallDisplayIcon={app}\{#CoreExe}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
; 关闭正在运行的核心/配置程序后再安装
CloseApplications=yes
CloseApplicationsFilter=*.exe

[Languages]
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\{#CoreExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\{#ConfigExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#CoreExe}"
Name: "{group}\{#MyAppName} 设置"; Filename: "{app}\{#ConfigExe}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#CoreExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#CoreExe}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; 卸载前结束常驻核心，避免文件占用
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#CoreExe}"; Flags: runhidden; RunOnceId: "KillCore"

[Code]
// 安装前结束正在运行的核心（无窗口进程，CloseApplications 无法关闭它）
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#CoreExe}', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#ConfigExe}', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := '';
end;
