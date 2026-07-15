use std::path::{Path, PathBuf};
use std::time::Duration;

use bosskey_common::Config;
use bosskey_common::ipc::{Command, PipeClient, Response};
use bosskey_common::model::WindowInfo;
use bosskey_common::verhub;
use serde::Serialize;
use tauri::{Emitter, Manager};

const CORE_EXE: &str = "core.exe";

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_path() -> PathBuf {
    exe_dir().join("config.json")
}

/// 定位同目录下的核心程序，不存在时报错。
fn core_exe_path() -> Result<PathBuf, String> {
    let exe = exe_dir().join(CORE_EXE);
    if exe.exists() {
        Ok(exe)
    } else {
        Err(format!("未找到核心程序 {CORE_EXE}"))
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

/// 把阻塞工作丢到专用线程，避免占用 Tauri 主线程/异步运行时。
/// 所有涉及命名管道、schtasks、图标提取的命令都必须经此包装，
/// 否则会阻塞 WebView 渲染（曾导致"状态获取卡界面"）。
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .expect("阻塞任务执行失败")
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

#[tauri::command]
fn load_config() -> Result<Config, String> {
    Config::load(&config_path()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(config: Config) -> Result<(), String> {
    blocking(move || {
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

/// 恢复显示指定窗口（窗口恢复工具）：直接对选中的句柄 ShowWindow。
#[tauri::command]
async fn show_windows(hwnds: Vec<i64>) {
    blocking(move || {
        use bosskey_core::platform::WindowManager;
        let mgr = bosskey_core::platform::manager();
        for h in hwnds {
            mgr.show(h);
        }
    })
    .await
}

#[tauri::command]
async fn set_autostart(enabled: bool) -> Result<(), String> {
    blocking(
        move || match notify_core(&Command::SetAutostart { enabled }) {
            Ok(Response::Error { message }) => Err(message),
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
    )
    .await
}

#[tauri::command]
async fn autostart_status() -> bool {
    blocking(|| {
        // 用核心 exe 路径查询：自启项由核心写入，比对路径须与之一致。
        bosskey_core::autostart::Autostart::for_exe(exe_dir().join(CORE_EXE))
            .status()
            .is_some()
    })
    .await
}

#[derive(Serialize, Clone, Copy)]
struct CoreStatus {
    running: bool,
    hidden: bool,
    elevated: bool,
    /// 核心此刻是否真的在监听热键与鼠标（由核心回报，不是界面自己猜的）。
    monitoring: bool,
}

const CORE_OFFLINE: CoreStatus = CoreStatus {
    running: false,
    hidden: false,
    elevated: false,
    monitoring: false,
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
            }) => CoreStatus {
                running: true,
                hidden,
                elevated,
                monitoring,
            },
            _ => CORE_OFFLINE,
        }
    })
    .await
}

/// 启动核心（核心未运行时）。`elevated=true` 走 UAC 提权，返回值表示用户是否同意；
/// 普通启动总是返回 true（spawn 成功）。
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

/// 停用 / 恢复核心的热键与鼠标监控（录制热键、在鼠标设置区里操作时用）。
///
/// 返回核心是否确实应答：界面据此显示真实状态，而不是一厢情愿地把状态灯改掉。
/// 核心没运行时返回 false——本来就没有热键会被触发，不算错误。
/// 停用期间界面须按 `SUSPEND_HEARTBEAT_MS` 重发本命令续期（核心侧有看门狗）。
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

/// 启动参数里请求的动作（如核心托盘「窗口恢复工具」传入的 `restore`）；只在启动时读一次。
#[tauri::command]
fn startup_action() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|a| a == bosskey_common::ARG_RESTORE)
}

/// 打开日志目录（`<exe 同目录>/logs`）；目录不存在时先创建，再用资源管理器打开。
#[tauri::command]
async fn open_log_dir() -> Result<(), String> {
    blocking(|| {
        let dir = exe_dir().join(bosskey_core::logging::LOG_DIR_NAME);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: bosskey_common::APP_NAME,
        // 程序版本（Cargo.toml 的 workspace 版本号，发版流程会改写它），
        // 不是配置文件的 schema 版本 APP_CONFIG_VERSION
        version: env!("CARGO_PKG_VERSION"),
        website: "https://github.com/IvanHanloth/Boss-Key",
        author: "Ivan Hanloth",
        email: "ivan@hanloth.com",
        blog: "https://blog.ivan-hanloth.cn/",
        license: "MIT",
    }
}

/// 用系统默认浏览器打开外部链接（关于页的博客 / 项目主页 / 版本下载页）。
///
/// WebView 里的 `target="_blank"` 在 Tauri 下不会打开浏览器，只能走这里。
/// 只放行 http/https：`url` 来自界面，但别给「用 ShellExecute 执行任意东西」留口子。
#[tauri::command]
async fn open_external(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("只允许打开 http/https 链接".to_string());
    }
    blocking(move || {
        // 交给 explorer 而不是 `cmd /c start`：不弹控制台窗口，也不会把 url 里的 & 当成命令分隔符。
        std::process::Command::new("explorer")
            .arg(&url)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
}

/// 检查更新。`required=true` 即强制更新，界面须阻断使用。
#[tauri::command]
async fn verhub_check_update(include_preview: bool) -> Result<verhub::CheckUpdate, String> {
    blocking(move || {
        verhub::check_update(env!("CARGO_PKG_VERSION"), include_preview).map_err(|e| e.to_string())
    })
    .await
}

/// 公告列表（本平台可见的，从新到旧）。
#[tauri::command]
async fn verhub_announcements(limit: u32) -> Result<Vec<verhub::Announcement>, String> {
    blocking(move || verhub::announcements(limit.clamp(1, 50)).map_err(|e| e.to_string())).await
}

/// `contact` 可空——留了才好回复用户。
#[tauri::command]
async fn verhub_submit_feedback(
    content: String,
    rating: Option<u8>,
    contact: String,
) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("请先填写反馈内容".to_string());
    }
    blocking(move || {
        let feedback = verhub::Feedback {
            rating: rating.map(|r| r.clamp(1, 5)),
            content,
            platform: verhub::PLATFORM,
            custom_data: serde_json::json!({
                "app_version": env!("CARGO_PKG_VERSION"),
                "os": os_description(),
                "contact": contact.trim(),
            }),
        };
        verhub::submit_feedback(&feedback).map_err(|e| e.to_string())
    })
    .await
}

/// 上报一段日志（出错弹框里由用户点了「上报」才会走到这里；本程序不自动上报）。
#[tauri::command]
async fn verhub_upload_log(content: String) -> Result<(), String> {
    blocking(move || {
        verhub::upload_log(
            verhub::LogLevel::Error,
            &content,
            serde_json::json!({
                "app_version": env!("CARGO_PKG_VERSION"),
                "os": os_description(),
            }),
        )
        .map_err(|e| e.to_string())
    })
    .await
}

/// 最近的本地日志（出错弹框把它展示给用户过目，同意后才上报）。
/// 取最新一个日志文件的末尾若干行。
#[tauri::command]
async fn recent_log_tail(lines: usize) -> String {
    blocking(move || {
        let dir = exe_dir().join(bosskey_core::logging::LOG_DIR_NAME);
        let latest = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_file())
            .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let Some(entry) = latest else {
            return String::new();
        };
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let tail: Vec<&str> = content.lines().rev().take(lines).collect();
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
    })
    .await
}

/// 形如 `Microsoft Windows [版本 10.0.26200.1234]`，随反馈 / 日志一起上报，
/// 方便定位环境相关的问题。CREATE_NO_WINDOW：否则每次上报都闪一下黑窗口。
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
        // 单实例：核心「设置」或用户重复启动配置程序时，激活已有窗口而非再开一个。
        // 必须是注册的第一个插件。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            // 核心托盘「窗口恢复工具」会带 restore 参数再拉一次；已在运行的实例据此直达该工具。
            if argv.iter().any(|a| a == bosskey_common::ARG_RESTORE) {
                let _ = app.emit("open-restore", ());
            }
        }))
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            list_windows,
            window_icons,
            show_all_windows,
            show_windows,
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
            open_external,
            verhub_check_update,
            verhub_announcements,
            verhub_submit_feedback,
            verhub_upload_log,
            recent_log_tail,
        ])
        // 窗口以 visible:false 启动，正常情况下由前端画出启动屏后自己 show（见 main.js）。
        // 这里兜底：万一前端起不来，5 秒后强制显示，别留一个永远看不见的窗口。
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
