// Verhub（版本 / 公告 / 反馈 / 日志）的前端封装。
//
// 真正的 HTTP 在 Rust 侧（见 crates/common/src/verhub.rs）：配置程序的 CSP 只放行 'self'，
// 前端直连要么被 CSP 拦、要么撞 CORS。这里只负责调命令和整理数据。

import { invoke } from "./ipc.js";

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

/** 上报日志。仅在出错弹框里由用户主动确认后调用——本程序不自动上报。 */
export function uploadLog(content) {
  return invoke("verhub_upload_log", { content });
}

/** 本地日志末尾若干行（出错弹框展示给用户过目，同意后才上报）。 */
export function recentLogTail(lines = 60) {
  return invoke("recent_log_tail", { lines });
}

/** 系统浏览器打开外链（WebView 里的 target=_blank 打不开浏览器）。 */
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

/** 毫秒 / 秒时间戳都能吃：Verhub 发的是秒还是毫秒，取决于后台录入方式。 */
export function formatTime(ts) {
  if (!ts) return "";
  const ms = ts < 1e12 ? ts * 1000 : ts;
  return new Date(ms).toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}
