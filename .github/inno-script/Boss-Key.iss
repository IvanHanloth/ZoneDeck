; Boss Key 安装脚本
; 由 scripts/package.ps1 -Installer 或 CI 调用：
; MyAppVersion  展示用版本号，可含预发布后缀（3.1.0-rc.1）
; MyAppVersion4 文件资源用四段数字号，必须纯数字（3.1.0.0）
;
; 依赖 package.ps1 先组装好便携文件夹 dist\Boss-Key，安装包的文件与许可协议都取自那里。
; 需要 Inno Setup 7+：简繁中文语言包自 7.0 起才随官方安装包分发（见 scripts/install-inno.ps1）。

#ifndef MyAppVersion
  #define MyAppVersion "3.0.0"
#endif
#ifndef MyAppVersion4
  #define MyAppVersion4 "3.0.0.0"
#endif

#define MyAppName "Boss Key"
#define MyAppPublisher "Ivan Hanloth"
#define MyAppURL "https://github.com/IvanHanloth/Boss-Key"
#define CoreExe "Boss Key.exe"
#define ConfigExe "config.exe"
#define SourceDir "..\..\dist\Boss-Key"

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
; 复用便携版里的那份，避免和仓库根 LICENSE 各自漂移
LicenseFile={#SourceDir}\LICENSE.txt
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
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "chinesetraditional"; MessagesFile: "compiler:Languages\ChineseTraditional.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\{#CoreExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\{#ConfigExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "static\icon.ico"; DestDir: "{app}"; Flags: ignoreversion

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
