// release 下以「窗口子系统」编译：核心是无界面的常驻 agent，不应弹出控制台窗口，
// 也不应因启动它的进程（如配置程序）关闭控制台而被连带终止。
// debug 保留控制台，便于开发时直接看 stdout/eprintln。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use bosskey_core::agent::{self, AgentOptions};
use bosskey_core::logging;
use bosskey_core::single_instance::SingleInstance;

const MUTEX_NAME: &str = "BossKey_SingleInstance_Mutex";
const LOG_FILE_NAME: &str = "bosskey.log";

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_path() -> PathBuf {
    exe_dir().join("config.json")
}

fn main() {
    // 日志与 panic 钩子最先就位：崩溃信息落盘后进程以非零码退出，
    // 由计划任务的 RestartOnFailure 负责重新拉起。
    logging::init(exe_dir().join(LOG_FILE_NAME));
    logging::install_panic_hook();
    logging::info(&format!("核心启动 {}", bosskey_common::APP_CONFIG_VERSION));

    let args: Vec<String> = std::env::args().collect();

    // 以管理员身份重启时，等待旧实例释放互斥后再接管。
    let elevated_restart = args.iter().any(|a| a == "elevated");
    let instance = if elevated_restart {
        SingleInstance::acquire_waiting(MUTEX_NAME, Duration::from_secs(4))
    } else {
        SingleInstance::acquire(MUTEX_NAME)
    };
    if instance.already_running() {
        logging::warn("已有核心实例在运行，本次启动退出");
        return;
    }

    let mut options = AgentOptions::standard(config_path());
    if args.iter().any(|a| a == "smoke") {
        let ms = args
            .iter()
            .position(|a| a == "smoke")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(3000);
        options.auto_quit_ms = Some(ms);
        println!("冒烟模式: {ms} 毫秒后自动退出");
    }
    agent::run(options);
}
