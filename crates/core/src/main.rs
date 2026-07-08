use std::path::PathBuf;

use bosskey_common::Config;
use bosskey_core::agent;
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

    if args.get(1).map(String::as_str) == Some("check") {
        let config = Config::load(&config_path()).unwrap_or_default();
        let ok = agent::check(&config);
        println!("热键注册自检: {}", if ok { "成功" } else { "失败" });
        return;
    }

    let instance = SingleInstance::acquire(MUTEX_NAME);
    if instance.already_running() {
        eprintln!("Boss Key 已在运行");
        return;
    }

    let config = Config::load(&config_path()).unwrap_or_default();
    agent::run(config);
}
