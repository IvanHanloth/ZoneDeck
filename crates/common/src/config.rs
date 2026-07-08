use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::APP_CONFIG_VERSION;
use crate::model::WindowInfo;

pub const DEFAULT_HIDE_HOTKEY: &str = "Ctrl+Q";
pub const DEFAULT_CLOSE_HOTKEY: &str = "Win+Esc";
pub const DEFAULT_AUTO_HIDE_TIME: u32 = 5;

fn default_hide_hotkey() -> String {
    DEFAULT_HIDE_HOTKEY.to_string()
}
fn default_close_hotkey() -> String {
    DEFAULT_CLOSE_HOTKEY.to_string()
}
fn default_version() -> String {
    APP_CONFIG_VERSION.to_string()
}
fn default_true() -> bool {
    true
}
fn default_auto_hide_time() -> u32 {
    DEFAULT_AUTO_HIDE_TIME
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置文件读写错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("配置文件 JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    #[serde(default = "default_hide_hotkey")]
    pub hide_hotkey: String,
    #[serde(default = "default_close_hotkey")]
    pub close_hotkey: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            hide_hotkey: default_hide_hotkey(),
            close_hotkey: default_close_hotkey(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    #[serde(default = "default_true")]
    pub mute_after_hide: bool,
    #[serde(default)]
    pub send_before_hide: bool,
    #[serde(default = "default_true")]
    pub hide_current: bool,
    #[serde(default = "default_true")]
    pub click_to_hide: bool,
    #[serde(default)]
    pub hide_icon_after_hide: bool,
    #[serde(default)]
    pub path_match: bool,
    #[serde(default)]
    pub freeze_after_hide: bool,
    #[serde(default)]
    pub enhanced_freeze: bool,
    #[serde(default)]
    pub show_float_window: bool,
    #[serde(default)]
    pub middle_button_hide: bool,
    #[serde(default)]
    pub side_button1_hide: bool,
    #[serde(default)]
    pub side_button2_hide: bool,
    #[serde(default)]
    pub auto_hide_enabled: bool,
    #[serde(default = "default_auto_hide_time")]
    pub auto_hide_time: u32,
    #[serde(default)]
    pub top_left_hide: bool,
    #[serde(default)]
    pub top_right_hide: bool,
    #[serde(default)]
    pub bottom_left_hide: bool,
    #[serde(default)]
    pub bottom_right_hide: bool,
    #[serde(default)]
    pub allow_move_restore: bool,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            mute_after_hide: true,
            send_before_hide: false,
            hide_current: true,
            click_to_hide: true,
            hide_icon_after_hide: false,
            path_match: false,
            freeze_after_hide: false,
            enhanced_freeze: false,
            show_float_window: false,
            middle_button_hide: false,
            side_button1_hide: false,
            side_button2_hide: false,
            auto_hide_enabled: false,
            auto_hide_time: DEFAULT_AUTO_HIDE_TIME,
            top_left_hide: false,
            top_right_hide: false,
            bottom_left_hide: false,
            bottom_right_hide: false,
            allow_move_restore: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub history: Vec<i64>,
    #[serde(default)]
    pub frozen_pids: Vec<u32>,
    #[serde(default)]
    pub hotkey: Hotkey,
    #[serde(default)]
    pub setting: Setting,
    #[serde(default)]
    pub hide_binding: Vec<WindowInfo>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            history: Vec::new(),
            frozen_pids: Vec::new(),
            hotkey: Hotkey::default(),
            setting: Setting::default(),
            hide_binding: Vec::new(),
        }
    }
}

impl Config {
    pub fn from_json(s: &str) -> Result<Self, ConfigError> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> &'static str {
        r#"{
            "version": "v2.1.0.0",
            "history": [111, 222],
            "frozen_pids": [4321],
            "hotkey": { "hide_hotkey": "Ctrl+Shift+H", "close_hotkey": "Win+Esc" },
            "setting": {
                "mute_after_hide": false,
                "send_before_hide": true,
                "hide_current": false,
                "click_to_hide": true,
                "hide_icon_after_hide": true,
                "path_match": true,
                "freeze_after_hide": true,
                "enhanced_freeze": false,
                "show_float_window": true,
                "middle_button_hide": true,
                "side_button1_hide": false,
                "side_button2_hide": false,
                "auto_hide_enabled": true,
                "auto_hide_time": 15,
                "top_left_hide": true,
                "top_right_hide": false,
                "bottom_left_hide": false,
                "bottom_right_hide": false,
                "allow_move_restore": true
            },
            "hide_binding": [
                {"title": "微信", "hwnd": 6789, "process": "WeChat.exe", "PID": 8888, "path": "C:\\WeChat.exe"}
            ]
        }"#
    }

    #[test]
    fn parses_full_v21_config() {
        let c = Config::from_json(sample_json()).unwrap();
        assert_eq!(c.version, "v2.1.0.0");
        assert_eq!(c.history, vec![111, 222]);
        assert_eq!(c.frozen_pids, vec![4321]);
        assert_eq!(c.hotkey.hide_hotkey, "Ctrl+Shift+H");
        assert!(!c.setting.mute_after_hide);
        assert!(c.setting.path_match);
        assert_eq!(c.setting.auto_hide_time, 15);
        assert_eq!(c.hide_binding.len(), 1);
        assert_eq!(c.hide_binding[0].process, "WeChat.exe");
        assert_eq!(c.hide_binding[0].pid, 8888);
    }

    #[test]
    fn missing_setting_keys_use_python_load_defaults() {
        let c = Config::from_json(r#"{"setting": {}}"#).unwrap();
        assert!(c.setting.mute_after_hide);
        assert!(c.setting.hide_current);
        assert!(c.setting.click_to_hide);
        assert!(!c.setting.path_match);
        assert!(!c.setting.freeze_after_hide);
        assert_eq!(c.setting.auto_hide_time, 5);
    }

    #[test]
    fn empty_object_yields_all_defaults() {
        let c = Config::from_json("{}").unwrap();
        assert_eq!(c, Config::default());
    }

    #[test]
    fn corrupt_string_is_a_hard_error_via_from_json() {
        assert!(Config::from_json("{ this is not json ").is_err());
    }

    #[test]
    fn round_trip_is_stable() {
        let c = Config::from_json(sample_json()).unwrap();
        let json = c.to_json().unwrap();
        let back = Config::from_json(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn serialized_binding_uses_uppercase_pid() {
        let mut c = Config::default();
        c.hide_binding
            .push(WindowInfo::new("t", 1, "p.exe", 77, "C:\\p.exe"));
        let json = c.to_json().unwrap();
        assert!(json.contains("\"PID\": 77"), "应保留大写 PID: {json}");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let c = Config::from_json(r#"{"future_flag": true, "setting": {"brand_new": 1}}"#).unwrap();
        assert_eq!(c.setting, Setting::default());
    }
}
