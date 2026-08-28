pub mod config;
pub mod i18n;
pub mod ipc;
pub mod matching;
pub mod model;
pub mod paths;

pub use config::{
    Config, ConfigError, Hotkey, LoadNote, MouseButton, MouseSetting, Notifications,
    POWER_SCOPE_IMAGE, POWER_SCOPE_SELF, POWER_SCOPE_TREE, POWER_SCOPES, Setting, TRAY_ACTION_MENU,
    TRAY_ACTION_NONE, TRAY_ACTION_SETTINGS, TRAY_ACTION_TOGGLE, TRAY_ACTIONS, TrayClicks, Verhub,
};
pub use i18n::{LANG_AUTO, Lang};
pub use ipc::{Command, Response};
pub use matching::{
    BUILTIN_FREEZE_GUARDS, BuiltinGuard, CONFIG_IMAGE_NAMES, IgnoreMode, WindowResolution,
    is_builtin_freeze_guarded, is_config_image, is_ignored, match_process_rule, regex_breadth,
    regex_is_broad, regex_is_valid, resolve_window_rule, whitelist_needs_paths,
};
pub use model::{ProcessRule, WhitelistRule, WindowInfo, WindowRule};

pub const APP_NAME: &str = "ZoneDeck";
/// 配置 schema 版本：配置结构变动时才动，与程序版本无关。
pub const APP_CONFIG_VERSION: &str = "v3.0.0.0";
/// 程序版本，真源是根 `Cargo.toml`。见 [`Config::app_version`]。
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NO_TITLE: &str = "无标题窗口";

/// 命令行参数：让配置程序启动后直达「窗口恢复工具」。
pub const ARG_RESTORE: &str = "restore";

/// 命令行参数：让配置程序启动后直达「关于」页。
pub const ARG_ABOUT: &str = "about";
