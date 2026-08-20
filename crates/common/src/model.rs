use serde::{Deserialize, Serialize};

use crate::NO_TITLE;

fn default_title() -> String {
    NO_TITLE.to_string()
}

fn de_null_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(de)?.unwrap_or_default())
}

fn de_title<'de, D>(de: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(de)?.unwrap_or_else(default_title))
}

fn default_visible() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    #[serde(default = "default_title", deserialize_with = "de_title")]
    pub title: String,
    #[serde(default, deserialize_with = "de_null_default")]
    pub hwnd: i64,
    #[serde(default, deserialize_with = "de_null_default")]
    pub process: String,
    #[serde(rename = "PID", default, deserialize_with = "de_null_default")]
    pub pid: u32,
    #[serde(default, deserialize_with = "de_null_default")]
    pub path: String,
    /// 枚举时该窗口是否可见；不参与身份判定。
    #[serde(default = "default_visible")]
    pub visible: bool,
}

/// 窗口身份只由标题/句柄/进程/PID/路径决定，不含 `visible`。
impl PartialEq for WindowInfo {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.hwnd == other.hwnd
            && self.process == other.process
            && self.pid == other.pid
            && self.path == other.path
    }
}

impl Eq for WindowInfo {}

impl WindowInfo {
    pub fn new(
        title: impl Into<String>,
        hwnd: i64,
        process: impl Into<String>,
        pid: u32,
        path: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            hwnd,
            process: process.into(),
            pid,
            path: path.into(),
            visible: true,
        }
    }

    /// 设置可见性（枚举时使用），返回自身便于链式构造。
    pub fn with_visibility(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// 「窗口」规则（细粒度）：按句柄 + 标题锁定单个窗口；`regex` 为 `Some` 时按标题正则命中。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRule {
    #[serde(default = "default_title", deserialize_with = "de_title")]
    pub title: String,
    #[serde(default, deserialize_with = "de_null_default")]
    pub hwnd: i64,
    #[serde(default, deserialize_with = "de_null_default")]
    pub process: String,
    #[serde(rename = "PID", default, deserialize_with = "de_null_default")]
    pub pid: u32,
    #[serde(default, deserialize_with = "de_null_default")]
    pub path: String,
    /// 高级模式：标题正则。`None` 表示初级精确规则。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// 是否把无标题窗口纳入正则匹配。
    #[serde(default)]
    pub include_untitled: bool,
    /// 是否把后台窗口纳入正则匹配。
    #[serde(default)]
    pub include_background: bool,
}

impl PartialEq for WindowRule {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.hwnd == other.hwnd
            && self.process == other.process
            && self.pid == other.pid
            && self.path == other.path
            && self.regex == other.regex
            && self.include_untitled == other.include_untitled
            && self.include_background == other.include_background
    }
}

impl Eq for WindowRule {}

impl WindowRule {
    /// 由现有窗口构造一条初级精确规则（用于界面「添加」与旧配置迁移）。
    pub fn from_window(w: &WindowInfo) -> Self {
        Self {
            title: w.title.clone(),
            hwnd: w.hwnd,
            process: w.process.clone(),
            pid: w.pid,
            path: w.path.clone(),
            regex: None,
            include_untitled: false,
            include_background: false,
        }
    }

    /// 由标题正则构造一条高级规则。默认只在「可见且有标题」的窗口中匹配。
    pub fn from_regex(regex: impl Into<String>) -> Self {
        Self {
            title: default_title(),
            hwnd: 0,
            process: String::new(),
            pid: 0,
            path: String::new(),
            regex: Some(regex.into()),
            include_untitled: false,
            include_background: false,
        }
    }

    /// 是否为高级（正则）规则。
    pub fn is_regex(&self) -> bool {
        self.regex.is_some()
    }
}

/// 「进程」规则（粗粒度）：按可执行文件路径隐藏该程序的所有窗口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRule {
    #[serde(default, deserialize_with = "de_null_default")]
    pub process: String,
    #[serde(default, deserialize_with = "de_null_default")]
    pub path: String,
    /// 高级模式：正则（作用于路径或文件名，取决于 `by_name`）。`None` 表示初级精确规则。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// 只按可执行文件名匹配，忽略路径。
    #[serde(default)]
    pub by_name: bool,
    /// 是否把无标题窗口纳入；进程规则默认纳入。
    #[serde(default = "default_bool_true")]
    pub include_untitled: bool,
    /// 是否把后台窗口纳入；默认不纳入。
    #[serde(default)]
    pub include_background: bool,
}

fn default_bool_true() -> bool {
    true
}

impl PartialEq for ProcessRule {
    fn eq(&self, other: &Self) -> bool {
        self.process == other.process
            && self.path == other.path
            && self.regex == other.regex
            && self.by_name == other.by_name
            && self.include_untitled == other.include_untitled
            && self.include_background == other.include_background
    }
}

impl Eq for ProcessRule {}

impl ProcessRule {
    /// 由现有窗口构造一条初级精确进程规则（按其可执行文件路径）。
    pub fn from_window(w: &WindowInfo) -> Self {
        Self {
            process: w.process.clone(),
            path: w.path.clone(),
            regex: None,
            by_name: false,
            include_untitled: true,
            include_background: false,
        }
    }

    /// 由正则构造一条高级规则（默认作用于路径）。
    pub fn from_regex(regex: impl Into<String>) -> Self {
        Self {
            process: String::new(),
            path: String::new(),
            regex: Some(regex.into()),
            by_name: false,
            include_untitled: true,
            include_background: false,
        }
    }

    /// 是否为高级（正则）规则。
    pub fn is_regex(&self) -> bool {
        self.regex.is_some()
    }
}

/// 白名单条目：声明某个程序在哪些模式下应被跳过。
/// 匹配方式与 [`ProcessRule`] 同构，但默认按文件名匹配。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistRule {
    #[serde(default, deserialize_with = "de_null_default")]
    pub process: String,
    #[serde(default, deserialize_with = "de_null_default")]
    pub path: String,
    /// 高级模式：正则（作用于路径或文件名，取决于 `by_name`）。`None` 表示初级精确规则。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// 只按可执行文件名匹配，忽略路径；白名单默认开。
    #[serde(default = "default_bool_true")]
    pub by_name: bool,
    /// 隐藏时跳过该程序的窗口。
    #[serde(default)]
    pub ignore_hide: bool,
    /// 不冻结该程序的进程。
    #[serde(default)]
    pub ignore_freeze: bool,
    /// 不静音该程序的进程。
    #[serde(default)]
    pub ignore_mute: bool,
}

impl PartialEq for WhitelistRule {
    fn eq(&self, other: &Self) -> bool {
        self.process == other.process
            && self.path == other.path
            && self.regex == other.regex
            && self.by_name == other.by_name
            && self.ignore_hide == other.ignore_hide
            && self.ignore_freeze == other.ignore_freeze
            && self.ignore_mute == other.ignore_mute
    }
}

impl Eq for WhitelistRule {}

impl WhitelistRule {
    /// 由现有窗口构造一条精确条目；三个模式开关一律置假。
    pub fn from_window(w: &WindowInfo) -> Self {
        Self {
            process: w.process.clone(),
            path: w.path.clone(),
            regex: None,
            by_name: true,
            ignore_hide: false,
            ignore_freeze: false,
            ignore_mute: false,
        }
    }

    /// 由正则构造一条高级条目（默认作用于文件名）。
    pub fn from_regex(regex: impl Into<String>) -> Self {
        Self {
            process: String::new(),
            path: String::new(),
            regex: Some(regex.into()),
            by_name: true,
            ignore_hide: false,
            ignore_freeze: false,
            ignore_mute: false,
        }
    }

    /// 是否为高级（正则）条目。
    pub fn is_regex(&self) -> bool {
        self.regex.is_some()
    }

    /// 三个模式全关的条目不起任何作用。
    pub fn is_inert(&self) -> bool {
        !self.ignore_hide && !self.ignore_freeze && !self.ignore_mute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_pid_uppercase_key() {
        let json = r#"{"title":"记事本","hwnd":123,"process":"notepad.exe","PID":4567,"path":"C:\\notepad.exe"}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, "记事本");
        assert_eq!(w.hwnd, 123);
        assert_eq!(w.process, "notepad.exe");
        assert_eq!(w.pid, 4567);
        assert_eq!(w.path, "C:\\notepad.exe");
    }

    #[test]
    fn serializes_pid_uppercase_key() {
        let w = WindowInfo::new("t", 1, "p.exe", 9, "C:\\p.exe");
        let json = serde_json::to_string(&w).unwrap();
        assert!(
            json.contains("\"PID\":9"),
            "序列化应输出大写 PID 字段: {json}"
        );
        assert!(!json.contains("\"pid\""), "不应输出小写 pid: {json}");
    }

    #[test]
    fn missing_title_defaults_to_no_title() {
        let json = r#"{"hwnd":1,"process":"p.exe","PID":2,"path":""}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, NO_TITLE);
    }

    #[test]
    fn null_fields_fall_back_to_defaults() {
        let json = r#"{"title":null,"hwnd":null,"process":null,"PID":null,"path":null}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, NO_TITLE);
        assert_eq!(w.hwnd, 0);
        assert_eq!(w.process, "");
        assert_eq!(w.pid, 0);
        assert_eq!(w.path, "");
    }

    #[test]
    fn round_trip_preserves_values() {
        let w = WindowInfo::new("窗口", 42, "app.exe", 100, "D:\\app.exe");
        let json = serde_json::to_string(&w).unwrap();
        let back: WindowInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }

    #[test]
    fn empty_title_string_is_preserved() {
        let json = r#"{"title":"","hwnd":1,"process":"p.exe","PID":2,"path":""}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert_eq!(w.title, "");
    }

    #[test]
    fn visible_defaults_true_when_absent_in_old_config() {
        let json = r#"{"title":"记事本","hwnd":1,"process":"notepad.exe","PID":2,"path":"C:\\notepad.exe"}"#;
        let w: WindowInfo = serde_json::from_str(json).unwrap();
        assert!(w.visible, "旧配置缺少 visible 字段时应默认为 true");
    }

    #[test]
    fn visibility_does_not_affect_window_identity() {
        let shown = WindowInfo::new("微信", 1, "WeChat.exe", 10, "C:\\WeChat.exe");
        let hidden = shown.clone().with_visibility(false);
        assert_eq!(
            shown, hidden,
            "同一窗口被隐藏后（visible=false）仍应等于其可见时的记录"
        );
    }

    #[test]
    fn window_rule_from_window_copies_identity_without_regex() {
        let w = WindowInfo::new("微信", 42, "WeChat.exe", 100, "C:\\WeChat.exe");
        let rule = WindowRule::from_window(&w);
        assert_eq!(rule.title, "微信");
        assert_eq!(rule.hwnd, 42);
        assert_eq!(rule.path, "C:\\WeChat.exe");
        assert!(!rule.is_regex(), "精确规则不应带正则");
    }

    #[test]
    fn window_rule_regex_omitted_from_json_when_none() {
        let rule = WindowRule::from_window(&WindowInfo::new("t", 1, "p.exe", 2, "C:\\p.exe"));
        let json = serde_json::to_string(&rule).unwrap();
        assert!(
            !json.contains("regex"),
            "精确规则不应序列化 regex 字段: {json}"
        );
        assert!(json.contains("\"PID\":2"), "应保留大写 PID: {json}");
    }

    #[test]
    fn window_rule_regex_round_trips() {
        let rule = WindowRule::from_regex("^记事本.*");
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\"regex\":\"^记事本.*\""));
        let back: WindowRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, back);
        assert!(back.is_regex());
    }

    #[test]
    fn process_rule_from_window_uses_path() {
        let w = WindowInfo::new("窗口一", 1, "game.exe", 9, "C:\\game.exe");
        let rule = ProcessRule::from_window(&w);
        assert_eq!(rule.process, "game.exe");
        assert_eq!(rule.path, "C:\\game.exe");
        assert!(!rule.is_regex());
    }

    #[test]
    fn process_rule_regex_round_trips() {
        let rule = ProcessRule::from_regex(r".*\\WeChat\.exe$");
        let json = serde_json::to_string(&rule).unwrap();
        let back: ProcessRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, back);
        assert!(back.is_regex());
    }

    #[test]
    fn whitelist_rule_from_window_defaults_to_name_matching() {
        let w = WindowInfo::new(
            "资源管理器",
            1,
            "explorer.exe",
            9,
            "C:\\Windows\\explorer.exe",
        );
        let rule = WhitelistRule::from_window(&w);
        assert_eq!(rule.process, "explorer.exe");
        assert_eq!(rule.path, "C:\\Windows\\explorer.exe");
        assert!(rule.by_name, "白名单默认按文件名匹配，换安装目录仍然生效");
        assert!(!rule.is_regex());
        assert!(rule.is_inert(), "三个模式默认全关，由界面逐项勾选");
    }

    #[test]
    fn whitelist_rule_modes_are_independent() {
        let json = r#"{"process":"a.exe","ignore_freeze":true}"#;
        let rule: WhitelistRule = serde_json::from_str(json).unwrap();
        assert!(rule.ignore_freeze);
        assert!(!rule.ignore_hide, "三个开关相互独立");
        assert!(!rule.ignore_mute);
        assert!(!rule.is_inert());
    }

    #[test]
    fn whitelist_rule_by_name_defaults_true_when_absent() {
        let rule: WhitelistRule = serde_json::from_str(r#"{"path":"C:\\a.exe"}"#).unwrap();
        assert!(rule.by_name, "手改配置缺该键时按更宽的文件名匹配");
    }

    #[test]
    fn whitelist_rule_round_trips_and_omits_regex_when_none() {
        let mut rule =
            WhitelistRule::from_window(&WindowInfo::new("t", 1, "p.exe", 2, "C:\\p.exe"));
        rule.ignore_hide = true;
        let json = serde_json::to_string(&rule).unwrap();
        assert!(!json.contains("regex"), "精确条目不应序列化 regex: {json}");
        let back: WhitelistRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, back);

        let regex = WhitelistRule::from_regex("(?i)^wechat");
        let back: WhitelistRule =
            serde_json::from_str(&serde_json::to_string(&regex).unwrap()).unwrap();
        assert_eq!(regex, back);
        assert!(back.is_regex());
    }
}
