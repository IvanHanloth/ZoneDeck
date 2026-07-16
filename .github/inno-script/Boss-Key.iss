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
AppId={{BA8E9784-B92D-48EE-B447-99709232260B}
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

[CustomMessages]
chinesesimplified.KeepConfigPrompt=是否保留配置文件（config.json）？%n%n选择“是”将保留你的设置，重新安装后可继续使用；%n选择“否”将删除包括配置文件在内的整个安装目录。
chinesetraditional.KeepConfigPrompt=是否保留設定檔（config.json）？%n%n選擇「是」將保留你的設定，重新安裝後可繼續使用；%n選擇「否」將刪除包括設定檔在內的整個安裝目錄。
english.KeepConfigPrompt=Do you want to keep your settings file (config.json)?%n%nChoose "Yes" to keep your settings for a future reinstall;%nchoose "No" to delete the entire installation folder, including the settings file.

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

[Code]
// 自启残留项，须与 crates/core/src/autostart.rs 中的常量保持一致。
const
  AutostartTaskName = 'BossKeyAutostart';
  RunSubkey = 'Software\Microsoft\Windows\CurrentVersion\Run';
  RunValueName = 'Boss Key Application';

// 强制结束核心与配置进程。核心是无窗口常驻进程，CloseApplications 关不掉它；
// 且映像名 "Boss Key.exe" 含空格，taskkill 的 /IM 值必须加引号，否则参数被拆断而失败。
procedure KillRunningApps;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM "{#CoreExe}"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM "{#ConfigExe}"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// 清理自启残留：先删计划任务，否则任务里的失败自动重启（看门狗）会在卸载途中
// 把核心重新拉起、重新占用文件；再删注册表启动项。
procedure RemoveAutostart;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\schtasks.exe'),
    '/Delete /F /TN "' + AutostartTaskName + '"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  RegDeleteValue(HKEY_CURRENT_USER, RunSubkey, RunValueName);
end;

// 安装前结束正在运行的核心（无窗口进程，CloseApplications 无法关闭它）
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  KillRunningApps;
  Result := '';
end;

// 卸载时清理：
// - usUninstall（删文件前）：摘掉自启看门狗并结束进程，确保核心不会被重新拉起、文件不被占用。
// - usPostUninstall（删文件后）：清理运行时产物（日志、恢复文件），并询问是否保留配置文件；
//   保留则只留下 config.json，不保留则连同整个安装目录一起删除。静默卸载不弹窗，默认保留配置。
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: string;
  KeepConfig: Boolean;
begin
  if CurUninstallStep = usUninstall then
  begin
    RemoveAutostart;
    KillRunningApps;
    Exit;
  end;

  if CurUninstallStep <> usPostUninstall then
    Exit;
  AppDir := ExpandConstant('{app}');
  DelTree(AppDir + '\logs', True, True, True);
  DeleteFile(AppDir + '\recovery.json');
  if FileExists(AppDir + '\config.json') then
  begin
    KeepConfig := UninstallSilent or
      (MsgBox(CustomMessage('KeepConfigPrompt'), mbConfirmation, MB_YESNO) = IDYES);
    if not KeepConfig then
      DelTree(AppDir, True, True, True);
  end
  else
    RemoveDir(AppDir);
end;
