// Verhub（版本 / 公告 / 反馈 / 日志）的前端封装；HTTP 实际在 Rust 侧完成。

import { invoke } from "./ipc.js";

/** 项目公开链接（主页 / 仓库 / 文档等）。后端带缓存，可随意调用。 */
export function projectLinks() {
  return invoke("verhub_project_links");
}

/** 检查更新。返回 { should_update, required, target_version, latest_version, … }。 */
export function checkUpdate(includePreview = false) {
  return invoke("verhub_check_update", { includePreview });
}

/** 公告列表（从新到旧，已滤掉隐藏公告）。 */
export function announcements(limit = 20) {
  return invoke("verhub_announcements", { limit });
}

/** 提交反馈。rating 为 1..5 或 null；contact 可空。 */
export function submitFeedback({ content, rating = null, contact = "" }) {
  return invoke("verhub_submit_feedback", { content, rating, contact });
}

/** 上报日志。 */
export function uploadLog(content) {
  return invoke("verhub_upload_log", { content });
}

/** 本地日志末尾若干行。 */
export function recentLogTail(lines = 60) {
  return invoke("recent_log_tail", { lines });
}

/** 用系统浏览器打开外链。 */
export function openExternal(url) {
  return invoke("open_external", { url });
}

/** 版本的下载地址：优先 Windows 链接，其次首个链接，最后回退 download_url。 */
export function downloadUrl(version) {
  if (!version) return "";
  const links = version.download_links || [];
  const win = links.find((l) => l.platform === "windows");
  return (win || links[0])?.url || version.download_url || "";
}

/** 格式化时间戳（秒或毫秒）为本地日期。 */
export function formatTime(ts) {
  if (!ts) return "";
  const ms = ts < 1e12 ? ts * 1000 : ts;
  return new Date(ms).toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}
