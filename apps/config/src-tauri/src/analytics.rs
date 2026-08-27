//! 匿名使用统计：记录哪些功能真被用到，用来判断该往哪里投入。
//!
//! 三条硬性约束，改动本模块时必须一并守住：
//! 1. 未获用户明确同意前一个字节都不采、不写盘，由 SDK 的 `require_consent` 保证；
//! 2. 只上报本模块 [`EVENTS`] 列出的事件，属性只留枚举、开关、计数与热键组合；
//! 3. 窗口标题、进程名、文件路径、正则式与反馈正文一律不出现在事件里——
//!    热键组合是唯一的字符串例外，它是产品设计信息，指不回具体的人。

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use verhub_sdk::models::JsonObject;
use verhub_sdk::{AnalyticsOptions, AnalyticsPersistence};

use crate::verhub;

/// 攒批的时间上限。配置界面通常开着几分钟，多数事件在会话中就发出去了，
/// 关窗前的 flush 只需补最后一批。
const FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// 单个属性值的字符上限，超出截断。
const VALUE_MAX: usize = 64;
/// 单条事件的属性个数上限。功能采用快照一条要带三十来项，留点余量。
const PROPS_MAX: usize = 40;

/// 允许上报的事件。界面只能报这张表里的名字，新增采集项必须先在这里登记。
pub const EVENTS: &[&str] = &[
    // 启动时的功能采用快照：哪些功能开着、规则与白名单各几条、热键设成了什么。
    // 「多少人在用某个功能」只能靠它算，变更事件算出来的是改动次数。
    "features",
    // 某一项设置被改动。
    "setting_changed",
    "core_action",
    "restore_tool_action",
    "update_checked",
    "update_download",
    "error_report",
    // 用户同意参与。退出不发任何东西，包括退出这件事本身。
    "analytics_consent",
];

/// 采集参数。状态文件跟着程序数据目录走，便携版不往 `%LOCALAPPDATA%` 留东西。
pub fn options() -> AnalyticsOptions {
    AnalyticsOptions {
        require_consent: true,
        persistence: AnalyticsPersistence::Device,
        flush_interval: FLUSH_INTERVAL,
        state_path: Some(state_path()),
        ..Default::default()
    }
}

fn state_path() -> PathBuf {
    zonedeck_common::paths::data_dir().join("verhub_analytics.json")
}

/// 按配置里的授权状态开闸或关闸。`None`（还没问过）与 `Some(false)` 都不采集。
///
/// SDK 只把「已拒绝」落盘，「已同意」是进程内状态，因此每次启动都要调一次。
pub fn apply_consent(granted: Option<bool>) {
    let Ok(client) = verhub::client() else {
        return;
    };
    match granted {
        Some(true) => {
            // 拒绝过的话退出标记还在本地，只开闸不够；opt_in 清掉标记并换一个
            // 新的匿名标识，不接回拒绝之前那条序列。
            if client.public().has_opted_out() {
                client.public().opt_in();
            }
            client.public().grant_consent();
        }
        // 撤回会顺带清空队列与匿名标识，只留下「已拒绝」这个事实。
        Some(false) => client.public().revoke_consent(),
        None => (),
    }
}

/// 记一次事件；未获授权时 SDK 直接丢弃，调用方不必自己判断。
/// 未登记的事件名一律不发，免得改前端就悄悄多出采集项。
pub async fn track(event: &str, props: Option<Value>) {
    if !EVENTS.contains(&event) {
        return;
    }
    let Ok(client) = verhub::client() else {
        return;
    };
    let _ = client.public().track(event, props.and_then(sanitize)).await;
}

/// 把攒着的事件发出去；Rust 版 SDK 不起后台任务，退出前须调一次。
pub async fn flush() {
    if let Ok(client) = verhub::client() {
        let _ = client.public().flush().await;
    }
}

/// 收紧属性：只留开关、数字与短枚举串，个数与长度都设上限。
/// 这是防止界面误传敏感内容（窗口标题、路径、反馈正文）的最后一道闸。
fn sanitize(props: Value) -> Option<JsonObject> {
    let Value::Object(map) = props else {
        return None;
    };
    let kept: JsonObject = map
        .into_iter()
        .filter_map(|(key, value)| match value {
            Value::Bool(_) | Value::Number(_) => Some((key, value)),
            Value::String(s) => {
                let end = s
                    .char_indices()
                    .nth(VALUE_MAX)
                    .map_or(s.len(), |(idx, _)| idx);
                Some((key, Value::String(s[..end].to_string())))
            }
            _ => None,
        })
        .take(PROPS_MAX)
        .collect();
    (!kept.is_empty()).then_some(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names_are_snake_case_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in EVENTS {
            assert!(seen.insert(name), "事件名重复: {name}");
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "事件名须为小写下划线形式: {name}"
            );
        }
    }

    #[test]
    fn sanitize_keeps_scalars_only() {
        let props = serde_json::json!({
            "enabled": true,
            "count": 3,
            "tab": "power",
            "nested": { "title": "记事本" },
            "list": ["C:\\secret.exe"],
            "absent": null,
        });
        let kept = sanitize(props).expect("标量属性应保留");
        assert_eq!(kept.len(), 3);
        assert_eq!(kept["enabled"], serde_json::json!(true));
        assert_eq!(kept["count"], serde_json::json!(3));
        assert_eq!(kept["tab"], serde_json::json!("power"));
        // 对象、数组、null 一律丢掉，避免夹带整条窗口 / 路径信息。
        assert!(!kept.contains_key("nested"));
        assert!(!kept.contains_key("list"));
        assert!(!kept.contains_key("absent"));
    }

    #[test]
    fn sanitize_truncates_long_strings_on_char_boundary() {
        let props = serde_json::json!({ "who": "记".repeat(200) });
        let kept = sanitize(props).expect("字符串属性应保留");
        let value = kept["who"].as_str().unwrap();
        assert_eq!(value.chars().count(), VALUE_MAX);
    }

    #[test]
    fn sanitize_drops_empty_and_non_object() {
        assert!(sanitize(serde_json::json!({})).is_none());
        assert!(sanitize(serde_json::json!({ "nested": {} })).is_none());
        assert!(sanitize(serde_json::json!("窗口标题")).is_none());
        assert!(sanitize(serde_json::json!(null)).is_none());
    }
}
