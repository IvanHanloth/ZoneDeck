// release 下以窗口子系统编译（无控制台），debug 保留控制台。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use zonedeck_common::paths;
use zonedeck_core::agent::{self, AgentOptions};
use zonedeck_core::i18n::{self, Msg};
use zonedeck_core::logging;
use zonedeck_core::single_instance::SingleInstance;

const MUTEX_NAME: &str = "ZoneDeck_SingleInstance_Mutex";
/// 改名前的互斥体名，用于探测仍在运行的旧版核心。
const LEGACY_MUTEX_NAME: &str = "BossKey_SingleInstance_Mutex";
/// 提权重启时等待前一个实例退出的上限。
const ELEVATED_HANDOVER_WAIT: Duration = Duration::from_secs(4);

/// 旧版核心仍在运行时弹窗提醒。此时日志尚不可用，语言跟随系统。
fn warn_legacy_core_running() {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONWARNING, MB_OK, MessageBoxW};
    use windows::core::PCWSTR;
    i18n::set_from_pref("");
    let to_wide = |s: &str| {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    };
    let title = to_wide(i18n::t(Msg::LegacyCoreRunningTitle));
    let body = to_wide(i18n::t(Msg::LegacyCoreRunningBody));
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONWARNING,
        );
    }
}

fn main() {
    // 旧品牌核心还在运行时不得继续：两个核心会抢热键，迁移还会搬走它的数据目录。
    // 探测必须先于 paths::locate()，后者一执行就触发迁移。
    let legacy_probe = SingleInstance::acquire(LEGACY_MUTEX_NAME);
    let legacy_running = legacy_probe.already_running();
    drop(legacy_probe);
    if legacy_running {
        warn_legacy_core_running();
        return;
    }

    // 数据目录只定位一次，配置、日志、恢复文件共用。
    let located = paths::locate();
    let data_dir = located.dir.clone();
    let config_path = data_dir.join(paths::CONFIG_FILE_NAME);

    // 配置只解析一次：这里取日志参数，随后原样交给 agent::run。
    let startup = agent::StartupConfig::load(&config_path);
    let retention_days = startup.config.setting.log_retention_days;
    let log_level = logging::Level::from_config(&startup.config.setting.log_level);
    logging::init(
        data_dir.join(logging::LOG_DIR_NAME),
        retention_days,
        log_level,
    );
    logging::install_panic_hook();
    // 会话起始标记：不受输出等级过滤。
    logging::session_start(&format!(
        "核心启动 {}（配置 schema {}，日志等级 {}）｜数据目录: {}（{}）",
        zonedeck_common::APP_VERSION,
        zonedeck_common::APP_CONFIG_VERSION,
        log_level.as_config_str(),
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

    // 提权重启时等待旧实例释放互斥后再接管。
    let elevated_restart = args.iter().any(|a| a == "elevated");
    let instance = if elevated_restart {
        logging::debug("以管理员身份重启，等待前一个实例释放单实例互斥体");
        SingleInstance::acquire_waiting(MUTEX_NAME, ELEVATED_HANDOVER_WAIT)
    } else {
        SingleInstance::acquire(MUTEX_NAME)
    };
    if instance.already_running() {
        if elevated_restart {
            logging::warn(&format!(
                "以管理员身份重启失败：等待 {ELEVATED_HANDOVER_WAIT:?} 后前一个实例仍在运行，本次启动退出，核心仍以原权限运行"
            ));
        } else if args.iter().any(|a| a == "smoke") {
            logging::warn("已有核心实例在运行，本次启动退出");
        } else {
            // 重复双击本该毫无反馈，转为打开配置界面，让用户看见核心确实在跑。
            logging::info("已有核心实例在运行，转为打开配置界面");
            if !agent::forward_open_settings() {
                logging::warn(
                    "已有核心实例在运行，但打开配置界面失败：核心未应答且同目录下拉起配置程序未成功（可能缺失或被安全软件拦截）",
                );
            }
        }
        return;
    }

    let mut options = AgentOptions::standard(config_path);
    options.preloaded = Some(startup);
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
