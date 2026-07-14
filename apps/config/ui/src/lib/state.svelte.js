// 全局应用状态（Svelte 5 runes）：配置双向绑定的单一数据源 + 核心状态轮询。

import { invoke } from "./ipc.js";
import { iconPathsToFetch } from "./grouping.js";

export const app = $state({
  /** config.json 的完整内容；表单直接 bind 到它的字段（双向绑定）。 */
  config: null,
  /** 当前所有存活窗口（左侧「现有窗口」选择区数据源）。 */
  available: [],
  /** 进程列表搜索关键字。 */
  search: "",
  /** 现有窗口过滤开关：默认只显示有标题的可见窗口。 */
  showBackground: false,
  showUntitled: false,
  /** exe 路径 → PNG data URI；null 表示查询过但无图标（负缓存）。 */
  icons: {},
  /** 核心状态；running 为 null 表示首次检测尚未返回。 */
  status: { running: null, hidden: false, elevated: false },
  autostart: false,
  info: null,
  maximized: false,
  saving: false,
  /** 程序目录下是否存在 pssuspend64.exe（增强冻结的前置条件）。 */
  pssuspend: false,
  /** 当前分页；托盘「窗口恢复工具」会把它切到 options。 */
  tab: "binding",
  /** 窗口恢复工具弹窗（可由托盘菜单直接拉起，故提升到全局）。 */
  restoreOpen: false,
});

/** 打开「窗口恢复工具」并切到对应分页（供托盘直达使用）。 */
export function openRestoreTool() {
  app.tab = "options";
  app.restoreOpen = true;
}

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
      return refreshWindows();
    }),
    invoke("autostart_status").then((v) => (app.autostart = !!v)),
    invoke("app_info").then((info) => (app.info = info)),
    invoke("pssuspend_available").then((v) => (app.pssuspend = !!v)),
  ];
  const results = await Promise.allSettled(tasks);
  const failed = results.find((r) => r.status === "rejected");
  if (failed) toast("部分数据加载失败：" + failed.reason, true);
}

export async function refreshWindows() {
  app.available = await invoke("list_windows");
  // 图标异步补充，失败不影响列表；窗口 + 规则的路径都取一遍图标。
  const rules = [
    ...(app.config?.window_rules || []),
    ...(app.config?.process_rules || []),
  ];
  loadIcons([...app.available, ...rules]).catch(() => {});
}

async function loadIcons(windows) {
  const paths = iconPathsToFetch(windows, app.icons);
  if (paths.length === 0) return;
  const fetched = await invoke("window_icons", { paths });
  for (const p of paths) {
    app.icons[p] = fetched[p] || null;
  }
}

// ---- 自动保存（去掉手动保存按钮，改动即存，带 debounce） ----

let saveTimer = null;

/** 安排一次自动保存；连续改动只在停顿后写一次盘。 */
export function scheduleSave(delayMs = 600) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveConfig();
  }, delayMs);
}

async function saveConfig() {
  if (!app.config || app.saving) return;
  app.saving = true;
  try {
    // 规则直接绑定在 app.config 上（window_rules / process_rules），整体快照保存即可。
    await invoke("save_config", { config: $state.snapshot(app.config) });
  } catch (err) {
    toast("保存失败：" + err, true);
  } finally {
    app.saving = false;
  }
}

// ---- 核心控制 ----

export async function startCore(elevated) {
  try {
    const accepted = await invoke("start_core", { elevated });
    if (accepted === false) return toast("已取消提权", true);
    toast(elevated ? "核心正在以管理员身份启动…" : "核心正在启动…");
    setTimeout(refreshStatus, 1200);
  } catch (err) {
    toast("启动核心失败：" + err, true);
  }
}

export async function restartCore(elevated) {
  try {
    const accepted = await invoke("restart_core", { elevated });
    if (accepted === false) return toast("已取消提权", true);
    toast(elevated ? "核心正在以管理员身份重启…" : "核心正在重启…");
    setTimeout(refreshStatus, 1500);
  } catch (err) {
    toast("重启核心失败：" + err, true);
  }
}

export async function quitCore() {
  try {
    await invoke("quit_core");
    toast("已请求核心退出");
    setTimeout(refreshStatus, 800);
  } catch (err) {
    toast("退出核心失败：" + err, true);
  }
}

// ---- 开机自启（与托盘协同：轮询时回读真实状态） ----

export async function setAutostart(enabled) {
  try {
    await invoke("set_autostart", { enabled });
    app.autostart = enabled;
    toast(enabled ? "已开启开机自启" : "已关闭开机自启");
  } catch (err) {
    app.autostart = !enabled; // 失败回滚
    toast("设置开机自启失败：" + err, true);
  }
}

// ---- 核心状态轮询（非阻塞、可随时手动刷新） ----

export async function refreshStatus() {
  try {
    app.status = await invoke("core_status");
  } catch {
    app.status = { running: false, hidden: false, elevated: false };
  }
  // 顺带回读开机自启的真实状态：用户可能在托盘菜单里改过，这里保持两端一致。
  try {
    app.autostart = !!(await invoke("autostart_status"));
  } catch {
    /* 忽略 */
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
