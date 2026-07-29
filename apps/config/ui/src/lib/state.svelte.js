// 全局应用状态（Svelte 5 runes）：配置双向绑定的单一数据源 + 核心状态轮询。

import { invoke } from "./ipc.js";
import { setLangPref, t } from "./i18n.svelte.js";
import { iconPathsToFetch } from "./grouping.js";
import * as verhub from "./verhub.js";

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
  /** 核心状态；running 为 null 表示首次检测尚未返回。monitoring 由核心回报。 */
  status: { running: null, hidden: false, elevated: false, monitoring: true, auto_hide_enabled: false },
  autostart: false,
  /** 当前自启注册方式："task"｜"registry"｜null（未注册）。 */
  autostartMethod: null,
  info: null,
  /** Verhub 上的项目公开链接（主页 / 仓库 / 文档等）；null 时用内置回退链接。 */
  project: null,
  maximized: false,
  saving: false,
  /** 程序目录下是否存在 pssuspend64.exe（增强冻结的前置条件）。 */
  pssuspend: false,
  /** 当前分页；托盘「窗口恢复工具」会把它切到 options。 */
  tab: "binding",
  /** 窗口恢复工具弹窗（可由托盘菜单直接拉起，故提升到全局）。 */
  restoreOpen: false,
  /** 停用请求已发出、核心尚未确认的空档（状态栏据此显示「暂停中…」）。 */
  monitorPending: false,

  /** check-update 的结果；required 为 true 即强制更新。 */
  update: null,
  /** 更新弹窗是否打开。强制更新时它关不掉（见 UpdateModal）。 */
  updateOpen: false,
  updateChecking: false,
  /** 公告列表（从新到旧）。 */
  announcements: [],
  /** 启动时弹出的未读公告；null 表示没有新公告。 */
  pendingAnnouncement: null,
  /** 出错报告 { message, detail }；有值即弹出错误框。 */
  errorReport: null,
  /** 数据目录 { dir, program_dir, kind }；kind 为 portable_fallback 时提示权限问题。 */
  dataLocation: null,
  /** 便携版回退提示弹窗是否打开。 */
  dataNoticeOpen: false,
});

// 按「理由」计数暂停核心监控，最后一个理由撤销后才恢复。
// 监控状态一律以核心回报的 status.monitoring 为准（见 refreshStatus）。

const suspenders = new Set();
let heartbeat = null;
/** 请求序号：先发的慢应答不能覆盖后发的结果。 */
let monitorSeq = 0;

/** 以 reason 为名义请求停用监控；同名重复请求是幂等的。 */
export function suspendMonitoring(reason) {
  const first = suspenders.size === 0;
  suspenders.add(reason);
  if (first) applyMonitoring(false);
}

/** 撤销某个停用理由；所有理由都撤销后才真正恢复监控。 */
export function resumeMonitoring(reason) {
  if (!suspenders.delete(reason) || suspenders.size > 0) return;
  applyMonitoring(true);
}

async function applyMonitoring(enabled) {
  const seq = ++monitorSeq;
  app.monitorPending = true;
  clearInterval(heartbeat);
  heartbeat = null;
  try {
    await invoke("set_hotkeys_enabled", { enabled });
  } catch (err) {
    toast(t("state.suspendFailed", { err }), true);
  }
  if (seq !== monitorSeq) return;
  app.monitorPending = false;
  // 停用期间持续心跳续期（核心侧看门狗认心跳）。
  if (!enabled) {
    heartbeat = setInterval(() => {
      invoke("set_hotkeys_enabled", { enabled: false }).catch(() => {});
    }, SUSPEND_HEARTBEAT_MS);
  }
  refreshStatus();
}

/** 心跳间隔，须显著小于核心 ipc.rs 的 SUSPEND_TIMEOUT_MS(15s)。 */
const SUSPEND_HEARTBEAT_MS = 4000;

/** 打开「窗口恢复工具」并切到对应分页（供托盘直达使用）。 */
export function openRestoreTool() {
  app.tab = "options";
  app.restoreOpen = true;
}

/** 重新检测 pssuspend64.exe；放入文件后无需重启即可启用增强冻结。 */
export async function refreshPssuspend() {
  try {
    app.pssuspend = !!(await invoke("pssuspend_available"));
    toast(t(app.pssuspend ? "state.pssuspendFound" : "state.pssuspendMissing"), !app.pssuspend);
  } catch (err) {
    toast(t("state.detectFailed", { err }), true);
  }
}

/** 切到「关于与反馈」分页（供托盘直达使用）。 */
export function openAboutTab() {
  app.tab = "about";
}

export const toastState = $state({ message: "", error: false, visible: false });
let toastTimer = null;

export function toast(message, error = false) {
  toastState.message = message;
  toastState.error = error;
  toastState.visible = true;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toastState.visible = false), 2600);
}

export async function loadAll() {
  // 各项并行加载；核心状态由轮询单独负责。
  const tasks = [
    invoke("load_config").then((c) => {
      app.config = c;
      // 界面语言先于首帧生效，避免加载后文案跳变。
      setLangPref(c?.setting?.language);
      return refreshWindows();
    }),
    refreshAutostart(),
    invoke("app_info").then((info) => {
      app.info = info;
    }),
    invoke("pssuspend_available").then((v) => (app.pssuspend = !!v)),
    invoke("data_location").then((loc) => {
      app.dataLocation = loc;
      // 回退即程序目录写不进去，须提示用户。
      app.dataNoticeOpen = loc?.kind === "portable_fallback";
    }),
  ];
  // 拉取失败静默：「关于」页有内置回退链接。
  verhub
    .projectLinks()
    .then((p) => (app.project = p))
    .catch(() => {});
  const results = await Promise.allSettled(tasks);
  const failed = results.find((r) => r.status === "rejected");
  if (failed) toast(t("state.partialLoadFailed", { reason: failed.reason }), true);
}

/** 回读开机自启真实状态（是否已注册 + 注册方式）。 */
async function refreshAutostart() {
  const v = await invoke("autostart_status");
  app.autostart = !!v?.enabled;
  app.autostartMethod = v?.method ?? null;
}

export async function refreshWindows() {
  app.available = await invoke("list_windows");
  // 图标异步补充，失败不影响列表。
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

// 自动保存：改动即存，带 debounce。

let saveTimer = null;
/** 有改动排队待存（debounce 期间为 true）；状态回读不得覆盖未保存的改动。 */
let savePending = false;

/** 安排一次自动保存；连续改动只在停顿后写一次盘。 */
export function scheduleSave(delayMs = 600) {
  savePending = true;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    savePending = false;
    saveConfig();
  }, delayMs);
}

async function saveConfig() {
  if (!app.config || app.saving) return;
  app.saving = true;
  try {
    await invoke("save_config", { config: $state.snapshot(app.config) });
  } catch (err) {
    reportError(t("state.saveFailed"), err);
  } finally {
    app.saving = false;
  }
}

export async function startCore(elevated) {
  try {
    const accepted = await invoke("start_core", { elevated });
    if (accepted === false) return toast(t("state.elevationCancelled"), true);
    toast(t(elevated ? "state.coreStartingAdmin" : "state.coreStarting"));
    setTimeout(refreshStatus, 1200);
  } catch (err) {
    reportError(t("state.coreStartFailed"), err);
  }
}

export async function restartCore(elevated) {
  try {
    const accepted = await invoke("restart_core", { elevated });
    if (accepted === false) return toast(t("state.elevationCancelled"), true);
    toast(t(elevated ? "state.coreRestartingAdmin" : "state.coreRestarting"));
    setTimeout(refreshStatus, 1500);
  } catch (err) {
    reportError(t("state.coreRestartFailed"), err);
  }
}

export async function quitCore() {
  try {
    await invoke("quit_core");
    toast(t("state.coreQuitRequested"));
    setTimeout(refreshStatus, 800);
  } catch (err) {
    toast(t("state.coreQuitFailed", { err }), true);
  }
}

export async function setAutostart(enabled, admin) {
  try {
    await invoke("set_autostart", { enabled, admin });
    app.autostart = enabled;
    // 计划任务可能回退到注册表，回读真实注册方式。
    await refreshAutostart();
    toast(t(enabled ? "state.autostartOn" : "state.autostartOff"));
  } catch (err) {
    app.autostart = !enabled; // 失败回滚
    toast(t("state.autostartFailed", { err }), true);
  }
}

/**
 * 检查更新。`manual` 为 true 时无论结果都给出提示；
 * 自动检查仅在有更新时弹窗，失败静默。
 */
export async function checkForUpdate(manual = false) {
  if (app.updateChecking) return;
  app.updateChecking = true;
  try {
    const result = await verhub.checkUpdate(app.config?.verhub?.include_preview ?? false);
    app.update = result;
    if (result.should_update) app.updateOpen = true;
    else if (manual) toast(t("state.upToDate"));
  } catch (err) {
    app.update = null;
    if (manual) toast(t("state.checkUpdateFailed"), true);
  } finally {
    app.updateChecking = false;
  }
}

/** 拉公告；启动时顺带挑出「比已读那条更新」的一条弹给用户。 */
export async function loadAnnouncements({ popNew = false } = {}) {
  try {
    const list = await verhub.announcements(20);
    app.announcements = list;
    if (!popNew) return;
    const seen = app.config?.verhub?.seen_announcement_id ?? "";
    // 置顶公告优先，否则取最新一条；已读过的不再弹出。
    const newest = list.find((a) => a.is_pinned) ?? list[0];
    if (newest && newest.id !== seen) app.pendingAnnouncement = newest;
  } catch {
    /* 公告拉取失败时静默 */
  }
}

/** 记住这条公告已读，下次启动不再弹出。 */
export function markAnnouncementSeen(id) {
  app.pendingAnnouncement = null;
  if (!app.config || !id) return;
  app.config.verhub.seen_announcement_id = id;
  scheduleSave(0);
}

/** 报告一次失败：弹出错误框，由用户决定是否上报日志。 */
export function reportError(message, detail = "") {
  app.errorReport = { message, detail: String(detail) };
}

/** 托盘菜单也能切换自动隐藏；以核心回报为准回读界面，但不覆盖用户尚未保存的改动。 */
function syncAutoHideFromCore() {
  if (!app.config || !app.status.running) return;
  if (savePending || app.saving) return;
  if (app.config.setting.auto_hide_enabled !== app.status.auto_hide_enabled) {
    app.config.setting.auto_hide_enabled = app.status.auto_hide_enabled;
  }
}

/** 刷新核心状态；失败时视为核心离线。 */
export async function refreshStatus() {
  try {
    app.status = await invoke("core_status");
  } catch {
    app.status = { running: false, hidden: false, elevated: false, monitoring: false, auto_hide_enabled: false };
  }
  syncAutoHideFromCore();
  // 核心在停用期间重启过（新实例默认监听），重新按下停用。
  if (suspenders.size > 0 && app.status.running && app.status.monitoring) {
    invoke("set_hotkeys_enabled", { enabled: false }).catch(() => {});
  }
  // 回读开机自启真实状态，与托盘保持一致。
  try {
    await refreshAutostart();
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
