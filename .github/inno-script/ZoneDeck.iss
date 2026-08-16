; ZoneDeck 安装脚本
; 由 scripts/package.ps1 -Installer 或 CI 调用：
; MyAppVersion  展示用版本号，可含预发布后缀（3.1.0-rc.1）
; MyAppVersion4 文件资源用四段数字号，必须纯数字（3.1.0.0）
;
; 依赖 package.ps1 先组装好便携文件夹 dist\ZoneDeck，安装包的文件与许可协议都取自那里。
; 需要 Inno Setup 7+：简繁中文语言包自 7.0 起才随官方安装包分发（见 scripts/install-inno.ps1）。

; 不设默认值：写死的版本号迟早会过期，装出一个版本号对不上的包比编译失败更难发现。
#ifndef MyAppVersion
  #error "缺少 MyAppVersion：请用 scripts/package.ps1 -Installer 编译，由它从 Cargo.toml 取版本号传入"
#endif
#ifndef MyAppVersion4
  #error "缺少 MyAppVersion4：请用 scripts/package.ps1 -Installer 编译"
#endif

#define MyAppName "ZoneDeck"
#define MyAppPublisher "Ivan Hanloth"
#define MyAppURL "https://github.com/IvanHanloth/ZoneDeck"
#define CoreExe "ZoneDeck.exe"
; 改名（Boss Key → ZoneDeck）前的核心映像名，升级时用于结束旧进程并删除旧文件
#define LegacyCoreExe "Boss Key.exe"
#define LegacyAppName "Boss Key"
#define ConfigExe "config.exe"
#define SourceDir "..\..\dist\ZoneDeck"
; 文件名须与 crates/common/src/paths.rs 的 INSTALLED_MARKER 一致
#define InstalledMarker "installed.marker"

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
; 默认普通权限安装（{autopf} 此时为 %LocalAppData%\Programs）：不必为装个隐藏窗口的小工具
; 弹 UAC。仍允许在启动对话框改选「为所有用户安装」装进 Program Files。
; 两种模式下数据都在 %APPDATA%\ZoneDeck，与安装目录无关，见 crates/common/src/paths.rs。
; 升级时 Inno 沿用上次的安装模式（UsePreviousPrivileges 默认开），旧的按机器安装原地升级。
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\..\dist\installer
OutputBaseFilename=ZoneDeck-{#MyAppVersion}-Setup
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
chinesesimplified.KeepConfigPrompt=是否保留配置文件（config.json）？%n%n选择“是”将保留你的设置，重新安装后可继续使用；%n选择“否”将删除包括配置文件在内的全部数据。
chinesetraditional.KeepConfigPrompt=是否保留設定檔（config.json）？%n%n選擇「是」將保留你的設定，重新安裝後可繼續使用；%n選擇「否」將刪除包括設定檔在內的全部資料。
english.KeepConfigPrompt=Do you want to keep your settings file (config.json)?%n%nChoose "Yes" to keep your settings for a future reinstall;%nchoose "No" to delete all data, including the settings file.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

; 旧品牌安装的残留：同 AppId 原地升级不会替换掉旧文件名的 exe 与旧名称的快捷方式，须显式删除。
[InstallDelete]
Type: files; Name: "{app}\{#LegacyCoreExe}"
Type: files; Name: "{autodesktop}\{#LegacyAppName}.lnk"
Type: filesandordirs; Name: "{autoprograms}\{#LegacyAppName}"

[Files]
Source: "{#SourceDir}\{#CoreExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\{#ConfigExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "static\icon.ico"; DestDir: "{app}"; Flags: ignoreversion
; 安装版标记：程序据它把数据存到 %APPDATA%\ZoneDeck 而非安装目录（见 crates/common/src/paths.rs）。
; 不放在便携文件夹 dist\ZoneDeck 里，故直接从脚本目录取——便携版有了它就不便携了。
Source: "{#InstalledMarker}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#CoreExe}"
Name: "{group}\{#MyAppName} 设置"; Filename: "{app}\{#ConfigExe}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#CoreExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#CoreExe}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// 自启残留项，须与 crates/core/src/autostart.rs 中的常量保持一致。
// Legacy* 为改名（Boss Key → ZoneDeck）前的名称，仅用于清理旧残留。
const
  AutostartTaskName = 'ZoneDeckAutostart';
  LegacyAutostartTaskName = 'BossKeyAutostart';
  RunSubkey = 'Software\Microsoft\Windows\CurrentVersion\Run';
  RunValueName = 'ZoneDeck Application';
  LegacyRunValueName = 'Boss Key Application';
  // 自启迁移标记，须与 autostart.rs 的 MIGRATION_MARKER_* 一致：安装前必须删掉旧
  // 看门狗任务，核心首启时旧残留已不在，只能凭此标记得知用户此前开着自启并重建。
  MigrationMarkerSubkey = 'Software\ZoneDeck';
  MigrationMarkerValue = 'MigrateAutostart';
  // 配置界面（Tauri）的 WebView2 用户数据目录名，等于 tauri.conf.json 里的 identifier。
  WebViewDataDirName = 'cn.hanloth.zonedeck.config';
  LegacyWebViewDataDirName = 'cn.hanloth.bosskey.config';

// 强制结束核心与配置进程（新旧映像名都杀，升级时旧核心仍在运行）。
// 核心是无窗口常驻进程，CloseApplications 关不掉它；
// 且旧映像名 "Boss Key.exe" 含空格，taskkill 的 /IM 值必须加引号，否则参数被拆断而失败。
procedure KillRunningApps;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM "{#CoreExe}"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM "{#LegacyCoreExe}"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM "{#ConfigExe}"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// 清理旧品牌的自启残留。安装与卸载都要做：升级会删掉旧 exe，旧任务再触发只会空转报错。
procedure RemoveLegacyAutostart;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\schtasks.exe'),
    '/Delete /F /TN "' + LegacyAutostartTaskName + '"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode);
  RegDeleteValue(HKEY_CURRENT_USER, RunSubkey, LegacyRunValueName);
end;

// 旧品牌的自启注册（计划任务或 Run 值）是否存在。
function LegacyAutostartPresent: Boolean;
var
  ResultCode: Integer;
  Value: string;
begin
  Result := RegQueryStringValue(HKEY_CURRENT_USER, RunSubkey, LegacyRunValueName, Value);
  if Result then Exit;
  Result := Exec(ExpandConstant('{sys}\schtasks.exe'),
    '/Query /TN "' + LegacyAutostartTaskName + '"', '',
    SW_HIDE, ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
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
  RemoveLegacyAutostart;
  // 未被核心消费的迁移标记一并清掉（装完从未运行核心就卸载的情形）。
  RegDeleteValue(HKEY_CURRENT_USER, MigrationMarkerSubkey, MigrationMarkerValue);
  RegDeleteKeyIfEmpty(HKEY_CURRENT_USER, MigrationMarkerSubkey);
end;

// 安装前结束正在运行的核心（无窗口进程，CloseApplications 无法关闭它），并摘掉
// 旧品牌的自启看门狗——它指向的旧 exe 即将被删除。删之前若发现旧自启存在，先写
// 迁移标记，核心首次启动据它按用户偏好在新名称下重建（autostart.rs migrate_legacy）。
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  if LegacyAutostartPresent then
    RegWriteStringValue(HKEY_CURRENT_USER, MigrationMarkerSubkey, MigrationMarkerValue, '1');
  RemoveLegacyAutostart;
  KillRunningApps;
  Result := '';
end;

// 清理一个数据目录里的运行时产物（日志、恢复文件、缓存、写入残留），配置文件除外。
// config.json.tmp 是原子保存的中间文件，写到一半崩溃会留下；
// .ZoneDeck-write-probe-* 是可写性探测的探针文件，进程被强杀时会留下（旧版前缀为 .BossKey-）。
// 二者都不该留在磁盘上，且留着会让空目录删不掉。
procedure RemoveRuntimeFiles(Dir: string);
begin
  DelTree(Dir + '\logs', True, True, True);
  DeleteFile(Dir + '\recovery.json');
  DeleteFile(Dir + '\verhub_cache.json');
  DeleteFile(Dir + '\config.json.tmp');
  DelTree(Dir + '\.ZoneDeck-write-probe-*', False, True, False);
  DelTree(Dir + '\.BossKey-write-probe-*', False, True, False);
end;

// 清理配置界面的 WebView2 用户数据目录（%LOCALAPPDATA%\<identifier>\EBWebView）。
// 它不在数据目录里，位置由 Tauri 按 identifier 决定，不删会残留几十 MB 的浏览器缓存。
// 里面只有缓存与本地存储，用户设置在 config.json 中，故不受「是否保留配置」影响。
procedure RemoveWebViewData;
begin
  DelTree(ExpandConstant('{localappdata}\' + WebViewDataDirName), True, True, True);
  DelTree(ExpandConstant('{localappdata}\' + LegacyWebViewDataDirName), True, True, True);
end;

// 卸载时清理：
// - usUninstall（删文件前）：摘掉自启看门狗并结束进程，确保核心不会被重新拉起、文件不被占用。
// - usPostUninstall（删文件后）：清理运行时产物（日志、恢复文件、缓存、WebView2 用户数据），
//   并询问是否保留配置文件；保留则只留下 config.json，不保留则连同整个目录一起删除。
//   静默卸载不弹窗，默认保留配置。
//
// 安装版的数据在 %APPDATA%\ZoneDeck（见 crates/common/src/paths.rs）。安装目录里也扫一遍：
// 早期版本把数据放在那里，迁移时删不掉的原文件会留下。
//
// 提权卸载（按机器安装）时 {userappdata} / {localappdata} 指向执行卸载的账户：与当初安装的是
// 同一账户（UAC 提权自己）时正确，由另一个管理员账户授权时则指向错误的用户目录，那里找不到
// 文件、什么也不会删。宁可留下也不去遍历所有用户目录误删别人的数据。
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir, UserDir, LegacyUserDir: string;
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
  UserDir := ExpandConstant('{userappdata}\ZoneDeck');
  // 旧品牌数据目录：程序启动时会自动迁走，但迁移被占用挡下时它还在原处。
  LegacyUserDir := ExpandConstant('{userappdata}\BossKey');
  RemoveRuntimeFiles(AppDir);
  RemoveRuntimeFiles(UserDir);
  RemoveRuntimeFiles(LegacyUserDir);
  RemoveWebViewData;

  if FileExists(AppDir + '\config.json') or FileExists(UserDir + '\config.json') then
  begin
    KeepConfig := UninstallSilent or
      (MsgBox(CustomMessage('KeepConfigPrompt'), mbConfirmation, MB_YESNO) = IDYES);
    if not KeepConfig then
    begin
      DelTree(AppDir, True, True, True);
      DelTree(UserDir, True, True, True);
      DelTree(LegacyUserDir, True, True, True);
      Exit;
    end;
  end;

  // 只在目录已空时收尾，留着配置的目录会被跳过。
  RemoveDir(AppDir);
  RemoveDir(UserDir);
  RemoveDir(LegacyUserDir);
end;
