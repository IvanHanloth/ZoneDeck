use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::APP_CONFIG_VERSION;
use crate::i18n::LANG_AUTO;
use crate::model::{ProcessRule, WhitelistRule, WindowInfo, WindowRule};

pub const DEFAULT_HIDE_HOTKEY: &str = "Ctrl+Q";
pub const DEFAULT_CLOSE_HOTKEY: &str = "Win+Esc";
pub const DEFAULT_AUTO_HIDE_TIME: u32 = 5;
/// 日志默认保留天数（`0` 表示关闭日志）。
pub const DEFAULT_LOG_RETENTION_DAYS: u32 = 7;
/// 日志输出等级的取值。低于所选等级的日志不写入文件。
pub const LOG_LEVEL_DEBUG: &str = "debug";
pub const LOG_LEVEL_INFO: &str = "info";
pub const LOG_LEVEL_WARN: &str = "warn";
pub const LOG_LEVEL_ERROR: &str = "error";
/// 由低到高的全部合法等级。
pub const LOG_LEVELS: [&str; 4] = [
    LOG_LEVEL_DEBUG,
    LOG_LEVEL_INFO,
    LOG_LEVEL_WARN,
    LOG_LEVEL_ERROR,
];
/// 默认输出等级：只记录警告及以上。
pub const DEFAULT_LOG_LEVEL: &str = LOG_LEVEL_WARN;
/// 连击判定窗口默认值（毫秒）：两次点击间隔不超过它才算连击。
pub const DEFAULT_MULTI_CLICK_MS: u32 = 350;
pub const MIN_MULTI_CLICK_MS: u32 = 150;
pub const MAX_MULTI_CLICK_MS: u32 = 1000;
/// 最多支持三连击。
pub const MAX_CLICKS: u8 = 3;

/// 能效控制的作用范围：冻结与清空工作集共用。仅命中窗口所属的进程本身。
pub const POWER_SCOPE_SELF: &str = "self";
/// 目标进程及其全部后代进程（不同名的子 exe、渲染进程等）。
pub const POWER_SCOPE_TREE: &str = "tree";
/// 与目标进程同一映像名的所有进程，不看亲缘关系。
pub const POWER_SCOPE_IMAGE: &str = "image";
/// 全部合法取值。
pub const POWER_SCOPES: [&str; 3] = [POWER_SCOPE_SELF, POWER_SCOPE_TREE, POWER_SCOPE_IMAGE];

/// 归一作用范围：忽略大小写与首尾空白，无法识别时回落 [`POWER_SCOPE_SELF`]。
pub fn normalize_power_scope(value: &str) -> String {
    let v = value.trim().to_ascii_lowercase();
    if POWER_SCOPES.contains(&v.as_str()) {
        v
    } else {
        POWER_SCOPE_SELF.to_string()
    }
}

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
fn default_log_level() -> String {
    DEFAULT_LOG_LEVEL.to_string()
}

/// 归一日志等级：忽略大小写与首尾空白，兼容 `warning`；无法识别时回落默认值。
pub fn normalize_log_level(value: &str) -> String {
    let v = value.trim().to_ascii_lowercase();
    let v = if v == "warning" {
        LOG_LEVEL_WARN.to_string()
    } else {
        v
    };
    if LOG_LEVELS.contains(&v.as_str()) {
        v
    } else {
        DEFAULT_LOG_LEVEL.to_string()
    }
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
    #[error("配置文件读写错误: {source}（路径: {path}）")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("配置文件 JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),
}

impl ConfigError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    #[serde(default = "default_hide_hotkey")]
    pub hide_hotkey: String,
    #[serde(default = "default_close_hotkey")]
    pub close_hotkey: String,
    /// 只隐藏、不恢复的热键；默认置空（关闭）。
    #[serde(default)]
    pub hide_only_hotkey: String,
    /// 只恢复、不隐藏的热键；默认置空（关闭）。
    #[serde(default)]
    pub show_only_hotkey: String,
    /// 只隐藏当前前台窗口的热键，可连续触发逐个隐藏；默认置空（关闭）。
    #[serde(default)]
    pub hide_foreground_hotkey: String,
    /// 隐藏热键是否不传递给其他程序（核心改用键盘钩子拦截）。
    #[serde(default)]
    pub hide_intercept: bool,
    /// 关闭热键是否不传递给其他程序（核心改用键盘钩子拦截）。
    #[serde(default)]
    pub close_intercept: bool,
    /// 仅隐藏热键是否不传递给其他程序。
    #[serde(default)]
    pub hide_only_intercept: bool,
    /// 仅显示热键是否不传递给其他程序。
    #[serde(default)]
    pub show_only_intercept: bool,
    /// 隐藏前台窗口热键是否不传递给其他程序。
    #[serde(default)]
    pub hide_foreground_intercept: bool,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            hide_hotkey: default_hide_hotkey(),
            close_hotkey: default_close_hotkey(),
            hide_only_hotkey: String::new(),
            show_only_hotkey: String::new(),
            hide_foreground_hotkey: String::new(),
            hide_intercept: false,
            close_intercept: false,
            hide_only_intercept: false,
            show_only_intercept: false,
            hide_foreground_intercept: false,
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

/// 全新安装的默认：中键双击隐藏，允许再按一次恢复。
impl Default for MouseSetting {
    fn default() -> Self {
        Self {
            middle: MouseButton {
                enabled: true,
                clicks: 2,
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

/// 托盘角标可绑定的状态源取值（空串 = 不显示该颜色）。
pub const TRAY_STATUS_HIDDEN: &str = "hidden";
pub const TRAY_STATUS_AUTO_HIDE: &str = "auto_hide";
pub const TRAY_STATUS_HIDE_CURRENT: &str = "hide_current";
pub const TRAY_STATUS_FREEZE: &str = "freeze";
pub const TRAY_STATUS_ELEVATED: &str = "elevated";
pub const TRAY_STATUS_MONITOR_PAUSED: &str = "monitor_paused";
/// 全部合法的非空状态源。
pub const TRAY_STATUSES: [&str; 6] = [
    TRAY_STATUS_HIDDEN,
    TRAY_STATUS_AUTO_HIDE,
    TRAY_STATUS_HIDE_CURRENT,
    TRAY_STATUS_FREEZE,
    TRAY_STATUS_ELEVATED,
    TRAY_STATUS_MONITOR_PAUSED,
];

fn default_badge_red() -> String {
    TRAY_STATUS_HIDDEN.to_string()
}
fn default_badge_green() -> String {
    TRAY_STATUS_AUTO_HIDE.to_string()
}
fn default_badge_yellow() -> String {
    TRAY_STATUS_HIDE_CURRENT.to_string()
}
fn default_badge_blue() -> String {
    TRAY_STATUS_FREEZE.to_string()
}

/// 托盘图标状态角标：四种颜色各自绑定一个状态源，多个同时活跃时按
/// 红 > 绿 > 黄 > 蓝 只显示一个；置空表示不显示该颜色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayBadges {
    #[serde(default = "default_badge_red")]
    pub red: String,
    #[serde(default = "default_badge_green")]
    pub green: String,
    #[serde(default = "default_badge_yellow")]
    pub yellow: String,
    #[serde(default = "default_badge_blue")]
    pub blue: String,
}

impl Default for TrayBadges {
    fn default() -> Self {
        Self {
            red: default_badge_red(),
            green: default_badge_green(),
            yellow: default_badge_yellow(),
            blue: default_badge_blue(),
        }
    }
}

impl TrayBadges {
    /// 未知状态源归一为置空。幂等。
    pub fn normalize(&mut self) {
        for v in [
            &mut self.red,
            &mut self.green,
            &mut self.yellow,
            &mut self.blue,
        ] {
            if !v.is_empty() && !TRAY_STATUSES.contains(&v.as_str()) {
                v.clear();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setting {
    #[serde(default = "default_true")]
    pub mute_after_hide: bool,
    #[serde(default)]
    pub send_before_hide: bool,
    /// 隐藏前先把窗口最小化，恢复时还原成隐藏前的形态；本就最小化的保持最小化。
    #[serde(default)]
    pub minimize_before_hide: bool,
    #[serde(default = "default_true")]
    pub hide_current: bool,
    #[serde(default = "default_true")]
    pub click_to_hide: bool,
    #[serde(default)]
    pub hide_icon_after_hide: bool,
    /// 托盘图标状态角标的颜色绑定，见 [`TrayBadges`]。
    #[serde(default)]
    pub tray_badges: TrayBadges,
    /// 是否显示托盘图标的悬浮名称（ZoneDeck）；关闭后悬停不显示任何文字。
    #[serde(default = "default_true")]
    pub tray_show_tooltip: bool,
    #[serde(default)]
    pub freeze_after_hide: bool,
    #[serde(default)]
    pub enhanced_freeze: bool,
    /// 能效控制的作用范围（[`POWER_SCOPE_SELF`] 等），同时决定冻结与清空工作集
    /// 覆盖到哪些进程。缺省为空串，由 [`Setting::normalize`] 填上。
    #[serde(default)]
    pub power_scope: String,
    /// 仅用于反序列化迁移，迁移后清零、不再写回文件。
    #[serde(default, skip_serializing)]
    pub freeze_whole_tree: bool,
    /// 冻结后清空进程工作集，压低内存占用。仅对被冻结的进程生效。
    #[serde(default)]
    pub trim_memory_after_freeze: bool,
    #[serde(default)]
    pub show_float_window: bool,
    /// 鼠标触发条件；缺这一节时全关。
    #[serde(default = "MouseSetting::all_off")]
    pub mouse: MouseSetting,
    /// 仅用于反序列化迁移，迁移后清零、不再写回文件。
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
    /// 日志输出等级：`debug`／`info`／`warn`／`error`，低于它的日志不写入文件。
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// 开机自启是否以管理员身份启动，仅影响计划任务方式。
    #[serde(default)]
    pub autostart_admin: bool,
    /// 界面语言：`auto`／`zh-CN`／`en`／`zh-TW`，核心与配置程序共用。
    #[serde(default = "default_language")]
    pub language: String,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            mute_after_hide: true,
            send_before_hide: false,
            minimize_before_hide: false,
            hide_current: true,
            click_to_hide: true,
            hide_icon_after_hide: false,
            tray_badges: TrayBadges::default(),
            tray_show_tooltip: true,
            freeze_after_hide: false,
            enhanced_freeze: false,
            power_scope: POWER_SCOPE_SELF.to_string(),
            freeze_whole_tree: false,
            trim_memory_after_freeze: false,
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
            log_level: default_log_level(),
            autostart_admin: false,
            language: default_language(),
        }
    }
}

impl Setting {
    /// 迁移旧版扁平鼠标开关与「冻结完整进程」，并把连击次数、连击窗口夹到合法范围。幂等。
    pub fn normalize(&mut self) {
        if !self.mouse.any_enabled() {
            self.mouse.middle.enabled = self.middle_button_hide;
            self.mouse.side1.enabled = self.side_button1_hide;
            self.mouse.side2.enabled = self.side_button2_hide;
        }
        self.middle_button_hide = false;
        self.side_button1_hide = false;
        self.side_button2_hide = false;
        // 空串表示配置文件里没有这个键，此时才看旧开关。
        if self.power_scope.is_empty() {
            self.power_scope = if self.freeze_whole_tree {
                POWER_SCOPE_TREE
            } else {
                POWER_SCOPE_SELF
            }
            .to_string();
        }
        self.freeze_whole_tree = false;
        self.power_scope = normalize_power_scope(&self.power_scope);
        self.tray_badges.normalize();
        self.mouse.normalize();
        self.language = crate::i18n::normalize_pref(&self.language);
        self.log_level = normalize_log_level(&self.log_level);
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
    /// 配置 schema 版本（[`APP_CONFIG_VERSION`]），结构变动时才动。
    #[serde(default = "default_version")]
    pub version: String,
    /// 上次运行过的程序版本（[`crate::APP_VERSION`]）；与之不符即「更新后首次启动」。
    #[serde(default)]
    pub app_version: String,
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
    /// 白名单：逐进程声明忽略隐藏 / 冻结 / 静音，见 [`Config::whitelist`]。
    /// `None` 表示文件里没有这个键，由 [`Config::normalize`] 播种默认项；
    /// 归一后恒为 `Some`。
    #[serde(default)]
    pub whitelist: Option<Vec<WhitelistRule>>,
    /// 仅用于反序列化迁移，迁移后清空、不再序列化。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hide_binding: Vec<WindowInfo>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            app_version: String::new(),
            history: Vec::new(),
            frozen_pids: Vec::new(),
            hotkey: Hotkey::default(),
            setting: Setting::default(),
            notifications: Notifications::default(),
            verhub: Verhub::default(),
            window_rules: Vec::new(),
            process_rules: Vec::new(),
            whitelist: Some(default_whitelist()),
            hide_binding: Vec::new(),
        }
    }
}

/// 全新配置预置的白名单：文件资源管理器就是桌面与任务栏本身，隐藏或冻结它会让
/// 外壳失效。这是普通条目，用户可以删掉。
/// ZoneDeck 自身的强制保护见 [`crate::matching::BUILTIN_FREEZE_GUARDS`]。
fn default_whitelist() -> Vec<WhitelistRule> {
    vec![WhitelistRule {
        process: "explorer.exe".to_string(),
        path: String::new(),
        regex: None,
        by_name: true,
        ignore_hide: true,
        ignore_freeze: true,
        ignore_mute: false,
    }]
}

impl Config {
    pub fn from_json(s: &str) -> Result<Self, ConfigError> {
        Self::from_value(serde_json::from_str(s)?)
    }

    /// 同 [`Config::from_json`]，入参为已解析的 JSON 值。反序列化前先剥离 `null`，
    /// 按「字段缺失」回落默认值。
    pub fn from_value(mut value: serde_json::Value) -> Result<Self, ConfigError> {
        strip_nulls(&mut value);
        let mut config: Config = serde_json::from_value(value)?;
        config.normalize();
        Ok(config)
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 无副作用的加载：解析失败回退默认值，原文件保持原样，回退原因被丢弃。
    /// 把结果作为本次生效配置时应改用 [`Config::load_reporting`]。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(Self::from_json(&s).unwrap_or_default()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(ConfigError::io(path, e)),
        }
    }

    /// 同 [`Config::load`]，但额外报告「文件存在却解析失败、已回退默认值」的情况。
    /// 解析失败的原文件改名为同目录 `*.bad` 备份，去向包含在报告里。
    /// 返回 `(配置, 回退原因)`；解析成功或文件不存在时第二项为 `None`。
    pub fn load_reporting(path: &Path) -> Result<(Self, Option<String>), ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(s) => match Self::from_json(&s) {
                Ok(config) => Ok((config, None)),
                Err(e) => Ok((Config::default(), Some(quarantine_corrupt(path, &e)))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((Config::default(), None)),
            Err(e) => Err(ConfigError::io(path, e)),
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
        // 缺这一节的配置播种默认白名单；`[]` 是用户清空的结果，不再播种。
        if self.whitelist.is_none() {
            self.whitelist = Some(default_whitelist());
        }
        self.setting.normalize();
    }

    /// 当前生效的白名单；未归一的配置回落空表。
    pub fn whitelist(&self) -> &[WhitelistRule] {
        self.whitelist.as_deref().unwrap_or_default()
    }

    /// 写入配置：先写同目录临时文件再原子替换。
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let json = self.to_json()?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::io(parent, e))?;
        }
        let tmp = tmp_path(path);
        std::fs::write(&tmp, json).map_err(|e| ConfigError::io(&tmp, e))?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            ConfigError::io(path, e)
        })
    }
}

/// 递归剥离 JSON 中的 `null`：对象里的键按「字段缺失」处理，数组里的元素丢弃。
fn strip_nulls(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            for v in map.values_mut() {
                strip_nulls(v);
            }
        }
        serde_json::Value::Array(items) => {
            items.retain(|v| !v.is_null());
            for v in items.iter_mut() {
                strip_nulls(v);
            }
        }
        _ => {}
    }
}

/// 原子写入用的临时文件路径（与目标同目录，rename 才是原子的）。
fn tmp_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

/// 把解析失败的配置文件改名为同目录 `*.bad` 备份，返回供调用方记日志的回退原因。
/// 备份失败不阻断回退，但原因里会注明数据仍会被随后的保存覆写。
fn quarantine_corrupt(path: &Path, parse_error: &ConfigError) -> String {
    let backup = bad_path(path);
    match std::fs::rename(path, &backup) {
        Ok(()) => format!("{parse_error}（原文件已备份为 {}）", backup.display()),
        Err(e) => format!("{parse_error}（备份原文件失败，随后的保存会将其覆盖: {e}）"),
    }
}

/// 损坏配置的备份路径（与原文件同目录，文件名追加 `.bad`）。
fn bad_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bad");
    path.with_file_name(name)
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
    fn separate_hide_show_hotkeys_default_to_disabled() {
        let h = Hotkey::default();
        assert!(h.hide_only_hotkey.is_empty(), "仅隐藏热键默认置空");
        assert!(h.show_only_hotkey.is_empty(), "仅显示热键默认置空");
        assert!(h.hide_foreground_hotkey.is_empty(), "隐藏前台热键默认置空");
        assert!(!h.hide_only_intercept);
        assert!(!h.show_only_intercept);
        assert!(!h.hide_foreground_intercept);

        let c = Config::from_json(r#"{"hotkey": {"hide_hotkey": "Ctrl+Q"}}"#).unwrap();
        assert!(c.hotkey.hide_only_hotkey.is_empty(), "旧配置无此字段应置空");
        assert!(c.hotkey.show_only_hotkey.is_empty());
        assert!(c.hotkey.hide_foreground_hotkey.is_empty());
    }

    #[test]
    fn separate_hide_show_hotkeys_round_trip() {
        let c = Config::from_json(
            r#"{"hotkey": {
                "hide_only_hotkey": "Ctrl+Alt+H",
                "show_only_hotkey": "Ctrl+Alt+S",
                "hide_foreground_hotkey": "Ctrl+Alt+F",
                "hide_foreground_intercept": true
            }}"#,
        )
        .unwrap();
        assert_eq!(c.hotkey.hide_only_hotkey, "Ctrl+Alt+H");
        assert_eq!(c.hotkey.show_only_hotkey, "Ctrl+Alt+S");
        assert_eq!(c.hotkey.hide_foreground_hotkey, "Ctrl+Alt+F");
        assert!(c.hotkey.hide_foreground_intercept);
        assert!(!c.hotkey.hide_only_intercept, "各热键的开关相互独立");

        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back.hotkey, c.hotkey, "写回后应保留");
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
    fn fresh_install_enables_middle_button_double_click() {
        let m = Config::default().setting.mouse;
        assert!(m.middle.enabled, "全新安装默认开中键");
        assert_eq!(m.middle.clicks, 2, "全新安装默认中键双击");
        assert!(m.middle.modifiers.is_empty());
        assert!(m.allow_click_restore, "默认允许再按一次恢复");
        assert_eq!(m.multi_click_ms, DEFAULT_MULTI_CLICK_MS);
        assert_eq!(DEFAULT_MULTI_CLICK_MS, 350);
        assert!(
            !m.left.enabled && !m.right.enabled && !m.side1.enabled && !m.side2.enabled,
            "其余四颗键默认关闭"
        );
        assert!(
            [&m.left, &m.right, &m.side1, &m.side2]
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
        assert_eq!(
            c.setting.power_scope, POWER_SCOPE_SELF,
            "作用范围默认只管自己"
        );
        assert!(!c.setting.minimize_before_hide, "隐藏前先最小化默认关闭");
        assert!(!c.setting.trim_memory_after_freeze, "降低内存占用默认关闭");
        assert_eq!(c.setting.auto_hide_time, 5);
        assert_eq!(c.setting.log_retention_days, 7, "日志保留天数默认 7");
        assert_eq!(c.setting.log_level, "warn", "日志等级默认只记警告及以上");
        assert!(!c.setting.autostart_admin, "自启默认普通权限");
    }

    /// 数字输入框被清空时会提交 `null`，不应导致整份配置保存失败。
    #[test]
    fn null_numeric_fields_fall_back_to_defaults() {
        let c = Config::from_json(
            r#"{"setting": {
                "auto_hide_time": null,
                "log_retention_days": null,
                "mouse": {
                    "multi_click_ms": null,
                    "left": {"enabled": true, "clicks": null}
                }
            }}"#,
        )
        .unwrap();
        assert_eq!(c.setting.auto_hide_time, DEFAULT_AUTO_HIDE_TIME);
        assert_eq!(c.setting.log_retention_days, DEFAULT_LOG_RETENTION_DAYS);
        assert_eq!(c.setting.mouse.multi_click_ms, DEFAULT_MULTI_CLICK_MS);
        assert_eq!(c.setting.mouse.left.clicks, 1);
        assert!(c.setting.mouse.left.enabled, "同级字段不受影响");
    }

    /// 覆盖全部会写入文件的字段（含规则数组元素）的完整样例。
    fn full_sample_config() -> Config {
        let w = WindowInfo::new("窗口", 42, "app.exe", 100, "D:\\app.exe");
        Config {
            history: vec![111],
            frozen_pids: vec![222],
            window_rules: vec![WindowRule::from_window(&w), WindowRule::from_regex("^a.*")],
            process_rules: vec![
                ProcessRule::from_window(&w),
                ProcessRule::from_regex(".*\\.exe$"),
            ],
            whitelist: Some(vec![
                WhitelistRule::from_window(&w),
                WhitelistRule::from_regex("^b.*"),
            ]),
            ..Config::default()
        }
    }

    /// 收集 JSON 中的全部路径（对象键与数组下标，含中间节点）。
    fn collect_paths(value: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let path = format!("{prefix}/{k}");
                    out.push(path.clone());
                    collect_paths(v, &path, out);
                }
            }
            serde_json::Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    let path = format!("{prefix}/{i}");
                    out.push(path.clone());
                    collect_paths(v, &path, out);
                }
            }
            _ => {}
        }
    }

    fn set_null(value: &mut serde_json::Value, path: &str) {
        let mut cur = value;
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            let last = i == parts.len() - 1;
            match cur {
                serde_json::Value::Object(map) => {
                    let v = map.get_mut(*part).unwrap();
                    if last {
                        *v = serde_json::Value::Null;
                        return;
                    }
                    cur = v;
                }
                serde_json::Value::Array(items) => {
                    let v = &mut items[part.parse::<usize>().unwrap()];
                    if last {
                        *v = serde_json::Value::Null;
                        return;
                    }
                    cur = v;
                }
                _ => unreachable!("路径中间节点必须是对象或数组: {path}"),
            }
        }
    }

    /// 配置里任意字段为 `null` 时读取与保存都不得失败。
    #[test]
    fn any_field_set_to_null_still_parses() {
        let base: serde_json::Value =
            serde_json::from_str(&full_sample_config().to_json().unwrap()).unwrap();
        let mut paths = Vec::new();
        collect_paths(&base, "", &mut paths);
        assert!(paths.len() > 60, "样例应覆盖全部字段，当前 {}", paths.len());
        for path in paths {
            let mut v = base.clone();
            set_null(&mut v, &path);
            let r = Config::from_json(&v.to_string());
            assert!(r.is_ok(), "字段 {path} 为 null 时解析失败: {r:?}");
        }
    }

    /// 整份配置为 `null` 或非对象时按损坏处理。
    #[test]
    fn top_level_null_is_still_an_error() {
        assert!(Config::from_json("null").is_err());
    }

    /// [`Config::from_value`] 与 [`Config::from_json`] 行为一致。
    #[test]
    fn from_value_strips_nulls_like_from_json() {
        let v = serde_json::json!({"setting": {"auto_hide_time": null}});
        let c = Config::from_value(v).unwrap();
        assert_eq!(c.setting.auto_hide_time, DEFAULT_AUTO_HIDE_TIME);
    }

    /// 数组里的 `null` 元素直接丢弃，其余元素保留。
    #[test]
    fn null_array_elements_are_dropped() {
        let c = Config::from_json(r#"{"history": [1, null, 2], "frozen_pids": [null]}"#).unwrap();
        assert_eq!(c.history, vec![1, 2]);
        assert!(c.frozen_pids.is_empty());
    }

    #[test]
    fn log_level_round_trips_and_normalizes() {
        assert_eq!(Setting::default().log_level, LOG_LEVEL_WARN);

        let c = Config::from_json(r#"{"setting": {"log_level": "debug"}}"#).unwrap();
        assert_eq!(c.setting.log_level, LOG_LEVEL_DEBUG);
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back.setting.log_level, LOG_LEVEL_DEBUG, "写回后应保留");

        assert_eq!(
            normalize_log_level(" INFO "),
            LOG_LEVEL_INFO,
            "忽略大小写与空白"
        );
        assert_eq!(
            normalize_log_level("warning"),
            LOG_LEVEL_WARN,
            "兼容 warning"
        );
        assert_eq!(
            normalize_log_level("verbose"),
            DEFAULT_LOG_LEVEL,
            "未知等级回落默认值"
        );
        let c = Config::from_json(r#"{"setting": {"log_level": "verbose"}}"#).unwrap();
        assert_eq!(c.setting.log_level, DEFAULT_LOG_LEVEL);
    }

    #[test]
    fn tray_badges_default_bindings() {
        let d = TrayBadges::default();
        assert_eq!(
            d.red, TRAY_STATUS_HIDDEN,
            "红色默认绑定「存在隐藏中的窗口」"
        );
        assert_eq!(
            d.green, TRAY_STATUS_AUTO_HIDE,
            "绿色默认绑定「启用了自动隐藏」"
        );
        assert_eq!(
            d.yellow, TRAY_STATUS_HIDE_CURRENT,
            "黄色默认绑定「同时隐藏当前窗口」"
        );
        assert_eq!(d.blue, TRAY_STATUS_FREEZE, "蓝色默认绑定「启用了进程冻结」");
        assert!(Setting::default().tray_show_tooltip, "悬浮名称默认显示");
    }

    #[test]
    fn tray_badges_round_trip_including_empty() {
        let c = Config::from_json(
            r#"{"setting": {"tray_badges": {"red": "", "green": "freeze"}, "tray_show_tooltip": false}}"#,
        )
        .unwrap();
        assert_eq!(c.setting.tray_badges.red, "", "置空表示不显示该颜色");
        assert_eq!(c.setting.tray_badges.green, TRAY_STATUS_FREEZE);
        assert_eq!(
            c.setting.tray_badges.yellow, TRAY_STATUS_HIDE_CURRENT,
            "缺失的颜色用默认绑定"
        );
        assert!(!c.setting.tray_show_tooltip);
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(
            back.setting.tray_badges, c.setting.tray_badges,
            "写回后应保留"
        );
        assert!(!back.setting.tray_show_tooltip, "写回后应保留");
    }

    #[test]
    fn tray_badges_unknown_status_normalizes_to_empty() {
        let c = Config::from_json(r#"{"setting": {"tray_badges": {"red": "no_such_status"}}}"#)
            .unwrap();
        assert_eq!(c.setting.tray_badges.red, "", "未知状态源应归一为置空");
        assert_eq!(
            c.setting.tray_badges.blue, TRAY_STATUS_FREEZE,
            "其余颜色不受影响"
        );
    }

    #[test]
    fn app_version_is_recorded_and_defaults_to_empty() {
        assert_eq!(
            Config::default().app_version,
            "",
            "默认值不写死当前版本：全新配置也要走一次「首次启动」流程"
        );
        let c = Config::from_json(r#"{"setting": {}}"#).unwrap();
        assert_eq!(c.app_version, "", "老配置没有该字段，视为未记录过");

        let c = Config {
            app_version: "3.1.0".to_string(),
            ..Config::default()
        };
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back.app_version, "3.1.0", "写回后应保留");
    }

    #[test]
    fn schema_version_and_app_version_are_separate_fields() {
        let json = Config::default().to_json().unwrap();
        assert!(json.contains("\"version\""), "配置 schema 版本仍要写出");
        assert!(json.contains("\"app_version\""), "程序版本单独记一份");
    }

    #[test]
    fn autostart_admin_round_trips() {
        assert!(!Setting::default().autostart_admin, "默认关（普通权限）");
        let c = Config::from_json(r#"{"setting": {"autostart_admin": true}}"#).unwrap();
        assert!(c.setting.autostart_admin);
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert!(back.setting.autostart_admin, "写回后应保留");
    }

    #[test]
    fn minimize_and_trim_memory_round_trip() {
        let c = Config::from_json(
            r#"{"setting": {"minimize_before_hide": true, "trim_memory_after_freeze": true}}"#,
        )
        .unwrap();
        assert!(c.setting.minimize_before_hide);
        assert!(c.setting.trim_memory_after_freeze);

        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert!(back.setting.minimize_before_hide, "写回后应保留");
        assert!(back.setting.trim_memory_after_freeze, "写回后应保留");

        // 老配置没有这两个键，两项功能都不该被打开。
        let old = Config::from_json(r#"{"setting": {"mute_after_hide": true}}"#).unwrap();
        assert!(!old.setting.minimize_before_hide);
        assert!(!old.setting.trim_memory_after_freeze);
    }

    #[test]
    fn power_scope_round_trips_and_normalizes() {
        assert_eq!(Setting::default().power_scope, POWER_SCOPE_SELF);

        for scope in POWER_SCOPES {
            let c = Config::from_json(&format!(r#"{{"setting": {{"power_scope": "{scope}"}}}}"#))
                .unwrap();
            assert_eq!(c.setting.power_scope, scope);
            let back = Config::from_json(&c.to_json().unwrap()).unwrap();
            assert_eq!(back.setting.power_scope, scope, "写回后应保留");
        }

        assert_eq!(
            normalize_power_scope(" TREE "),
            POWER_SCOPE_TREE,
            "忽略大小写与空白"
        );
        assert_eq!(
            normalize_power_scope("everything"),
            POWER_SCOPE_SELF,
            "未知范围回落到最保守的「仅目标进程」"
        );
        let c = Config::from_json(r#"{"setting": {"power_scope": "everything"}}"#).unwrap();
        assert_eq!(c.setting.power_scope, POWER_SCOPE_SELF);
    }

    /// 「冻结完整进程」迁移为作用范围，且不再写回文件。
    #[test]
    fn legacy_freeze_whole_tree_migrates_to_power_scope() {
        let c = Config::from_json(r#"{"setting": {"freeze_whole_tree": true}}"#).unwrap();
        assert_eq!(c.setting.power_scope, POWER_SCOPE_TREE);
        assert!(!c.setting.freeze_whole_tree, "迁移后旧字段清零");

        let json = c.to_json().unwrap();
        assert!(
            !json.contains("freeze_whole_tree"),
            "旧字段不应写回: {json}"
        );

        let off = Config::from_json(r#"{"setting": {"freeze_whole_tree": false}}"#).unwrap();
        assert_eq!(off.setting.power_scope, POWER_SCOPE_SELF);
    }

    /// 已显式配过范围的文件不受旧开关影响。
    #[test]
    fn explicit_power_scope_wins_over_legacy_flag() {
        let c = Config::from_json(
            r#"{"setting": {"freeze_whole_tree": true, "power_scope": "image"}}"#,
        )
        .unwrap();
        assert_eq!(c.setting.power_scope, POWER_SCOPE_IMAGE);
    }

    /// 「什么都没配」的配置文件：除 mouse 全关外，其余同默认值。
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

    /// 没有 `whitelist` 键时播种默认项。
    #[test]
    fn missing_whitelist_is_seeded_with_explorer() {
        let c = Config::from_json(r#"{"setting": {}}"#).unwrap();
        assert_eq!(c.whitelist().len(), 1);
        let rule = &c.whitelist()[0];
        assert_eq!(rule.process, "explorer.exe");
        assert!(rule.by_name, "按文件名匹配，不锁死安装目录");
        assert!(rule.ignore_hide, "隐藏它会连桌面图标一起没掉");
        assert!(rule.ignore_freeze, "冻结它会让整个外壳卡住");
        assert!(!rule.ignore_mute);
        assert_eq!(Config::default().whitelist(), c.whitelist(), "全新配置一致");
    }

    /// 用户清空白名单后必须保持为空。
    #[test]
    fn emptied_whitelist_is_not_reseeded() {
        let c = Config::from_json(r#"{"whitelist": []}"#).unwrap();
        assert!(c.whitelist().is_empty());
        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert!(back.whitelist().is_empty(), "写回再读仍应为空");
    }

    #[test]
    fn whitelist_round_trips_all_three_modes() {
        let c = Config::from_json(
            r#"{"whitelist": [
                {"process": "a.exe", "ignore_hide": true, "ignore_mute": true},
                {"regex": "(?i)^b", "by_name": false, "ignore_freeze": true}
            ]}"#,
        )
        .unwrap();
        assert_eq!(c.whitelist().len(), 2);
        assert!(c.whitelist()[0].ignore_hide && c.whitelist()[0].ignore_mute);
        assert!(!c.whitelist()[0].ignore_freeze, "三个开关相互独立");
        assert!(c.whitelist()[1].is_regex() && !c.whitelist()[1].by_name);

        let back = Config::from_json(&c.to_json().unwrap()).unwrap();
        assert_eq!(back.whitelist, c.whitelist, "写回后应保留");
    }

    /// 归一后 `whitelist` 恒为 `Some`，写出的永远是数组。
    #[test]
    fn whitelist_is_always_written_as_an_array() {
        let json = Config::from_json(r#"{"whitelist": []}"#)
            .unwrap()
            .to_json()
            .unwrap();
        assert!(json.contains("\"whitelist\": []"), "应写出空数组: {json}");
        assert!(!json.contains("\"whitelist\": null"));
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
