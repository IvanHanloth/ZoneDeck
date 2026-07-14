; Boss Key 安装脚本
; 由 scripts/package.ps1 -Installer 或 CI 调用：
; MyAppVersion  展示用版本号，可含预发布后缀（3.1.0-rc.1）
; MyAppVersion4 文件资源用四段数字号，必须纯数字（3.1.0.0）

#ifndef MyAppVersion
  #define MyAppVersion "3.0.0"
#endif
#ifndef MyAppVersion4
  #define MyAppVersion4 "3.0.0.0"
#endif

#define MyAppName "Boss Key"
#define MyAppPublisher "Ivan Hanloth"
#define MyAppURL "https://github.com/IvanHanloth/Boss-Key"
#define CoreExe "core.exe"
#define ConfigExe "Boss Key.exe"
#define SourceDir "..\..\dist"

[Setup]
AppId={{C993A2A8-0714-46E7-A393-DF3F19C43537}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
VersionInfoVersion={#MyAppVersion4}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=static\LICENSE.txt
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\..\dist\installer
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
; 中文语言包不随 Inno Setup 安装包分发，由 scripts/install-inno.ps1 下载到 Languages 目录
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "chinesetraditional"; MessagesFile: "compiler:Languages\ChineseTraditional.isl"
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
