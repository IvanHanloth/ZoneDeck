use std::path::PathBuf;
use std::time::Duration;

use bosskey_common::Config;
use bosskey_common::ipc::{Command, PipeClient, Response};
use bosskey_common::model::WindowInfo;
use serde::Serialize;

const CORE_EXE: &str = "bosskey-core.exe";

fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.json")
}

fn notify_core(command: &Command) -> Result<Response, String> {
    PipeClient::connect_default()
        .send(command)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    website: &'static str,
    update_feed: &'static str,
}

#[tauri::command]
fn load_config() -> Result<Config, String> {
    Config::load(&config_path()).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    config.save(&config_path()).map_err(|e| e.to_string())?;
    // 通知核心热重载；核心未运行时忽略错误。
    let _ = notify_core(&Command::ReloadConfig);
    Ok(())
}

#[tauri::command]
fn list_windows() -> Vec<WindowInfo> {
    use bosskey_core::platform::WindowManager;
    bosskey_core::platform::manager().enumerate()
}

/// 批量提取可执行文件图标，返回 `路径 → PNG data URI`。
/// 没有图标或文件不存在的路径不出现在结果里。
#[tauri::command]
fn window_icons(paths: Vec<String>) -> std::collections::HashMap<String, String> {
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
}

#[tauri::command]
fn show_all_windows() -> Result<(), String> {
    notify_core(&Command::Show).map(|_| ())
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    match notify_core(&Command::SetAutostart { enabled }) {
        Ok(Response::Error { message }) => Err(message),
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn autostart_status() -> bool {
    bosskey_core::autostart::Autostart::standard()
        .map(|a| a.status().is_some())
        .unwrap_or(false)
}

#[derive(Serialize)]
struct CoreStatus {
    running: bool,
    hidden: bool,
    elevated: bool,
}

#[tauri::command]
fn core_status() -> CoreStatus {
    let client = PipeClient::connect_default();
    let running = client.send(&Command::GetState).is_ok();
    let hidden = matches!(
        client.send(&Command::GetState),
        Ok(Response::State { hidden: true })
    );
    let elevated = matches!(
        client.send(&Command::GetElevation),
        Ok(Response::Elevated { elevated: true })
    );
    CoreStatus {
        running,
        hidden,
        elevated,
    }
}

/// 以管理员身份重启核心：先请求核心退出释放互斥，再用 UAC 提权重新启动。
#[tauri::command]
fn restart_core_elevated() -> Result<bool, String> {
    let core_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(CORE_EXE)))
        .ok_or_else(|| "无法定位核心程序目录".to_string())?;
    if !core_exe.exists() {
        return Err(format!("未找到核心程序 {CORE_EXE}"));
    }

    let _ = notify_core(&Command::Quit);
    std::thread::sleep(Duration::from_millis(400));

    Ok(bosskey_core::elevation::relaunch_as_admin(
        &core_exe, "elevated",
    ))
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: bosskey_common::APP_NAME,
        version: bosskey_common::APP_CONFIG_VERSION,
        website: "https://github.com/IvanHanloth/Boss-Key",
        update_feed: "https://ivanhanloth.github.io/Boss-Key/releases.json",
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            list_windows,
            window_icons,
            show_all_windows,
            set_autostart,
            autostart_status,
            core_status,
            restart_core_elevated,
            app_info,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Boss Key 配置程序时出错");
}
