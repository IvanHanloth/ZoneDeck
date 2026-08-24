use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use zonedeck_common::Config;
use zonedeck_common::ipc::{Command, PipeClient, Response};
use zonedeck_common::model::WindowInfo;
use zonedeck_core::i18n::{self, Msg};
use zonedeck_core::key_capture::{KeyCapture, KeyEvent};

mod verhub;

const CORE_EXE: &str = "ZoneDeck.exe";

/// 程序自身所在目录：只用来找同目录下的可执行文件（核心、pssuspend）。
/// 数据文件一律走 [`zonedeck_common::paths`]。
fn exe_dir() -> PathBuf {
    zonedeck_common::paths::exe_dir()
}

/// 数据目录（配置、日志、恢复文件、缓存）。
fn data_dir() -> PathBuf {
    zonedeck_common::paths::data_dir()
}

fn config_path() -> PathBuf {
    zonedeck_common::paths::config_path()
}

fn log_dir() -> PathBuf {
    data_dir().join(zonedeck_core::logging::LOG_DIR_NAME)
}

/// 定位同目录下的核心程序，不存在时报错。
fn core_exe_path() -> Result<PathBuf, String> {
    let exe = exe_dir().join(CORE_EXE);
    if exe.exists() {
        Ok(exe)
    } else {
        Err(i18n::tf(Msg::ErrCoreExeMissing, &[("exe", CORE_EXE)]))
    }
}

/// 以脱离进程方式（无控制台、不随本程序退出）启动核心。
fn spawn_core_detached(exe: &Path, arg: Option<&str>) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new(exe);
    if let Some(a) = arg {
        cmd.arg(a);
    }
    cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn notify_core(command: &Command) -> Result<Response, String> {
    PipeClient::connect_default()
        .send(command)
        .map_err(|e| e.to_string())
}

/// 把阻塞工作丢到专用线程，避免阻塞 Tauri 异步运行时。
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .expect("阻塞任务执行失败")
}

/// 数据目录的位置与由来，供界面在便携版回退时提示用户。
#[derive(Serialize)]
struct DataLocation {
    dir: String,
    program_dir: String,
    /// `installed` / `portable` / `portable_fallback`。
    kind: &'static str,
}

#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    website: &'static str,
    author: &'static str,
    email: &'static str,
    blog: &'static str,
    license: &'static str,
}

#[derive(Serialize)]
struct LoadedConfig {
    config: Config,
    /// 解析失败回退默认值时的原因（含备份去向）；正常加载为 `None`。
    fallback: Option<String>,
}

#[tauri::command]
fn load_config() -> Result<LoadedConfig, String> {
    let (config, fallback) = Config::load_reporting(&config_path()).map_err(|e| e.to_string())?;
    i18n::set_from_pref(&config.setting.language);
    Ok(LoadedConfig { config, fallback })
}

/// 入参用 JSON 值而非 `Config`：经 [`Config::from_value`] 剥离 `null` 后再反序列化，
/// 界面上被清空的输入框不会导致整份配置保存失败。
#[tauri::command]
async fn save_config(config: serde_json::Value) -> Result<(), String> {
    blocking(move || {
        let config = Config::from_value(config).map_err(|e| e.to_string())?;
        i18n::set_from_pref(&config.setting.language);
        config.save(&config_path()).map_err(|e| e.to_string())?;
        // 通知核心热重载；核心未运行时忽略错误。
        let _ = notify_core(&Command::ReloadConfig);
        Ok(())
    })
    .await
}

#[tauri::command]
async fn list_windows() -> Vec<WindowInfo> {
    blocking(|| {
        use zonedeck_core::platform::WindowManager;
        zonedeck_core::platform::manager().enumerate()
    })
    .await
}

/// 批量提取可执行文件图标，返回 `路径 → PNG data URI`。
/// 没有图标或文件不存在的路径不出现在结果里。
#[tauri::command]
async fn window_icons(paths: Vec<String>) -> std::collections::HashMap<String, String> {
    blocking(move || {
        let mut icons = std::collections::HashMap::new();
        for path in paths {
            if path.is_empty() || icons.contains_key(&path) {
                continue;
            }
            if let Some(uri) = zonedeck_core::icon::icon_data_uri(&path) {
                icons.insert(path, uri);
            }
        }
        icons
    })
    .await
}

#[tauri::command]
async fn show_all_windows() -> Result<(), String> {
    blocking(|| notify_core(&Command::Show).map(|_| ())).await
}

/// 把恢复工具的窗口操作交给核心执行；核心未应答时返回 false，由调用方退回直接操作。
fn try_core_window_op(command: &Command) -> bool {
    matches!(
        PipeClient::connect_default().fast().send(command),
        Ok(Response::Ok)
    )
}

/// 恢复显示指定窗口（窗口恢复工具）；核心不在运行才直接对句柄 ShowWindow。
#[tauri::command]
async fn show_windows(hwnds: Vec<i64>) {
    blocking(move || {
        if try_core_window_op(&Command::ReleaseWindows {
            hwnds: hwnds.clone(),
        }) {
            return;
        }
        use zonedeck_core::platform::WindowManager;
        let mgr = zonedeck_core::platform::manager();
        for h in hwnds {
            mgr.show(h);
        }
    })
    .await
}

/// 隐藏指定窗口（窗口恢复工具），不施加静音 / 冻结；核心不在运行才直接隐藏。
#[tauri::command]
async fn hide_windows(hwnds: Vec<i64>) {
    blocking(move || {
        if try_core_window_op(&Command::AdoptWindows {
            hwnds: hwnds.clone(),
        }) {
            return;
        }
        use zonedeck_core::platform::WindowManager;
        let mgr = zonedeck_core::platform::manager();
        for h in hwnds {
            mgr.hide(h);
        }
    })
    .await
}

/// 冻结指定进程（窗口恢复工具）。`scope` 为作用范围（`self` / `tree` / `image`），
/// `enhanced` 为真且 pssuspend64.exe 可用时走增强冻结。失败个数汇总到错误信息。
#[tauri::command]
async fn freeze_pids(pids: Vec<u32>, enhanced: bool, scope: String) -> Result<(), String> {
    run_freeze(pids, enhanced, scope, true).await
}

/// 解冻指定进程（窗口恢复工具）；参数含义同 [`freeze_pids`]。
#[tauri::command]
async fn resume_pids(pids: Vec<u32>, enhanced: bool, scope: String) -> Result<(), String> {
    run_freeze(pids, enhanced, scope, false).await
}

/// freeze_pids / resume_pids 的公共实现：`suspend=true` 冻结，否则解冻。
async fn run_freeze(
    pids: Vec<u32>,
    enhanced: bool,
    scope: String,
    suspend: bool,
) -> Result<(), String> {
    blocking(move || {
        use zonedeck_core::freeze;
        let dir = exe_dir();
        let use_enhanced = enhanced && freeze::pssuspend_available(&dir);
        let names = freeze::process_names();
        // 与核心的 `scoped_pids` 同一套范围语义。
        let targets = match zonedeck_common::config::normalize_power_scope(&scope).as_str() {
            zonedeck_common::POWER_SCOPE_TREE => {
                zonedeck_core::hide::expand_descendants(&pids, &freeze::process_tree())
            }
            zonedeck_common::POWER_SCOPE_IMAGE => {
                zonedeck_core::hide::expand_same_image(&pids, &names)
            }
            _ => pids,
        };
        let targets: Vec<u32> = targets
            .into_iter()
            .filter(|pid| {
                !suspend
                    || !zonedeck_common::is_builtin_freeze_guarded(
                        names.get(pid).map(String::as_str).unwrap_or_default(),
                    )
            })
            .collect();
        let mut failed = 0usize;
        for pid in &targets {
            if *pid == 0 {
                continue;
            }
            let result = match (suspend, use_enhanced) {
                (true, true) => freeze::suspend_enhanced(&dir, *pid),
                (true, false) => freeze::suspend_process(*pid),
                (false, true) => freeze::resume_enhanced(&dir, *pid),
                (false, false) => freeze::resume_process(*pid),
            };
            if result.is_err() {
                failed += 1;
            }
        }
        if failed == 0 {
            Ok(())
        } else {
            let msg = if suspend {
                Msg::ErrFreezePartial
            } else {
                Msg::ErrResumePartial
            };
            Err(i18n::tf(
                msg,
                &[
                    ("failed", &failed.to_string()),
                    ("total", &targets.len().to_string()),
                ],
            ))
        }
    })
    .await
}

#[tauri::command]
async fn set_autostart(enabled: bool, admin: bool) -> Result<(), String> {
    blocking(
        move || match notify_core(&Command::SetAutostart { enabled, admin }) {
            Ok(Response::Error { message }) => Err(message),
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
    )
    .await
}

/// 开机自启当前状态：是否已注册，以及注册方式（计划任务 / 注册表）。
#[derive(Serialize, Clone, Copy)]
struct AutostartStatus {
    enabled: bool,
    /// `"task"` = 计划任务，`"registry"` = 注册表，`None` = 未注册。
    method: Option<&'static str>,
}

#[tauri::command]
async fn autostart_status() -> AutostartStatus {
    blocking(|| {
        use zonedeck_core::autostart::Method;
        // 用核心 exe 路径查询。
        let status =
            zonedeck_core::autostart::Autostart::for_exe(exe_dir().join(CORE_EXE)).status();
        let method = match status {
            Some(Method::TaskScheduler) => Some("task"),
            Some(Method::Registry) => Some("registry"),
            None => None,
        };
        AutostartStatus {
            enabled: status.is_some(),
            method,
        }
    })
    .await
}

#[derive(Serialize, Clone, Copy)]
struct CoreStatus {
    running: bool,
    hidden: bool,
    elevated: bool,
    /// 核心是否正在监听热键与鼠标。
    monitoring: bool,
    /// 自动隐藏当前是否启用。
    auto_hide_enabled: bool,
}

const CORE_OFFLINE: CoreStatus = CoreStatus {
    running: false,
    hidden: false,
    elevated: false,
    monitoring: false,
    auto_hide_enabled: false,
};

/// 核心状态：单次管道往返 + 快速失败，核心未运行时立即返回。
#[tauri::command]
async fn core_status() -> CoreStatus {
    blocking(|| {
        match PipeClient::connect_default()
            .fast()
            .send(&Command::GetStatus)
        {
            Ok(Response::Status {
                hidden,
                elevated,
                monitoring,
                auto_hide_enabled,
            }) => CoreStatus {
                running: true,
                hidden,
                elevated,
                monitoring,
                auto_hide_enabled,
            },
            _ => CORE_OFFLINE,
        }
    })
    .await
}

/// 启动核心。`elevated=true` 走 UAC 提权，返回值表示用户是否同意。
#[tauri::command]
async fn start_core(elevated: bool) -> Result<bool, String> {
    let exe = core_exe_path()?;
    blocking(move || {
        if elevated {
            Ok(zonedeck_core::elevation::relaunch_as_admin(&exe, ""))
        } else {
            spawn_core_detached(&exe, None).map(|_| true)
        }
    })
    .await
}

/// 重启核心：先请求旧核心退出释放互斥，再以指定权限重新启动。
#[tauri::command]
async fn restart_core(elevated: bool) -> Result<bool, String> {
    let exe = core_exe_path()?;
    blocking(move || {
        let _ = notify_core(&Command::Quit);
        std::thread::sleep(Duration::from_millis(400));
        if elevated {
            Ok(zonedeck_core::elevation::relaunch_as_admin(
                &exe, "elevated",
            ))
        } else {
            spawn_core_detached(&exe, Some("elevated")).map(|_| true)
        }
    })
    .await
}

#[tauri::command]
async fn quit_core() -> Result<(), String> {
    blocking(|| {
        let _ = notify_core(&Command::Quit);
        Ok(())
    })
    .await
}

/// 停用 / 恢复核心的热键与鼠标监控；返回核心是否应答。
#[tauri::command]
async fn set_hotkeys_enabled(enabled: bool) -> Result<bool, String> {
    blocking(move || {
        match PipeClient::connect_default()
            .fast()
            .send(&Command::SetHotkeys { enabled })
        {
            Ok(Response::Error { message }) => Err(message),
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    })
    .await
}

/// 录制期对键盘的独占；Tauri 托管，程序退出时随之析构，钩子自动卸掉。
#[derive(Default)]
struct CaptureState(Mutex<Option<KeyCapture>>);

/// 一次录制状态快照，推给界面渲染。
#[derive(Serialize, Clone, PartialEq)]
struct KeyCapturePayload {
    /// 当前按住的修饰键，形如 `"Ctrl+Shift"`；没有按住则为空串。
    modifiers: String,
    /// 当前按住的主键名；只按着修饰键或主键已抬起时为 null。
    key: Option<String>,
    down: bool,
    /// 按了热键表里没有的键（如小键盘、OEM 符号键）。
    unsupported: bool,
}

fn capture_payload(ev: &KeyEvent) -> KeyCapturePayload {
    // 抬起时不报主键，界面据此知道主键已松开。
    let key = if ev.down {
        zonedeck_core::hotkey::vk_to_key(ev.vk)
    } else {
        None
    };
    KeyCapturePayload {
        modifiers: zonedeck_core::hotkey::format_modifiers(ev.modifiers),
        unsupported: ev.down && key.is_none() && !zonedeck_core::key_capture::is_modifier(ev.vk),
        key,
        down: ev.down,
    }
}

/// 开始独占键盘录制。期间所有按键都被吞掉，不会漏给任何其他程序；
/// 每次按下 / 抬起以 `key-capture` 事件推给界面。幂等。
#[tauri::command]
fn start_key_capture(app: AppHandle, state: State<CaptureState>) -> Result<(), String> {
    let mut slot = state
        .0
        .lock()
        .map_err(|_| i18n::t(Msg::ErrKeyCaptureFailed).to_string())?;
    if slot.is_some() {
        return Ok(());
    }

    let (tx, rx) = std::sync::mpsc::channel::<KeyEvent>();
    // 钩子回调只往通道里塞一条就返回：低级钩子超时（默认 300ms）后事件会被系统丢弃，
    // 序列化与推送都放到排空线程上做。
    let capture = KeyCapture::start(move |ev| {
        let _ = tx.send(ev);
    })
    .ok_or_else(|| i18n::t(Msg::ErrKeyCaptureFailed).to_string())?;

    std::thread::spawn(move || {
        // 通道随录制结束而关闭，循环随之退出。
        let mut last: Option<KeyCapturePayload> = None;
        for ev in rx {
            let payload = capture_payload(&ev);
            // 长按的自动重复会刷出一串相同快照，吃掉。
            if last.as_ref() == Some(&payload) {
                continue;
            }
            let _ = app.emit("key-capture", &payload);
            last = Some(payload);
        }
    });

    *slot = Some(capture);
    Ok(())
}

/// 结束录制，把键盘还给系统。返回此前是否确实在录。幂等。
#[tauri::command]
fn stop_key_capture(state: State<CaptureState>) -> bool {
    match state.0.lock() {
        Ok(mut slot) => slot.take().is_some(),
        Err(mut poisoned) => poisoned.get_mut().take().is_some(),
    }
}

/// 增强冻结是否可用：需要 exe 同目录存在 pssuspend64.exe。
#[tauri::command]
async fn pssuspend_available() -> bool {
    blocking(|| zonedeck_core::freeze::pssuspend_available(&exe_dir())).await
}

/// 一批正则各自命中随机样本的条数；`None` 表示该条正则编译失败。
/// 判定放在后端，与核心共用同一个 `regex` 引擎。
#[tauri::command]
async fn regex_breadth(patterns: Vec<String>) -> Vec<Option<usize>> {
    blocking(move || {
        patterns
            .iter()
            .map(|p| zonedeck_common::regex_breadth(p))
            .collect()
    })
    .await
}

/// 白名单里不可删除的内置项（永不冻结的自有进程），供界面渲染锁定行。
#[derive(Serialize)]
struct BuiltinWhitelistEntry {
    /// 稳定标识，界面据此查本地化的角色名。
    key: &'static str,
    /// 该角色覆盖的全部映像名。
    names: &'static [&'static str],
}

#[tauri::command]
fn whitelist_builtins() -> Vec<BuiltinWhitelistEntry> {
    zonedeck_common::BUILTIN_FREEZE_GUARDS
        .iter()
        .map(|g| BuiltinWhitelistEntry {
            key: g.key,
            names: g.names,
        })
        .collect()
}

/// 启动参数里请求的动作（如 `restore`/`about`），只在启动时读一次。
#[tauri::command]
fn startup_action() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|a| a == zonedeck_common::ARG_RESTORE || a == zonedeck_common::ARG_ABOUT)
}

/// 打开日志目录（`<数据目录>/logs`）；目录不存在时先创建。
#[tauri::command]
async fn open_log_dir() -> Result<(), String> {
    blocking(|| {
        let dir = log_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        zonedeck_core::shell::open(&dir.to_string_lossy())
    })
    .await
}

/// 打开程序所在目录；pssuspend64.exe 需放在这里，增强冻结才可用。
#[tauri::command]
async fn open_program_dir() -> Result<(), String> {
    blocking(|| zonedeck_core::shell::open(&exe_dir().to_string_lossy())).await
}

/// 数据目录及其由来；界面据 `kind` 判断是否要提示写不进程序目录。
#[tauri::command]
fn data_location() -> DataLocation {
    use zonedeck_common::paths::DataDirKind;
    let located = zonedeck_common::paths::locate();
    DataLocation {
        dir: located.dir.display().to_string(),
        program_dir: located.program_dir.display().to_string(),
        kind: match located.kind {
            DataDirKind::Installed => "installed",
            DataDirKind::Portable => "portable",
            DataDirKind::PortableFallback => "portable_fallback",
        },
    }
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: zonedeck_common::APP_NAME,
        // 程序版本，非配置 schema 版本。
        version: env!("CARGO_PKG_VERSION"),
        website: "https://github.com/IvanHanloth/ZoneDeck",
        author: "Ivan Hanloth",
        email: "ivan@hanloth.com",
        blog: "https://blog.ivan-hanloth.cn/",
        license: "MIT",
    }
}

/// 用系统默认浏览器打开外部链接；仅放行 http/https/mailto。
#[tauri::command]
async fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") && !url.starts_with("mailto:") {
        return Err(i18n::t(Msg::ErrUrlSchemeNotAllowed).to_string());
    }
    blocking(move || zonedeck_core::shell::open(&url)).await
}

/// 项目公开链接（主页 / 仓库 / 文档等），带内存 + 磁盘缓存，有效期一天。
#[tauri::command]
async fn verhub_project_links() -> Result<verhub::ProjectLinks, String> {
    verhub::project_links(&data_dir().join("verhub_cache.json"))
        .await
        .map_err(|e| e.to_string())
}

/// 检查更新；`required=true` 即强制更新，界面须阻断使用。
#[tauri::command]
async fn verhub_check_update(include_preview: bool) -> Result<verhub::CheckUpdate, String> {
    verhub::check_update(env!("CARGO_PKG_VERSION"), include_preview)
        .await
        .map_err(|e| e.to_string())
}

/// 公告列表（本平台可见的，从新到旧）。
#[tauri::command]
async fn verhub_announcements(limit: u32) -> Result<Vec<verhub::Announcement>, String> {
    verhub::announcements(limit.clamp(1, 50))
        .await
        .map_err(|e| e.to_string())
}

/// 反馈提交选项，供前端决定是否显示「转换为 Issue」。
#[tauri::command]
async fn verhub_feedback_options() -> Result<verhub::FeedbackOptions, String> {
    verhub::feedback_options().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn verhub_submit_feedback(
    content: String,
    rating: Option<u8>,
    contact: String,
    forward_to_github: bool,
) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err(i18n::t(Msg::ErrFeedbackEmpty).to_string());
    }
    let contact = verhub::normalize_contact(&contact);
    if forward_to_github && contact.is_none() {
        return Err(i18n::t(Msg::ErrFeedbackContactRequired).to_string());
    }
    let custom_data = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "os": os_description(),
    });
    verhub::submit_feedback(
        content,
        rating.map(|r| r.clamp(1, 5)),
        contact,
        forward_to_github,
        custom_data,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn verhub_upload_log(content: String) -> Result<(), String> {
    let device_info = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "os": os_description(),
    });
    verhub::upload_log(&content, device_info)
        .await
        .map_err(|e| e.to_string())
}

/// 核心最近一次运行的日志，压到上报预算以内。
#[tauri::command]
async fn current_session_log() -> String {
    blocking(move || zonedeck_core::logging::latest_session(&log_dir(), verhub::LOG_EXCERPT_MAX))
        .await
}

/// 系统版本描述，形如 `Microsoft Windows [版本 10.0.26200.1234]`。
fn os_description() -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let version = std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if version.is_empty() {
        "Windows".to_string()
    } else {
        version
    }
}

/// 从 `cmd /C ver` 的横幅里取内部版本号；取不到按 0 处理。
/// `10.0.` 这一段不随系统语言变化，据它定位后面的 build 号。
fn parse_build(desc: &str) -> u32 {
    desc.split_once("10.0.")
        .and_then(|(_, rest)| {
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse().ok())
        })
        .unwrap_or(0)
}

/// 窗口背景材质。Mica 由 DWM 绘制，只有 Win11 22000+ 有；
/// Tauri 的 `apply_effects` 会吞掉失败，所以这里自己判版本。
/// 界面据此决定 body 留透明（让 Mica 透上来）还是自己铺一层不透明底色。
#[tauri::command]
fn backdrop_kind() -> &'static str {
    static KIND: OnceLock<&'static str> = OnceLock::new();
    KIND.get_or_init(|| {
        if parse_build(&os_description()) >= 22000 {
            "mica"
        } else {
            "solid"
        }
    })
}

pub fn run() {
    tauri::Builder::default()
        // 单实例：重复启动时激活已有窗口。必须是注册的第一个插件。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            if argv.iter().any(|a| a == zonedeck_common::ARG_RESTORE) {
                let _ = app.emit("open-restore", ());
            }
            if argv.iter().any(|a| a == zonedeck_common::ARG_ABOUT) {
                let _ = app.emit("open-about", ());
            }
        }))
        .manage(CaptureState::default())
        // 失焦时结束录制：钩子本身已在非前台时放行按键，这里只是让界面同步复位。
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<CaptureState>()
                    && stop_key_capture(state)
                {
                    let _ = app.emit("key-capture-stopped", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            list_windows,
            window_icons,
            show_all_windows,
            show_windows,
            hide_windows,
            freeze_pids,
            resume_pids,
            set_autostart,
            autostart_status,
            core_status,
            start_core,
            restart_core,
            quit_core,
            open_log_dir,
            open_program_dir,
            set_hotkeys_enabled,
            start_key_capture,
            stop_key_capture,
            pssuspend_available,
            regex_breadth,
            whitelist_builtins,
            startup_action,
            app_info,
            backdrop_kind,
            data_location,
            open_external,
            verhub_project_links,
            verhub_check_update,
            verhub_announcements,
            verhub_feedback_options,
            verhub_submit_feedback,
            verhub_upload_log,
            current_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("运行 ZoneDeck 配置程序时出错");
}

#[cfg(test)]
mod tests {
    use super::{capture_payload, parse_build};
    use zonedeck_core::hotkey::{MOD_CONTROL, MOD_SHIFT};
    use zonedeck_core::key_capture::KeyEvent;

    const VK_Q: u16 = 0x51;
    const VK_LCONTROL: u16 = 0xA2;
    const VK_NUMPAD0: u16 = 0x60;

    fn ev(vk: u16, down: bool, modifiers: u32) -> KeyEvent {
        KeyEvent {
            vk,
            down,
            modifiers,
        }
    }

    #[test]
    fn only_modifiers_held_reports_no_main_key() {
        let p = capture_payload(&ev(VK_LCONTROL, true, MOD_CONTROL));
        assert_eq!(p.modifiers, "Ctrl");
        assert_eq!(p.key, None);
        assert!(!p.unsupported, "修饰键本身不算不支持的按键");
    }

    #[test]
    fn a_supported_main_key_reports_the_whole_combo() {
        let p = capture_payload(&ev(VK_Q, true, MOD_CONTROL | MOD_SHIFT));
        assert_eq!(p.modifiers, "Ctrl+Shift");
        assert_eq!(p.key.as_deref(), Some("Q"));
        assert!(!p.unsupported);
    }

    #[test]
    fn a_key_outside_the_hotkey_table_is_flagged_unsupported() {
        let p = capture_payload(&ev(VK_NUMPAD0, true, 0));
        assert_eq!(p.key, None);
        assert!(p.unsupported, "界面据此提示换一个键");
    }

    #[test]
    fn releasing_a_key_clears_the_main_key_and_never_flags_unsupported() {
        let p = capture_payload(&ev(VK_Q, false, MOD_CONTROL));
        assert_eq!(p.modifiers, "Ctrl", "抬起主键后修饰键还按着");
        assert_eq!(p.key, None, "抬起时不报主键，界面据此知道松手了");
        assert!(!p.down);
        assert!(!p.unsupported);
        // 不支持的键抬起同样不该再刷提示。
        assert!(!capture_payload(&ev(VK_NUMPAD0, false, 0)).unsupported);
    }

    #[test]
    fn build_number_comes_from_the_ver_banner() {
        assert_eq!(
            parse_build("Microsoft Windows [版本 10.0.26200.1234]"),
            26200
        );
        assert_eq!(
            parse_build("Microsoft Windows [Version 10.0.22000.1]"),
            22000
        );
        // 少了 revision 段也要取得到
        assert_eq!(parse_build("Microsoft Windows [Version 10.0.19045]"), 19045);
    }

    #[test]
    fn unparsable_banner_falls_back_to_no_mica() {
        assert_eq!(parse_build("Windows"), 0);
        assert_eq!(parse_build(""), 0);
        assert_eq!(parse_build("Microsoft Windows [Version 10.0.]"), 0);
    }
}
