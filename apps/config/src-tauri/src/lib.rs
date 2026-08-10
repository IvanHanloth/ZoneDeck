use std::path::{Path, PathBuf};
use std::time::Duration;

use bosskey_common::Config;
use bosskey_common::ipc::{Command, PipeClient, Response};
use bosskey_common::model::WindowInfo;
use bosskey_core::i18n::{self, Msg};
use serde::Serialize;
use tauri::{Emitter, Manager};

mod verhub;

const CORE_EXE: &str = "Boss Key.exe";

/// 程序自身所在目录：只用来找同目录下的可执行文件（核心、pssuspend）。
/// 数据文件一律走 [`bosskey_common::paths`]——安装版存到用户目录，便携版才在这里。
fn exe_dir() -> PathBuf {
    bosskey_common::paths::exe_dir()
}

/// 数据目录（配置、日志、恢复文件、缓存）；与核心得出的结果一致。
fn data_dir() -> PathBuf {
    bosskey_common::paths::data_dir()
}

fn config_path() -> PathBuf {
    bosskey_common::paths::config_path()
}

fn log_dir() -> PathBuf {
    data_dir().join(bosskey_core::logging::LOG_DIR_NAME)
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

/// 把阻塞工作丢到专用线程，避免阻塞 Tauri 异步运行时与 WebView 渲染。
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
    /// 解析失败回退默认值时的原因（含备份去向），由前端提示用户；正常加载为 `None`。
    fallback: Option<String>,
}

#[tauri::command]
fn load_config() -> Result<LoadedConfig, String> {
    let (config, fallback) =
        Config::load_reporting(&config_path()).map_err(|e| e.to_string())?;
    i18n::set_from_pref(&config.setting.language);
    Ok(LoadedConfig { config, fallback })
}

/// 入参用 JSON 值而非 `Config`：经 [`Config::from_value`] 剥离 `null` 后再反序列化，
/// 界面上被清空的输入框（提交 `null`）不会导致整份配置保存失败。
#[tauri::command]
async fn save_config(config: serde_json::Value) -> Result<(), String> {
    blocking(move || {
        let config = Config::from_value(config).map_err(|e| e.to_string())?;
        i18n::set_from_pref(&config.setting.language);
        config.save(&config_path()).map_err(|e| e.to_string())?;
        // 通知核心热重载；核心据此同步语言。核心未运行时忽略错误。
        let _ = notify_core(&Command::ReloadConfig);
        Ok(())
    })
    .await
}

#[tauri::command]
async fn list_windows() -> Vec<WindowInfo> {
    blocking(|| {
        use bosskey_core::platform::WindowManager;
        bosskey_core::platform::manager().enumerate()
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
            if let Some(uri) = bosskey_core::icon::icon_data_uri(&path) {
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

/// 把恢复工具的窗口操作交给核心执行；核心未应答（未运行 / 出错）时返回 false，
/// 由调用方退回直接操作。
fn try_core_window_op(command: &Command) -> bool {
    matches!(
        PipeClient::connect_default().fast().send(command),
        Ok(Response::Ok)
    )
}

/// 恢复显示指定窗口（窗口恢复工具）。优先经核心释放并更新记录；
/// 核心不在运行才直接对句柄 ShowWindow。
#[tauri::command]
async fn show_windows(hwnds: Vec<i64>) {
    blocking(move || {
        if try_core_window_op(&Command::ReleaseWindows {
            hwnds: hwnds.clone(),
        }) {
            return;
        }
        use bosskey_core::platform::WindowManager;
        let mgr = bosskey_core::platform::manager();
        for h in hwnds {
            mgr.show(h);
        }
    })
    .await
}

/// 隐藏指定窗口（窗口恢复工具）。优先经核心纳入隐藏记录（不施加静音 / 冻结）；
/// 核心不在运行才直接对句柄隐藏。
#[tauri::command]
async fn hide_windows(hwnds: Vec<i64>) {
    blocking(move || {
        if try_core_window_op(&Command::AdoptWindows {
            hwnds: hwnds.clone(),
        }) {
            return;
        }
        use bosskey_core::platform::WindowManager;
        let mgr = bosskey_core::platform::manager();
        for h in hwnds {
            mgr.hide(h);
        }
    })
    .await
}

/// 冻结指定进程（窗口恢复工具）。`whole_tree` 为真时先展开为整棵子进程树；
/// `enhanced` 为真且 pssuspend64.exe 可用时走增强冻结，否则普通冻结。尽力而为，
/// 失败个数汇总到错误信息。直接操作，不经核心的隐藏状态跟踪。
#[tauri::command]
async fn freeze_pids(pids: Vec<u32>, enhanced: bool, whole_tree: bool) -> Result<(), String> {
    run_freeze(pids, enhanced, whole_tree, true).await
}

/// 解冻指定进程（窗口恢复工具）。参数含义同 [`freeze_pids`]。
#[tauri::command]
async fn resume_pids(pids: Vec<u32>, enhanced: bool, whole_tree: bool) -> Result<(), String> {
    run_freeze(pids, enhanced, whole_tree, false).await
}

/// freeze_pids / resume_pids 的公共实现：`suspend=true` 冻结，否则解冻。
async fn run_freeze(
    pids: Vec<u32>,
    enhanced: bool,
    whole_tree: bool,
    suspend: bool,
) -> Result<(), String> {
    blocking(move || {
        use bosskey_core::freeze;
        let dir = exe_dir();
        let use_enhanced = enhanced && freeze::pssuspend_available(&dir);
        let targets = if whole_tree {
            bosskey_core::hide::expand_descendants(&pids, &freeze::process_tree())
        } else {
            pids
        };
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
        use bosskey_core::autostart::Method;
        // 用核心 exe 路径查询，与核心写入的自启项一致。
        let status = bosskey_core::autostart::Autostart::for_exe(exe_dir().join(CORE_EXE)).status();
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
    /// 核心是否正在监听热键与鼠标（由核心回报）。
    monitoring: bool,
    /// 自动隐藏当前是否启用（托盘菜单也可切换，须回读对齐界面）。
    auto_hide_enabled: bool,
}

const CORE_OFFLINE: CoreStatus = CoreStatus {
    running: false,
    hidden: false,
    elevated: false,
    monitoring: false,
    auto_hide_enabled: false,
};

/// 核心状态：单次管道往返 + 快速失败（核心未运行时立即返回，不重试）。
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
            Ok(bosskey_core::elevation::relaunch_as_admin(&exe, ""))
        } else {
            spawn_core_detached(&exe, None).map(|_| true)
        }
    })
    .await
}

/// 重启核心：先请求旧核心退出释放互斥，再以指定权限重新启动（新实例等待互斥交接）。
#[tauri::command]
async fn restart_core(elevated: bool) -> Result<bool, String> {
    let exe = core_exe_path()?;
    blocking(move || {
        let _ = notify_core(&Command::Quit);
        std::thread::sleep(Duration::from_millis(400));
        if elevated {
            Ok(bosskey_core::elevation::relaunch_as_admin(&exe, "elevated"))
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

/// 停用 / 恢复核心的热键与鼠标监控。返回核心是否应答（未运行时返回 false）。
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

/// 增强冻结是否可用：需要 exe 同目录存在 pssuspend64.exe。
#[tauri::command]
async fn pssuspend_available() -> bool {
    blocking(|| bosskey_core::freeze::pssuspend_available(&exe_dir())).await
}

/// 启动参数里请求的动作（如 `restore`/`about`），只在启动时读一次。
#[tauri::command]
fn startup_action() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|a| a == bosskey_common::ARG_RESTORE || a == bosskey_common::ARG_ABOUT)
}

/// 打开日志目录（`<数据目录>/logs`）；目录不存在时先创建，再用资源管理器打开。
#[tauri::command]
async fn open_log_dir() -> Result<(), String> {
    blocking(|| {
        let dir = log_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        bosskey_core::shell::open(&dir.to_string_lossy())
    })
    .await
}

/// 数据目录及其由来。界面据 `kind` 判断是否要提示便携版写不进程序目录。
#[tauri::command]
fn data_location() -> DataLocation {
    use bosskey_common::paths::DataDirKind;
    let located = bosskey_common::paths::locate();
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
        name: bosskey_common::APP_NAME,
        // 程序版本（非配置 schema 版本 APP_CONFIG_VERSION）。
        version: env!("CARGO_PKG_VERSION"),
        website: "https://github.com/IvanHanloth/Boss-Key",
        author: "Ivan Hanloth",
        email: "ivan@hanloth.com",
        blog: "https://blog.ivan-hanloth.cn/",
        license: "MIT",
    }
}

/// 用系统默认浏览器打开外部链接。仅放行 http/https/mailto（与前端 markdown 白名单一致）。
#[tauri::command]
async fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") && !url.starts_with("mailto:") {
        return Err(i18n::t(Msg::ErrUrlSchemeNotAllowed).to_string());
    }
    blocking(move || bosskey_core::shell::open(&url)).await
}

/// 项目公开链接（主页 / 仓库 / 文档等）。带缓存（内存 + 数据目录下的磁盘文件，
/// 有效期一天），过期才请求 Verhub；请求失败退回过期缓存。
#[tauri::command]
async fn verhub_project_links() -> Result<verhub::ProjectLinks, String> {
    verhub::project_links(&data_dir().join("verhub_cache.json"))
        .await
        .map_err(|e| e.to_string())
}

/// 检查更新。`required=true` 即强制更新，界面须阻断使用。
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
    // 转换成 Issue 后要靠 GitHub 账号跟进，缺了服务端也不受理。
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

/// 核心最近一次运行的日志：从该次运行的 `[START]` 起至今，压到上报预算以内。
#[tauri::command]
async fn current_session_log() -> String {
    blocking(move || bosskey_core::logging::latest_session(&log_dir(), verhub::LOG_EXCERPT_MAX))
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

pub fn run() {
    tauri::Builder::default()
        // 单实例：重复启动时激活已有窗口。必须是注册的第一个插件。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            if argv.iter().any(|a| a == bosskey_common::ARG_RESTORE) {
                let _ = app.emit("open-restore", ());
            }
            if argv.iter().any(|a| a == bosskey_common::ARG_ABOUT) {
                let _ = app.emit("open-about", ());
            }
        }))
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
            set_hotkeys_enabled,
            pssuspend_available,
            startup_action,
            app_info,
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
        // 兜底：前端起不来时，5 秒后强制显示窗口。
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    if !window.is_visible().unwrap_or(false) {
                        let _ = window.show();
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 Boss Key 配置程序时出错");
}
