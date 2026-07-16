//! Verhub 客户端：版本 / 公告 / 反馈 / 日志。
//!
//! 只使用公开端点（`/public/{projectKey}/…`），不需要令牌。字段一律 `#[serde(default)]`。

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Verhub 基础路径。
pub const BASE_URL: &str = "https://verhub.hanloth.cn/api/v1";
pub const PROJECT_KEY: &str = "ivanhanloth-boss-key";
/// 客户端平台（本程序只发行 Windows 版）。
pub const PLATFORM: &str = "windows";

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum VerhubError {
    #[error("网络请求失败: {0}")]
    Http(String),
    #[error("Verhub 返回 {status}: {message}")]
    Api { status: u16, message: String },
    #[error("响应解析失败: {0}")]
    Decode(String),
}

type Result<T> = std::result::Result<T, VerhubError>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadLink {
    pub url: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Version {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub comparable_version: String,
    #[serde(default)]
    pub title: Option<String>,
    /// 更新说明（Markdown）。
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub download_links: Vec<DownloadLink>,
    #[serde(default)]
    pub forced: bool,
    #[serde(default)]
    pub is_latest: bool,
    #[serde(default)]
    pub is_preview: bool,
    #[serde(default)]
    pub is_milestone: bool,
    #[serde(default)]
    pub is_deprecated: bool,
    #[serde(default)]
    pub published_at: i64,
}

impl Version {
    /// 下载地址：优先取匹配 Windows 的链接，其次首个链接，最后回退到 download_url。
    pub fn best_download_url(&self) -> Option<&str> {
        self.download_links
            .iter()
            .find(|l| l.platform.as_deref() == Some(PLATFORM))
            .or_else(|| self.download_links.first())
            .map(|l| l.url.as_str())
            .or(self.download_url.as_deref())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckUpdate {
    #[serde(default)]
    pub should_update: bool,
    /// 强制更新。
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub reason_codes: Vec<String>,
    #[serde(default)]
    pub current_version: Option<String>,
    #[serde(default)]
    pub latest_version: Option<Version>,
    /// 该升到哪个版本（可能是里程碑版本，而非最新版）。
    #[serde(default)]
    pub target_version: Option<Version>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Announcement {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub published_at: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnnouncementList {
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub data: Vec<Announcement>,
}

/// 客户端反馈。`rating` 为 1..=5；`custom_data` 携带附加信息。
#[derive(Debug, Clone, Default, Serialize)]
pub struct Feedback {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<u8>,
    pub content: String,
    pub platform: &'static str,
    pub custom_data: serde_json::Value,
}

/// 日志级别，与 Verhub 的 0..=3 对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
}

const LOG_CONTENT_MAX: usize = 4096;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(concat!("BossKey/", env!("CARGO_PKG_VERSION")))
        // 不把 4xx/5xx 当传输错误，以便自行读取响应体中的报错信息。
        .http_status_as_error(false)
        .build()
        .into()
}

fn url(path: &str) -> String {
    format!("{BASE_URL}/public/{PROJECT_KEY}{path}")
}

fn map_err(e: ureq::Error) -> VerhubError {
    VerhubError::Http(e.to_string())
}

/// 非 2xx 时把响应体读出来作为错误消息。
fn check_status(resp: &mut ureq::http::Response<ureq::Body>) -> Result<()> {
    let status = resp.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(());
    }
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("message").and_then(|m| {
                // message 可能是字符串或字符串数组
                m.as_str().map(str::to_string).or_else(|| {
                    m.as_array().map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join("; ")
                    })
                })
            })
        })
        .filter(|s| !s.is_empty())
        .unwrap_or(body);
    Err(VerhubError::Api { status, message })
}

fn read_json<T: serde::de::DeserializeOwned>(
    mut resp: ureq::http::Response<ureq::Body>,
) -> Result<T> {
    check_status(&mut resp)?;
    resp.body_mut()
        .read_json::<T>()
        .map_err(|e| VerhubError::Decode(e.to_string()))
}

/// 检查更新：把当前版本发给 Verhub，由服务端判断是否需要更新、是否强制。
pub fn check_update(current_version: &str, include_preview: bool) -> Result<CheckUpdate> {
    let resp = agent()
        .post(url("/versions/check-update"))
        .send_json(serde_json::json!({
            "current_version": current_version,
            "current_comparable_version": current_version,
            "include_preview": include_preview,
        }))
        .map_err(map_err)?;
    read_json(resp)
}

/// 公告列表（只要本平台 / 全平台的），从新到旧。
pub fn announcements(limit: u32) -> Result<Vec<Announcement>> {
    let resp = agent()
        .get(url("/announcements"))
        .query("platform", PLATFORM)
        .query("limit", limit.to_string())
        .call()
        .map_err(map_err)?;
    let list: AnnouncementList = read_json(resp)?;
    // 再兜一层，滤掉隐藏公告。
    Ok(list.data.into_iter().filter(|a| !a.is_hidden).collect())
}

pub fn submit_feedback(feedback: &Feedback) -> Result<()> {
    let mut resp = agent()
        .post(url("/feedbacks"))
        .send_json(feedback)
        .map_err(map_err)?;
    check_status(&mut resp)
}

/// 上报一条日志；内容超长会被截断到 Verhub 的上限内。
pub fn upload_log(level: LogLevel, content: &str, device_info: serde_json::Value) -> Result<()> {
    let content = truncate_log(content);
    let mut resp = agent()
        .post(url("/logs"))
        .send_json(serde_json::json!({
            "level": level as u8,
            "content": content,
            "device_info": device_info,
        }))
        .map_err(map_err)?;
    check_status(&mut resp)
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
    fn download_url_prefers_windows_link() {
        let v = Version {
            download_url: Some("https://fallback".into()),
            download_links: vec![
                DownloadLink {
                    url: "https://mac".into(),
                    platform: Some("mac".into()),
                    ..Default::default()
                },
                DownloadLink {
                    url: "https://win".into(),
                    platform: Some("windows".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(v.best_download_url(), Some("https://win"));

        let only_fallback = Version {
            download_url: Some("https://fallback".into()),
            ..Default::default()
        };
        assert_eq!(only_fallback.best_download_url(), Some("https://fallback"));
    }

    #[test]
    fn log_level_maps_to_verhub_numbers() {
        assert_eq!(LogLevel::Debug as u8, 0);
        assert_eq!(LogLevel::Error as u8, 3);
    }
}
