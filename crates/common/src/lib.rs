pub mod config;
pub mod ipc;
pub mod matching;
pub mod model;

pub use config::{Config, ConfigError, Hotkey, Setting};
pub use ipc::{Command, Response};
pub use matching::is_same_window;
pub use model::WindowInfo;

pub const APP_NAME: &str = "Boss Key";
pub const APP_CONFIG_VERSION: &str = "v3.0.0.0";
pub const NO_TITLE: &str = "无标题窗口";
