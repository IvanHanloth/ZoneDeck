// 全局应用状态：配置双向绑定的单一数据源 + 核心状态轮询。

import { invoke } from "./ipc.js";
import { setLangPref, t } from "./i18n.svelte.js";
import { iconPathsToFetch } from "./grouping.js";
import { sanitizeConfig } from "./sanitize.js";
import { createAutosave } from "./autosave.js";
import { createBreadthGuard } from "./regexcheck.js";
import * as verhub from "./verhub.js";

export const app = $state({
  /** config.json 的完整内容；表单直接 bind 到它的字段。 */
  config: null,
  /** 当前所有存活窗口。 */
  available: [],
  /** 进程列表搜索关键字。 */
  search: "",
  /** 现有窗口过滤开关：默认只显示有标题的可见窗口。 */
  showBackground: false,
  showUntitled: false,
  /** exe 路径 → PNG data URI；null 表示查询过但无图标。 */
  icons: {},
  /** 核心状态；running 为 null 表示首次检测尚未返回。 */
  status: { running: null, hidden: false, elevated: false, monitoring: true, auto_hide_enabled: false },
  autostart: false,
  /** 当前自启注册方式："task"｜"registry"｜null。 */
  autostartMethod: null,
  info: null,
  /** Verhub 上的项目公开链接；null 时用内置回退链接。 */
  project: null,
  maximized: false,
  saving: false,
  /** 程序目录下是否存在 pssuspend64.exe。 */
  pssuspend: false,
  /** 当前分页。 */
  tab: "binding",
  /** 窗口恢复工具弹窗，可由托盘菜单直接拉起。 */
  restoreOpen: false,
  /** 停用请求已发出、核心尚未确认的空档。 */
  monitorPending: false,

  /** check-update 的结果；required 为 true 即强制更新。 */
  update: null,
  /** 更新弹窗是否打开；强制更新时关不掉。 */
  updateOpen: false,
  updateChecking: false,
  /** 公告列表（从新到旧）。 */
  announcements: [],
  /** 启动时弹出的未读公告；null 表示没有新公告。 */
  pendingAnnouncement: null,
  /** 出错报告 { message, detail }；有值即弹出错误框。 */
  errorReport: null,
  /** 数据目录 { dir, program_dir, kind }。 */
  dataLocation: null,
  /** 便携版回退提示弹窗是否打开。 */
  dataNoticeOpen: false,
  /** 白名单里不可删除的内置项 [{ key, names }]。 */
  whitelistBuiltins: [],
  /** 待提示的过宽正则 [{ kind, pattern, hits }]；有值即弹窗。 */
  broadRegex: null,
  /** 判定为过宽且用户尚未确认的正则，界面据此标红。 */
  broadPatterns: new Set(),
});

// 按「理由」计数暂停核心监控，最后一个理由撤销后才恢复。

const suspenders = new Set();
let heartbeat = null;
/** 请求序号，防止慢应答覆盖后发的结果。 */
let monitorSeq = 0;

/** 最近一次停用请求的应答；探测热键占用要等它落地。 */
let suspended = Promise.resolve();

/**
 * 以 reason 为名义请求停用监控；同名重复请求是幂等的。
 * 返回的 promise 在核心确实撤掉热键后落定。
 */
export function suspendMonitoring(reason) {
  const first = suspenders.size === 0;
  suspenders.add(reason);
  if (first) suspended = applyMonitoring(false);
  return suspended;
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
  // 停用期间持续心跳续期。
  if (!enabled) {
    heartbeat = setInterval(() => {
      invoke("set_hotkeys_enabled", { enabled: false }).catch(() => {});
    }, SUSPEND_HEARTBEAT_MS);
  }
  refreshStatus();
}

/** 心跳间隔，须显著小于核心的 SUSPEND_TIMEOUT_MS。 */
const SUSPEND_HEARTBEAT_MS = 4000;

/** 打开「窗口恢复工具」并切到对应分页。 */
export function openRestoreTool() {
  app.tab = "options";
  app.restoreOpen = true;
}

/** 重新检测 pssuspend64.exe。 */
export async function refreshPssuspend() {
  try {
    app.pssuspend = !!(await invoke("pssuspend_available"));
    toast(t(app.pssuspend ? "state.pssuspendFound" : "state.pssuspendMissing"), !app.pssuspend);
  } catch (err) {
    toast(t("state.detectFailed", { err }), true);
  }
}

/** 打开程序目录，即 pssuspend64.exe 该放的位置。 */
export async function openProgramDir() {
  try {
    await invoke("open_program_dir");
  } catch (err) {
    toast(t("state.openDirFailed", { err }), true);
  }
}

/** 切到「关于与反馈」分页。 */
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
    invoke("load_config").then((loaded) => {
      app.config = loaded.config;
      // 界面语言先于首帧生效。
      setLangPref(loaded.config?.setting?.language);
      // 配置损坏已回退默认值，原因与备份去向必须让用户看到。
      if (loaded.fallback) reportError(t("state.configFallback"), loaded.fallback);
      // 配置来自更高版本：本次照常生效，但保存后新版设置项会丢。
      if (loaded.schema_note) reportError(t("state.configSchemaNewer"), loaded.schema_note);
      // 语言定下来之后再拉，项目信息才会是当前语言的译文。
      loadProjectLinks();
      return refreshWindows();
    }),
    refreshAutostart(),
    invoke("app_info").then((info) => {
      app.info = info;
    }),
    invoke("pssuspend_available").then((v) => (app.pssuspend = !!v)),
    invoke("whitelist_builtins").then((v) => {
      app.whitelistBuiltins = v ?? [];
    }),
    invoke("data_location").then((loc) => {
      app.dataLocation = loc;
      // 回退即程序目录写不进去。
      app.dataNoticeOpen = loc?.kind === "portable_fallback";
    }),
  ];
  const results = await Promise.allSettled(tasks);
  const failed = results.find((r) => r.status === "rejected");
  if (failed) toast(t("state.partialLoadFailed", { reason: failed.reason }), true);
}

/** 项目公开链接；拉取失败静默，「关于」页有内置回退链接。 */
function loadProjectLinks() {
  verhub
    .projectLinks()
    .then((p) => (app.project = p))
    .catch(() => {});
}

/** 回读开机自启真实状态。 */
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

// 自动保存：改动即存，带 debounce；关窗前由 flushSave 兜底。

/** 正则过宽检查器；判定在后端。 */
const breadthGuard = createBreadthGuard((patterns) =>
  invoke("regex_breadth", { patterns }),
);

/** 弹窗关闭后要 resolve 的回调。 */
let broadRegexResolve = null;

function closeBroadRegex() {
  app.broadRegex = null;
  broadRegexResolve?.();
  broadRegexResolve = null;
}

/** 「仍然保存」：此后不再提醒、不再标红。 */
export function acknowledgeBroadRegex() {
  const patterns = (app.broadRegex ?? []).map((i) => i.pattern);
  breadthGuard.acknowledge(patterns);
  app.broadPatterns = new Set([...app.broadPatterns].filter((p) => !patterns.includes(p)));
  closeBroadRegex();
}

/** 「我知道了」：不再打断保存，但保留标红。 */
export function dismissBroadRegex() {
  breadthGuard.dismiss((app.broadRegex ?? []).map((i) => i.pattern));
  closeBroadRegex();
}

/** 写盘前的过宽正则检查；不拦保存，只决定此后提不提醒、标不标红。 */
async function warnOnBroadRegex(config) {
  const { broad, toWarn } = await breadthGuard.inspect(config);
  const patterns = new Set(broad.map((i) => i.pattern));
  // 用户改好、删掉或确认无误的正则，红框跟着消失。
  if (patterns.size > 0 || app.broadPatterns.size > 0) app.broadPatterns = patterns;
  if (toWarn.length === 0) return;
  app.broadRegex = toWarn;
  await new Promise((resolve) => {
    broadRegexResolve = resolve;
  });
}

const autosave = createAutosave(async () => {
  if (!app.config) return true;
  const config = sanitizeConfig($state.snapshot(app.config));
  await warnOnBroadRegex(config);
  app.saving = true;
  try {
    await invoke("save_config", { config });
    return true;
  } catch (err) {
    reportError(t("state.saveFailed"), err);
    return false;
  } finally {
    app.saving = false;
  }
});

/** 安排一次自动保存；连续改动只在停顿后写一次盘。 */
export function scheduleSave(delayMs) {
  autosave.schedule(delayMs);
}

/** 立即写盘全部未落盘的改动；返回是否成功。关窗前调用。 */
export function flushSave() {
  return autosave.flush();
}

/** 是否还有未落盘的改动；状态回读不得覆盖它们。 */
export function hasUnsavedChanges() {
  return autosave.dirty;
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
    // 计划任务可能回退到注册表。
    await refreshAutostart();
    toast(t(enabled ? "state.autostartOn" : "state.autostartOff"));
  } catch (err) {
    app.autostart = !enabled; // 失败回滚
    toast(t("state.autostartFailed", { err }), true);
  }
}

/** 检查更新；`manual` 为 true 时无论结果都给出提示，自动检查失败静默。 */
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

/** 拉公告；启动时挑出比已读那条更新的一条弹给用户。 */
export async function loadAnnouncements({ popNew = false } = {}) {
  try {
    const list = await verhub.announcements(20);
    app.announcements = list;
    if (!popNew) return;
    const seen = app.config?.verhub?.seen_announcement_id ?? "";
    // 置顶公告优先，否则取最新一条。
    const newest = list.find((a) => a.is_pinned) ?? list[0];
    if (newest && newest.id !== seen) app.pendingAnnouncement = newest;
  } catch {
    /* 公告拉取失败时静默 */
  }
}

/** 记住这条公告已读。 */
export function markAnnouncementSeen(id) {
  app.pendingAnnouncement = null;
  if (!app.config || !id) return;
  app.config.verhub.seen_announcement_id = id;
  scheduleSave(0);
}

/** 界面语言切换后按新语言重取服务端下发的内容：项目信息、公告与更新说明。 */
export async function refreshLocalizedContent() {
  loadProjectLinks();
  loadAnnouncements();
  // 只重取已有的结果，免得换个语言就弹一次更新提示。
  if (!app.update || app.updateChecking) return;
  app.updateChecking = true;
  try {
    app.update = await verhub.checkUpdate(app.config?.verhub?.include_preview ?? false);
  } catch {
    /* 拉取失败时保留原语言的内容 */
  } finally {
    app.updateChecking = false;
  }
}

/** 报告一次失败：弹出错误框，由用户决定是否上报日志。 */
export function reportError(message, detail = "") {
  app.errorReport = { message, detail: String(detail) };
}

/** 以核心回报回读自动隐藏开关，但不覆盖用户尚未保存的改动。 */
function syncAutoHideFromCore() {
  if (!app.config || !app.status.running) return;
  if (autosave.dirty) return;
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
  // 核心在停用期间重启过，重新按下停用。
  if (suspenders.size > 0 && app.status.running && app.status.monitoring) {
    invoke("set_hotkeys_enabled", { enabled: false }).catch(() => {});
  }
  // 回读开机自启真实状态。
  try {
    await refreshAutostart();
  } catch {
    /* 忽略 */
  }
}

/** 启动轮询，返回停止函数；页面不可见时暂停。 */
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
