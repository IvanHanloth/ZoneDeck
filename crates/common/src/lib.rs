pub mod config;
pub mod i18n;
pub mod ipc;
pub mod matching;
pub mod model;
pub mod paths;

pub use config::{
    Config, ConfigError, Hotkey, MouseButton, MouseSetting, Notifications, Setting, Verhub,
};
pub use i18n::{LANG_AUTO, Lang};
pub use ipc::{Command, Response};
pub use matching::{WindowResolution, match_process_rule, regex_is_valid, resolve_window_rule};
pub use model::{ProcessRule, WindowInfo, WindowRule};

pub const APP_NAME: &str = "Boss Key";
/// 配置 schema 版本：配置结构变动时才动，与程序版本无关。
pub const APP_CONFIG_VERSION: &str = "v3.0.0.0";
/// 程序版本（workspace 版本号，唯一真源是根 `Cargo.toml`）。
/// 核心据它判断「更新后首次启动」，见 [`Config::app_version`]。
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NO_TITLE: &str = "无标题窗口";

/// 命令行参数：让配置程序启动后直达「窗口恢复工具」（核心托盘菜单使用）。
pub const ARG_RESTORE: &str = "restore";

/// 命令行参数：让配置程序启动后直达「关于」页（核心托盘菜单使用）。
pub const ARG_ABOUT: &str = "about";
