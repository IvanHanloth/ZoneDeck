// release 下以窗口子系统编译（无控制台）；debug 保留控制台便于开发。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use bosskey_common::paths;
use bosskey_core::agent::{self, AgentOptions};
use bosskey_core::logging;
use bosskey_core::single_instance::SingleInstance;

const MUTEX_NAME: &str = "BossKey_SingleInstance_Mutex";

fn main() {
    // 数据目录只定位一次，配置、日志、恢复文件共用，避免两次定位得出不同结果。
    let located = paths::locate();
    let data_dir = located.dir.clone();
    let config_path = data_dir.join(paths::CONFIG_FILE_NAME);

    // 日志与 panic 钩子最先就位。日志保留天数取自配置（0 = 关闭日志）。
    let retention_days = bosskey_common::Config::load(&config_path)
        .map(|c| c.setting.log_retention_days)
        .unwrap_or(bosskey_common::config::DEFAULT_LOG_RETENTION_DAYS);
    logging::init(data_dir.join(logging::LOG_DIR_NAME), retention_days);
    logging::install_panic_hook();
    logging::info(&format!(
        "核心启动 {}（配置 schema {}）",
        bosskey_common::APP_VERSION,
        bosskey_common::APP_CONFIG_VERSION
    ));
    // 数据究竟落在哪里是排查读写失败的第一手信息，每次启动都记一笔。
    logging::info(&format!(
        "数据目录: {}（{}）",
        data_dir.display(),
        match located.kind {
            paths::DataDirKind::Installed => "安装版",
            paths::DataDirKind::Portable => "便携版",
            paths::DataDirKind::PortableFallback => "便携版，程序目录不可写，已回退",
        }
    ));
    if located.kind == paths::DataDirKind::PortableFallback {
        logging::warn(&format!(
            "程序目录 {} 无写入权限，设置改存到 {}。要让设置随程序目录携带，请把程序移到有写入权限的位置",
            located.program_dir.display(),
            data_dir.display()
        ));
    }

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

    let mut options = AgentOptions::standard(config_path);
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
