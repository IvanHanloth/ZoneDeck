use std::path::PathBuf;

use bosskey_core::agent::{self, AgentOptions};
use bosskey_core::single_instance::SingleInstance;

const MUTEX_NAME: &str = "BossKey_SingleInstance_Mutex";

fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.json")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let instance = SingleInstance::acquire(MUTEX_NAME);
    if instance.already_running() {
        eprintln!("Boss Key 已在运行");
        return;
    }

    let mut options = AgentOptions::standard(config_path());
    if args.get(1).map(String::as_str) == Some("smoke") {
        let ms = args
            .get(2)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(3000);
        options.auto_quit_ms = Some(ms);
        println!("冒烟模式: {ms} 毫秒后自动退出");
    }
    agent::run(options);
}
