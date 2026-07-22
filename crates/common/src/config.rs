use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::APP_CONFIG_VERSION;
use crate::i18n::LANG_AUTO;
use crate::model::{ProcessRule, WindowInfo, WindowRule};

pub const DEFAULT_HIDE_HOTKEY: &str = "Ctrl+Q";
pub const DEFAULT_CLOSE_HOTKEY: &str = "Win+Esc";
pub const DEFAULT_AUTO_HIDE_TIME: u32 = 5;
/// 日志默认保留天数（`0` 表示关闭日志）。
pub const DEFAULT_LOG_RETENTION_DAYS: u32 = 7;
/// 连击判定窗口默认值（毫秒）：两次点击间隔不超过它才算连击。
pub const DEFAULT_MULTI_CLICK_MS: u32 = 350;
pub const MIN_MULTI_CLICK_MS: u32 = 150;
pub const MAX_MULTI_CLICK_MS: u32 = 1000;
/// 最多支持三连击。
pub const MAX_CLICKS: u8 = 3;

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
fn default_log_retention_days() -> u32 {
    DEFAULT_LOG_RETENTION_DAYS
}
fn default_clicks() -> u8 {
    1
}
fn default_multi_click_ms() -> u32 {
    DEFAULT_MULTI_CLICK_MS
}
fn default_language() -> String {
    LANG_AUTO.to_string()
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
    /// 隐藏热键是否不传递给其他程序（核心改用键盘钩子拦截）。
    #[serde(default)]
    pub hide_intercept: bool,
    /// 关闭热键是否不传递给其他程序（核心改用键盘钩子拦截）。
    #[serde(default)]
    pub close_intercept: bool,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            hide_hotkey: default_hide_hotkey(),
            close_hotkey: default_close_hotkey(),
            hide_intercept: false,
            close_intercept: false,
        }
    }
}

/// 一颗鼠标键的触发条件：连击几次、要不要同时按住修饰键。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseButton {
    #[serde(default)]
    pub enabled: bool,
    /// 连击次数，1..=[`MAX_CLICKS`]。
    #[serde(default = "default_clicks")]
    pub clicks: u8,
    /// 修饰键组合，如 `"Ctrl+Shift"`；空串表示不需要修饰键。
    #[serde(default)]
    pub modifiers: String,
}

impl Default for MouseButton {
    fn default() -> Self {
        Self {
            enabled: false,
            clicks: default_clicks(),
            modifiers: String::new(),
        }
    }
}

impl MouseButton {
    fn normalize(&mut self) {
        self.clicks = self.clicks.clamp(1, MAX_CLICKS);
    }
}

/// 五颗鼠标键各自的触发条件 + 全局的连击判定窗口。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseSetting {
    #[serde(default)]
    pub left: MouseButton,
    #[serde(default)]
    pub middle: MouseButton,
    #[serde(default)]
    pub right: MouseButton,
    /// 侧键 1（前进键）。
    #[serde(default)]
    pub side1: MouseButton,
    /// 侧键 2（后退键）。
    #[serde(default)]
    pub side2: MouseButton,
    /// 连击判定窗口（毫秒），[`MIN_MULTI_CLICK_MS`]..=[`MAX_MULTI_CLICK_MS`]。
    #[serde(default = "default_multi_click_ms")]
    pub multi_click_ms: u32,
    /// 是否允许再按一次同样的键恢复窗口。
    #[serde(default = "default_true")]
    pub allow_click_restore: bool,
}

/// 全新安装的默认：中键单击隐藏，允许再按一次恢复。
impl Default for MouseSetting {
    fn default() -> Self {
        Self {
            middle: MouseButton {
                enabled: true,
                ..MouseButton::default()
            },
            ..Self::all_off()
        }
    }
}

impl MouseSetting {
    /// 配置文件没有 `mouse` 一节时用的值：全部关闭。
    fn all_off() -> Self {
        Self {
            left: MouseButton::default(),
            middle: MouseButton::default(),
            right: MouseButton::default(),
            side1: MouseButton::default(),
            side2: MouseButton::default(),
            multi_click_ms: default_multi_click_ms(),
            allow_click_restore: true,
        }
    }

    pub fn buttons(&self) -> [&MouseButton; 5] {
        [
            &self.left,
            &self.middle,
            &self.right,
            &self.side1,
            &self.side2,
        ]
    }

    pub fn any_enabled(&self) -> bool {
        self.buttons().iter().any(|b| b.enabled)
    }

    fn normalize(&mut self) {
        for b in [
            &mut self.left,
            &mut self.middle,
            &mut self.right,
            &mut self.side1,
            &mut self.side2,
        ] {
            b.normalize();
        }
        self.multi_click_ms = self
            .multi_click_ms
            .clamp(MIN_MULTI_CLICK_MS, MAX_MULTI_CLICK_MS);
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
    pub freeze_after_hide: bool,
    #[serde(default)]
    pub enhanced_freeze: bool,
    /// 递归冻结命中程序的整棵子进程树（不同名子 exe / 渲染进程等），
    /// 对普通与增强冻结均生效。
    #[serde(default)]
    pub freeze_whole_tree: bool,
    #[serde(default)]
    pub show_float_window: bool,
    /// 鼠标触发条件；缺这一节的老配置读进来是「全关」。
    #[serde(default = "MouseSetting::all_off")]
    pub mouse: MouseSetting,
    /// 旧版扁平鼠标开关，仅用于反序列化迁移；迁移后清零、不再写回文件。
    #[serde(default, skip_serializing)]
    pub middle_button_hide: bool,
    #[serde(default, skip_serializing)]
    pub side_button1_hide: bool,
    #[serde(default, skip_serializing)]
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
    /// 仅快速甩到角落才触发；默认开。
    #[serde(default = "default_true")]
    pub corner_fast_only: bool,
    /// 日志保留天数；`0` 表示关闭日志。
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,
    /// 开机自启是否以管理员身份启动：`true` 注册最高权限计划任务，`false` 用普通权限。
    /// 仅影响计划任务方式；注册表回退始终以普通权限运行。
    #[serde(default)]
    pub autostart_admin: bool,
    /// 界面语言：`auto` 跟随系统，或 `zh-CN`／`en`／`zh-TW`。核心与配置程序共用。
    #[serde(default = "default_language")]
    pub language: String,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            mute_after_hide: true,
            send_before_hide: false,
            hide_current: true,
            click_to_hide: true,
            hide_icon_after_hide: false,
            freeze_after_hide: false,
            enhanced_freeze: false,
            freeze_whole_tree: false,
            show_float_window: false,
            mouse: MouseSetting::default(),
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
            corner_fast_only: true,
            log_retention_days: DEFAULT_LOG_RETENTION_DAYS,
            autostart_admin: false,
            language: default_language(),
        }
    }
}

impl Setting {
    /// 迁移旧版扁平鼠标开关，并把连击次数、连击窗口夹到合法范围。幂等。
    pub fn normalize(&mut self) {
        if !self.mouse.any_enabled() {
            self.mouse.middle.enabled = self.middle_button_hide;
            self.mouse.side1.enabled = self.side_button1_hide;
            self.mouse.side2.enabled = self.side_button2_hide;
        }
        self.middle_button_hide = false;
        self.side_button1_hide = false;
        self.side_button2_hide = false;
        self.mouse.normalize();
        self.language = crate::i18n::normalize_pref(&self.language);
    }
}

/// 通知开关：逐事件控制是否弹出托盘气泡。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notifications {
    /// 核心启动运行时的气泡。
    #[serde(default = "default_true")]
    pub on_start: bool,
    /// 核心退出时的气泡。
    #[serde(default = "default_true")]
    pub on_quit: bool,
    /// 开机自启状态变更时的气泡。
    #[serde(default = "default_true")]
    pub on_autostart: bool,
    /// 每次隐藏窗口时的气泡（默认关闭）。
    #[serde(default)]
    pub on_hide: bool,
    /// 每次显示窗口时的气泡（默认关闭）。
    #[serde(default)]
    pub on_show: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            on_start: true,
            on_quit: true,
            on_autostart: true,
            on_hide: false,
            on_show: false,
        }
    }
}

/// Verhub 相关的用户可见设置。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verhub {
    /// 是否把预览版也算进更新检查；默认只看稳定版。
    #[serde(default)]
    pub include_preview: bool,
    /// 用户已读过的最新一条公告 id。
    #[serde(default)]
    pub seen_announcement_id: String,
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
    pub notifications: Notifications,
    #[serde(default)]
    pub verhub: Verhub,
    /// 「窗口」规则（细粒度）。
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
    /// 「进程」规则（粗粒度）。
    #[serde(default)]
    pub process_rules: Vec<ProcessRule>,
    /// 旧版扁平绑定，仅用于反序列化迁移；迁移后清空、不再序列化。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
            notifications: Notifications::default(),
            verhub: Verhub::default(),
            window_rules: Vec::new(),
            process_rules: Vec::new(),
            hide_binding: Vec::new(),
        }
    }
}

impl Config {
    pub fn from_json(s: &str) -> Result<Self, ConfigError> {
        let mut config: Config = serde_json::from_str(s)?;
        config.normalize();
        Ok(config)
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        Self::load_reporting(path).map(|(config, _)| config)
    }

    /// 同 [`Config::load`]，但额外报告「文件存在却解析失败、已回退默认值」的情况。
    ///
    /// 损坏文件回退默认值是刻意行为（保证核心总能启动），代价是用户的规则会「凭空消失」。
    /// 调用方据第二个返回值把解析错误写进日志，否则这一幕无迹可循。
    /// 返回 `(配置, 解析错误)`；解析成功或文件不存在时第二项为 `None`。
    pub fn load_reporting(path: &Path) -> Result<(Self, Option<String>), ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                let (mut config, parse_error) = match serde_json::from_str::<Config>(&s) {
                    Ok(config) => (config, None),
                    Err(e) => (Config::default(), Some(e.to_string())),
                };
                config.normalize();
                Ok((config, parse_error))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((Config::default(), None)),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    /// 迁移旧版扁平绑定 `hide_binding` 为「窗口」精确规则。幂等。
    pub fn normalize(&mut self) {
        if self.window_rules.is_empty() && !self.hide_binding.is_empty() {
            self.window_rules = self
                .hide_binding
                .iter()
                .map(WindowRule::from_window)
                .collect();
        }
        self.hide_binding.clear();
        self.setting.normalize();
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
    fn parses_full_v21_config_and_migrates_binding() {
        let c = Config::from_json(sample_json()).unwrap();
        assert_eq!(c.version, "v2.1.0.0");
        assert_eq!(c.history, vec![111, 222]);
        assert_eq!(c.frozen_pids, vec![4321]);
        assert_eq!(c.hotkey.hide_hotkey, "Ctrl+Shift+H");
        assert!(!c.setting.mute_after_hide);
        assert_eq!(c.setting.auto_hide_time, 15);
        assert!(c.hide_binding.is_empty(), "迁移后旧字段应清空");
        assert_eq!(c.window_rules.len(), 1);
        assert_eq!(c.window_rules[0].process, "WeChat.exe");
        assert_eq!(c.window_rules[0].pid, 8888);
        assert!(!c.window_rules[0].is_regex(), "迁移出的规则应为精确规则");
    }

    #[test]
    fn intercept_flags_default_off_for_old_configs() {
        let c = Config::from_json(
            r#"{"hotkey": {"hide_hotkey": "Ctrl+Q", "close_hotkey": "Win+Esc"}}"#,
        )
        .unwrap();
        assert!(!c.hotkey.hide_intercept, "旧配置无此字段应默认关闭");
        assert!(!c.hotkey.close_intercept);
        assert!(!Hotkey::default().hide_intercept, "全新配置也默认关闭");
        assert!(!Hotkey::default().close_intercept);
    }

    #[test]
    fn intercept_flags_round_trip_independently() {
        let c = Config::from_json(r#"{"hotkey": {"hide_intercept": true}}"#).unwrap();
        assert!(c.hotkey.hide_intercept);
        assert!(!c.hotkey.close_intercept, "两个开关相互独立");
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert!(back.hotkey.hide_intercept, "写回后应保留");
        assert!(!back.hotkey.close_intercept);
    }

    #[test]
    fn legacy_mouse_flags_migrate_to_single_click_triggers() {
        let c = Config::from_json(
            r#"{"setting": {"middle_button_hide": true, "side_button2_hide": true}}"#,
        )
        .unwrap();
        assert!(c.setting.mouse.middle.enabled);
        assert!(c.setting.mouse.side2.enabled);
        assert!(!c.setting.mouse.side1.enabled);
        assert_eq!(c.setting.mouse.middle.clicks, 1, "旧开关迁移为单击");
        assert!(c.setting.mouse.middle.modifiers.is_empty());
        assert!(!c.setting.middle_button_hide, "迁移后旧字段清零");

        let json = c.to_json().unwrap();
        assert!(
            !json.contains("middle_button_hide"),
            "旧字段不应写回: {json}"
        );
    }

    #[test]
    fn explicit_mouse_block_wins_over_legacy_flags() {
        let c = Config::from_json(
            r#"{"setting": {
                "middle_button_hide": true,
                "mouse": {"left": {"enabled": true, "clicks": 3, "modifiers": "Ctrl"}}
            }}"#,
        )
        .unwrap();
        assert!(c.setting.mouse.left.enabled);
        assert_eq!(c.setting.mouse.left.clicks, 3);
        assert_eq!(c.setting.mouse.left.modifiers, "Ctrl");
        assert!(!c.setting.mouse.middle.enabled, "旧开关不应再迁移");
    }

    #[test]
    fn clicks_and_multi_click_window_are_clamped() {
        let c = Config::from_json(
            r#"{"setting": {"mouse": {
                "left": {"enabled": true, "clicks": 9},
                "right": {"enabled": true, "clicks": 0},
                "multi_click_ms": 5000
            }}}"#,
        )
        .unwrap();
        assert_eq!(c.setting.mouse.left.clicks, MAX_CLICKS);
        assert_eq!(c.setting.mouse.right.clicks, 1);
        assert_eq!(c.setting.mouse.multi_click_ms, MAX_MULTI_CLICK_MS);
    }

    #[test]
    fn fresh_install_enables_middle_button_single_click() {
        let m = Config::default().setting.mouse;
        assert!(m.middle.enabled, "全新安装默认开中键");
        assert!(m.allow_click_restore, "默认允许再按一次恢复");
        assert_eq!(m.multi_click_ms, DEFAULT_MULTI_CLICK_MS);
        assert_eq!(DEFAULT_MULTI_CLICK_MS, 350);
        assert!(
            !m.left.enabled && !m.right.enabled && !m.side1.enabled && !m.side2.enabled,
            "其余四颗键默认关闭"
        );
        assert!(
            m.buttons()
                .iter()
                .all(|b| b.clicks == 1 && b.modifiers.is_empty())
        );
    }

    #[test]
    fn old_config_without_mouse_section_stays_all_off() {
        let c = Config::from_json(r#"{"setting": {"mute_after_hide": true}}"#).unwrap();
        assert!(!c.setting.mouse.any_enabled(), "老配置不该被塞进默认的中键");
        assert!(c.setting.mouse.allow_click_restore);
    }

    #[test]
    fn corner_fast_only_defaults_on() {
        assert!(Setting::default().corner_fast_only);
        let c = Config::from_json(r#"{"setting": {"corner_fast_only": false}}"#).unwrap();
        assert!(!c.setting.corner_fast_only);
    }

    #[test]
    fn missing_setting_keys_use_defaults() {
        let c = Config::from_json(r#"{"setting": {}}"#).unwrap();
        assert!(c.setting.mute_after_hide);
        assert!(c.setting.hide_current);
        assert!(c.setting.click_to_hide);
        assert!(!c.setting.freeze_after_hide);
        assert!(!c.setting.freeze_whole_tree);
        assert_eq!(c.setting.auto_hide_time, 5);
        assert_eq!(c.setting.log_retention_days, 7, "日志保留天数默认 7");
        assert!(!c.setting.autostart_admin, "自启默认普通权限");
    }

    #[test]
    fn autostart_admin_round_trips() {
        assert!(!Setting::default().autostart_admin, "默认关（普通权限）");
        let c = Config::from_json(r#"{"setting": {"autostart_admin": true}}"#).unwrap();
        assert!(c.setting.autostart_admin);
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert!(back.setting.autostart_admin, "写回后应保留");
    }

    /// 已存在的配置文件读出来的「什么都没配」：除 mouse 全关外，其余同默认值。
    fn setting_from_old_file() -> Setting {
        Setting {
            mouse: MouseSetting::all_off(),
            ..Setting::default()
        }
    }

    #[test]
    fn legacy_path_match_key_is_ignored() {
        let c = Config::from_json(r#"{"setting": {"path_match": true}}"#).unwrap();
        assert_eq!(c.setting, setting_from_old_file());
    }

    #[test]
    fn migration_is_idempotent_and_preserves_existing_rules() {
        let json = r#"{
            "window_rules": [{"title": "已存在", "hwnd": 1, "process": "a.exe", "PID": 2, "path": "C:\\a.exe"}],
            "hide_binding": [{"title": "旧的", "hwnd": 9, "process": "b.exe", "PID": 8, "path": "C:\\b.exe"}]
        }"#;
        let c = Config::from_json(json).unwrap();
        assert_eq!(c.window_rules.len(), 1);
        assert_eq!(c.window_rules[0].title, "已存在");
        assert!(c.hide_binding.is_empty());
    }

    #[test]
    fn process_rules_round_trip() {
        let json = r#"{
            "process_rules": [
                {"process": "game.exe", "path": "C:\\game.exe"},
                {"regex": ".*\\\\WeChat\\.exe$"}
            ]
        }"#;
        let c = Config::from_json(json).unwrap();
        assert_eq!(c.process_rules.len(), 2);
        assert!(!c.process_rules[0].is_regex());
        assert!(c.process_rules[1].is_regex());
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
    fn serialized_window_rule_uses_uppercase_pid() {
        let mut c = Config::default();
        c.window_rules
            .push(WindowRule::from_window(&WindowInfo::new(
                "t",
                1,
                "p.exe",
                77,
                "C:\\p.exe",
            )));
        let json = c.to_json().unwrap();
        assert!(json.contains("\"PID\": 77"), "应保留大写 PID: {json}");
    }

    #[test]
    fn empty_hide_binding_is_not_serialized() {
        let json = Config::default().to_json().unwrap();
        assert!(
            !json.contains("hide_binding"),
            "空的旧字段不应写入文件: {json}"
        );
        assert!(json.contains("window_rules"));
        assert!(json.contains("process_rules"));
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let c = Config::from_json(r#"{"future_flag": true, "setting": {"brand_new": 1}}"#).unwrap();
        assert_eq!(c.setting, setting_from_old_file());
    }
}
