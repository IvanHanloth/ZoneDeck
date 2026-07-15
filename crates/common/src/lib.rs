pub mod config;
pub mod ipc;
pub mod matching;
pub mod model;
pub mod verhub;

pub use config::{
    Config, ConfigError, Hotkey, MouseButton, MouseSetting, Notifications, Setting, Verhub,
};
pub use ipc::{Command, Response};
pub use matching::{WindowResolution, match_process_rule, regex_is_valid, resolve_window_rule};
pub use model::{ProcessRule, WindowInfo, WindowRule};

pub const APP_NAME: &str = "Boss Key";
pub const APP_CONFIG_VERSION: &str = "v3.0.0.0";
pub const NO_TITLE: &str = "无标题窗口";

/// 命令行参数：让配置程序启动后直达「窗口恢复工具」（核心托盘菜单使用）。
pub const ARG_RESTORE: &str = "restore";
