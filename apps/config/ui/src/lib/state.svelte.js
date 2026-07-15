// 全局应用状态（Svelte 5 runes）：配置双向绑定的单一数据源 + 核心状态轮询。

import { invoke } from "./ipc.js";
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
  status: { running: null, hidden: false, elevated: false, monitoring: true },
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
  /**
   * 出错报告：{ message, detail } —— 有值即弹出错误框，用户可选择把日志上报给开发者。
   * 日志**只在这里**上报，程序不会自动往外发（见 verhub_upload_log）。
   */
  errorReport: null,
});

// 两个地方必须让核心先停手：录制热键 / 修饰键时（按下的组合键会直接触发现有热键），
// 以及鼠标在「鼠标按键隐藏」那块设置区里的时候（刚设好左键双击，一双击就把设置窗口藏了）。
// 停用的理由可能同时成立，所以按「理由」计数，最后一个理由撤销了才恢复。
//
// 界面**不自己认定**监控是否停了：状态一律以核心回报的 status.monitoring 为准
// （见 refreshStatus）。核心侧的停用带看门狗，故停用期间要持续心跳续期，
// 配置程序若崩在停用状态，核心超时后自会把热键还给用户。

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
    toast("暂停热键监控失败：" + err, true);
  }
  if (seq !== monitorSeq) return; // 期间又变了主意，交给后来者收尾
  app.monitorPending = false;
  // 停用期间持续续期：核心的看门狗认心跳，不认「我曾经说过一次」。
  if (!enabled) {
    heartbeat = setInterval(() => {
      invoke("set_hotkeys_enabled", { enabled: false }).catch(() => {});
    }, SUSPEND_HEARTBEAT_MS);
  }
  refreshStatus(); // 立刻回读核心真实状态，不等下一次轮询
}

/** 与核心 ipc.rs 的 SUSPEND_TIMEOUT_MS(15s) 配套，必须显著小于它。 */
const SUSPEND_HEARTBEAT_MS = 4000;

/** 打开「窗口恢复工具」并切到对应分页（供托盘直达使用）。 */
export function openRestoreTool() {
  app.tab = "options";
  app.restoreOpen = true;
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
  // 各项并行加载、互不阻塞；核心状态由轮询单独负责。
  const tasks = [
    invoke("load_config").then((c) => {
      app.config = c;
      return refreshWindows();
    }),
    invoke("autostart_status").then((v) => (app.autostart = !!v)),
    invoke("app_info").then((info) => {
      app.info = info;
    }),
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

// 自动保存：去掉手动保存按钮，改动即存，带 debounce。

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
    reportError("保存配置失败，你的改动可能没有写入磁盘。", err);
  } finally {
    app.saving = false;
  }
}

export async function startCore(elevated) {
  try {
    const accepted = await invoke("start_core", { elevated });
    if (accepted === false) return toast("已取消提权", true);
    toast(elevated ? "核心正在以管理员身份启动…" : "核心正在启动…");
    setTimeout(refreshStatus, 1200);
  } catch (err) {
    reportError("核心启动失败，热键与鼠标触发都不会生效。", err);
  }
}

export async function restartCore(elevated) {
  try {
    const accepted = await invoke("restart_core", { elevated });
    if (accepted === false) return toast("已取消提权", true);
    toast(elevated ? "核心正在以管理员身份重启…" : "核心正在重启…");
    setTimeout(refreshStatus, 1500);
  } catch (err) {
    reportError("核心重启失败。", err);
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

// 开机自启：与托盘协同，轮询时回读真实状态。

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

/**
 * 检查更新。`manual` 为 true 时（用户点了「检查更新」）无论有没有新版都给个反馈；
 * 自动检查（启动时）只在真有更新时弹窗，静默失败——网络不通不该在启动时糊用户一脸。
 */
export async function checkForUpdate(manual = false) {
  if (app.updateChecking) return;
  app.updateChecking = true;
  try {
    const result = await verhub.checkUpdate(app.config?.verhub?.include_preview ?? false);
    app.update = result;
    // 强制更新必须弹：这是唯一拦住用户继续用旧版的地方。
    if (result.should_update) app.updateOpen = true;
    else if (manual) toast("已是最新版本");
  } catch (err) {
    app.update = null;
    if (manual) toast("检查更新失败：" + err, true);
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
    // 置顶公告优先；否则取最新一条。已读过的那条（含更早的）不再打扰。
    const newest = list.find((a) => a.is_pinned) ?? list[0];
    if (newest && newest.id !== seen) app.pendingAnnouncement = newest;
  } catch {
    /* 公告拉不到就算了，不打扰用户 */
  }
}

/** 记住这条公告已读——下次启动不再弹它。 */
export function markAnnouncementSeen(id) {
  app.pendingAnnouncement = null;
  if (!app.config || !id) return;
  app.config.verhub.seen_announcement_id = id;
  scheduleSave(0);
}

/**
 * 报告一次失败：弹出错误框，让用户决定要不要把日志上报给开发者。
 * 日志绝不自动上报——用户在框里看得见将要发送的内容，点了「上报」才发。
 */
export function reportError(message, detail = "") {
  app.errorReport = { message, detail: String(detail) };
}

// 核心状态轮询：非阻塞，可随时手动刷新。

export async function refreshStatus() {
  try {
    app.status = await invoke("core_status");
  } catch {
    app.status = { running: false, hidden: false, elevated: false, monitoring: false };
  }
  // 核心在停用期间重启过（新实例默认在监听）——把停用重新按下去，否则用户还停在
  // 录制界面里，鼠标一双击窗口就没了。
  if (suspenders.size > 0 && app.status.running && app.status.monitoring) {
    invoke("set_hotkeys_enabled", { enabled: false }).catch(() => {});
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
