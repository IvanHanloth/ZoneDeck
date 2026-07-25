//! Verhub 客户端：版本 / 公告 / 反馈 / 日志，基于官方 verhub-sdk。
//!
//! 只用公开端点（无需凭据）。HTTP 由 SDK 完成；本模块把 SDK 的响应类型映射成
//! 前端 IPC 契约所需的可序列化 DTO，字段名保持不变。

use std::time::Duration;

use serde::Serialize;
use verhub_sdk::VerhubClient;
use verhub_sdk::models::{
    AnnouncementItem, CheckUpdateInput, CreateFeedbackInput, JsonObject, ListAnnouncementsOptions,
    LogLevel, Platform, UploadLogInput, VersionDownloadLink, VersionItem,
};

/// Verhub 基础路径。
pub const BASE_URL: &str = "https://verhub.hanloth.cn/api/v1";
pub const PROJECT_KEY: &str = "ivanhanloth-boss-key";
/// 客户端平台（本程序只发行 Windows 版）。
pub const PLATFORM: Platform = Platform::Windows;

const TIMEOUT: Duration = Duration::from_secs(10);
const LOG_CONTENT_MAX: usize = 4096;

type Result<T> = verhub_sdk::Result<T>;

/// 构造公开接口客户端；User-Agent 追加 `BossKey/{版本}` 以便服务端识别。
fn client() -> Result<VerhubClient> {
    VerhubClient::builder(BASE_URL)
        .project_key(PROJECT_KEY)
        .platform(PLATFORM)
        .timeout(TIMEOUT)
        .app_identifier(concat!("BossKey/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// 把 `serde_json::Value` 收敛为 JSON 对象；非对象一律丢弃。
fn json_object(value: serde_json::Value) -> Option<JsonObject> {
    match value {
        serde_json::Value::Object(map) => Some(map),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DownloadLink {
    pub url: String,
    pub name: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Version {
    pub id: String,
    pub version: String,
    pub comparable_version: String,
    pub title: Option<String>,
    /// 更新说明（Markdown）。
    pub content: Option<String>,
    pub download_url: Option<String>,
    pub download_links: Vec<DownloadLink>,
    pub forced: bool,
    pub is_latest: bool,
    pub is_preview: bool,
    pub is_milestone: bool,
    pub is_deprecated: bool,
    pub published_at: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CheckUpdate {
    pub should_update: bool,
    /// 强制更新。
    pub required: bool,
    pub reason_codes: Vec<String>,
    pub current_version: Option<String>,
    pub latest_version: Option<Version>,
    /// 该升到哪个版本（可能是里程碑版本，而非最新版）。
    pub target_version: Option<Version>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Announcement {
    pub id: String,
    pub title: String,
    pub content: String,
    pub is_pinned: bool,
    pub is_hidden: bool,
    pub author: Option<String>,
    pub published_at: i64,
}

fn map_link(link: VersionDownloadLink) -> DownloadLink {
    DownloadLink {
        url: link.url,
        name: link.name,
        platform: link.platform,
    }
}

fn map_version(version: VersionItem) -> Version {
    Version {
        id: version.id,
        version: version.version,
        comparable_version: version.comparable_version,
        title: version.title,
        content: version.content,
        download_url: version.download_url,
        download_links: version.download_links.into_iter().map(map_link).collect(),
        forced: version.forced,
        is_latest: version.is_latest,
        is_preview: version.is_preview,
        is_milestone: version.is_milestone,
        is_deprecated: version.is_deprecated,
        published_at: version.published_at,
    }
}

fn map_announcement(item: AnnouncementItem) -> Announcement {
    Announcement {
        id: item.id,
        title: item.title,
        content: item.content,
        is_pinned: item.is_pinned,
        is_hidden: item.is_hidden,
        author: item.author,
        published_at: item.published_at,
    }
}

/// 检查更新：把当前版本发给 Verhub，由服务端判断是否需要更新、是否强制。
pub async fn check_update(current_version: &str, include_preview: bool) -> Result<CheckUpdate> {
    let input = CheckUpdateInput {
        current_version: Some(current_version.to_string()),
        current_comparable_version: Some(current_version.to_string()),
        include_preview: Some(include_preview),
    };
    let resp = client()?.public().check_update(&input).await?;
    Ok(CheckUpdate {
        should_update: resp.should_update,
        required: resp.required,
        reason_codes: resp.reason_codes,
        current_version: resp.current_version,
        latest_version: Some(map_version(resp.latest_version)),
        target_version: resp.target_version.map(map_version),
    })
}

/// 公告列表（只要本平台 / 全平台的），从新到旧，并滤掉隐藏公告。
pub async fn announcements(limit: u32) -> Result<Vec<Announcement>> {
    let options = ListAnnouncementsOptions {
        limit: Some(limit),
        platform: Some(PLATFORM),
        ..Default::default()
    };
    let resp = client()?.public().list_announcements(&options).await?;
    Ok(resp
        .data
        .into_iter()
        .filter(|a| !a.is_hidden)
        .map(map_announcement)
        .collect())
}

/// 提交客户端反馈。`rating` 为 1..=5；`custom_data` 携带附加信息。
pub async fn submit_feedback(
    content: String,
    rating: Option<u8>,
    custom_data: serde_json::Value,
) -> Result<()> {
    let input = CreateFeedbackInput {
        content,
        rating,
        platform: Some(PLATFORM),
        custom_data: json_object(custom_data),
        ..Default::default()
    };
    client()?.public().create_feedback(&input).await?;
    Ok(())
}

/// 上报一条错误日志；内容超长会被截断到 Verhub 的上限内。
pub async fn upload_log(content: &str, device_info: serde_json::Value) -> Result<()> {
    let input = UploadLogInput {
        level: LogLevel::Error.into(),
        content: truncate_log(content),
        device_info: json_object(device_info),
        custom_data: None,
    };
    client()?.public().upload_log(&input).await?;
    Ok(())
}

/// 截到上限以内，按字符边界切以避免切碎多字节字符。
fn truncate_log(content: &str) -> String {
    if content.len() <= LOG_CONTENT_MAX {
        return content.to_string();
    }
    const MARK: &str = "…（日志过长，已截断前半部分）\n";
    let budget = LOG_CONTENT_MAX - MARK.len();
    // 保留末尾（出错现场）。
    let start = content.len() - budget;
    let start = (start..content.len())
        .find(|i| content.is_char_boundary(*i))
        .unwrap_or(content.len());
    format!("{MARK}{}", &content[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_tail_within_limit() {
        let long = "错误".repeat(4000); // 远超 4096 字节
        let out = truncate_log(&long);
        assert!(
            out.len() <= LOG_CONTENT_MAX,
            "截断后仍超上限: {}",
            out.len()
        );
        assert!(out.contains("已截断"));
        assert!(out.ends_with('误')); // 保的是末尾（出错现场）
    }

    #[test]
    fn truncate_leaves_short_content_alone() {
        assert_eq!(truncate_log("崩了"), "崩了");
    }

    #[test]
    fn log_level_maps_to_verhub_numbers() {
        assert_eq!(u8::from(LogLevel::Debug), 0);
        assert_eq!(u8::from(LogLevel::Error), 3);
    }
}
