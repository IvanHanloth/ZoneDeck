// 全局应用状态（Svelte 5 runes）：配置双向绑定的单一数据源 + 核心状态轮询。

import { invoke } from "./ipc.js";
import { availableWindows, iconPathsToFetch } from "./grouping.js";

export const app = $state({
  /** config.json 的完整内容；表单直接 bind 到它的字段（双向绑定）。 */
  config: null,
  /** 已绑定 / 可绑定窗口列表。 */
  bound: [],
  available: [],
  /** exe 路径 → PNG data URI；null 表示查询过但无图标（负缓存）。 */
  icons: {},
  /** 核心状态；running 为 null 表示首次检测尚未返回。 */
  status: { running: null, hidden: false, elevated: false },
  autostart: false,
  info: null,
  maximized: false,
  saving: false,
});

// ---- 提示条 ----

export const toastState = $state({ message: "", error: false, visible: false });
let toastTimer = null;

export function toast(message, error = false) {
  toastState.message = message;
  toastState.error = error;
  toastState.visible = true;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toastState.visible = false), 2600);
}

// ---- 数据加载 ----

export async function loadAll() {
  // 各项并行加载、互不阻塞；核心状态由轮询单独负责。
  const tasks = [
    invoke("load_config").then((c) => {
      app.config = c;
      app.bound = (c.hide_binding || []).slice();
      return refreshWindows();
    }),
    invoke("autostart_status").then((v) => (app.autostart = !!v)),
    invoke("app_info").then((info) => (app.info = info)),
  ];
  const results = await Promise.allSettled(tasks);
  const failed = results.find((r) => r.status === "rejected");
  if (failed) toast("部分数据加载失败：" + failed.reason, true);
}

export async function refreshWindows() {
  const all = await invoke("list_windows");
  app.available = availableWindows(all, app.bound);
  // 图标异步补充，失败不影响列表。
  loadIcons([...app.available, ...app.bound]).catch(() => {});
}

async function loadIcons(windows) {
  const paths = iconPathsToFetch(windows, app.icons);
  if (paths.length === 0) return;
  const fetched = await invoke("window_icons", { paths });
  for (const p of paths) {
    app.icons[p] = fetched[p] || null;
  }
}

// ---- 保存 ----

export async function saveConfig() {
  if (!app.config || app.saving) return;
  app.saving = true;
  try {
    app.config.hide_binding = app.bound;
    await invoke("save_config", { config: $state.snapshot(app.config) });
    toast("设置已保存并应用");
  } catch (err) {
    toast("保存失败：" + err, true);
  } finally {
    app.saving = false;
  }
}

// ---- 核心状态轮询（非阻塞、可随时手动刷新） ----

export async function refreshStatus() {
  try {
    app.status = await invoke("core_status");
  } catch {
    app.status = { running: false, hidden: false, elevated: false };
  }
}

/** 启动轮询；返回停止函数。页面不可见时暂停，恢复可见立即刷新。 */
export function startStatusPolling(intervalMs = 2000) {
  refreshStatus();
  const timer = setInterval(() => {
    if (document.visibilityState === "visible") refreshStatus();
  }, intervalMs);
  const onVisible = () => {
    if (document.visibilityState === "visible") refreshStatus();
  };
  document.addEventListener("visibilitychange", onVisible);
  return () => {
    clearInterval(timer);
    document.removeEventListener("visibilitychange", onVisible);
  };
}
