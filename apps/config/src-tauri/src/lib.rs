use std::path::PathBuf;

use bosskey_common::Config;
use bosskey_common::ipc::{Command, PipeClient, Response};
use bosskey_common::model::WindowInfo;
use serde::Serialize;

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
    core_running: bool,
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

#[tauri::command]
fn hidden_state() -> bool {
    matches!(
        notify_core(&Command::GetState),
        Ok(Response::State { hidden: true })
    )
}

#[tauri::command]
fn show_all_windows() -> Result<(), String> {
    notify_core(&Command::Show).map(|_| ())
}

#[tauri::command]
fn hide_now() -> Result<(), String> {
    notify_core(&Command::Hide).map(|_| ())
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

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: bosskey_common::APP_NAME,
        version: bosskey_common::APP_CONFIG_VERSION,
        website: "https://github.com/IvanHanloth/Boss-Key",
        update_feed: "https://ivanhanloth.github.io/Boss-Key/releases.json",
        core_running: PipeClient::connect_default()
            .send(&Command::GetState)
            .is_ok(),
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            list_windows,
            hidden_state,
            show_all_windows,
            hide_now,
            set_autostart,
            autostart_status,
            app_info,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Boss Key 配置程序时出错");
}
