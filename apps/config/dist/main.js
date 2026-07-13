"use strict";

// ---- Tauri 桥接：真实环境用 __TAURI__，否则回退到 mock 数据（便于浏览器预览）----
const IN_TAURI = !!(window.__TAURI__ && window.__TAURI__.core);

const mockConfig = {
  version: "v3.0.0.0",
  history: [],
  frozen_pids: [],
  hotkey: { hide_hotkey: "Ctrl+Q", close_hotkey: "Win+Esc" },
  setting: {
    mute_after_hide: true,
    send_before_hide: false,
    hide_current: true,
    click_to_hide: true,
    hide_icon_after_hide: false,
    path_match: false,
    freeze_after_hide: false,
    enhanced_freeze: false,
    show_float_window: false,
    middle_button_hide: false,
    side_button1_hide: false,
    side_button2_hide: false,
    auto_hide_enabled: false,
    auto_hide_time: 5,
    top_left_hide: false,
    top_right_hide: false,
    bottom_left_hide: false,
    bottom_right_hide: false,
    allow_move_restore: false,
  },
  hide_binding: [],
};

const mockWindows = [
  { title: "微信", hwnd: 101, process: "WeChat.exe", PID: 2001, path: "C:\\WeChat.exe" },
  { title: "文件传输助手", hwnd: 102, process: "WeChat.exe", PID: 2001, path: "C:\\WeChat.exe" },
  { title: "王者荣耀", hwnd: 201, process: "TiMi.exe", PID: 3002, path: "D:\\Games\\TiMi.exe" },
  { title: "记事本", hwnd: 301, process: "notepad.exe", PID: 4003, path: "C:\\Windows\\notepad.exe" },
];

async function invoke(cmd, args) {
  if (IN_TAURI) return window.__TAURI__.core.invoke(cmd, args);
  // mock 实现
  switch (cmd) {
    case "load_config":
      return JSON.parse(JSON.stringify(mockConfig));
    case "list_windows":
      return JSON.parse(JSON.stringify(mockWindows));
    case "autostart_status":
      return false;
    case "window_icons":
      return {};
    case "core_status":
      return { running: false, hidden: false, elevated: false };
    case "restart_core_elevated":
      return true;
    case "app_info":
      return {
        name: "Boss Key",
        version: "v3.0.0.0",
        website: "https://github.com/IvanHanloth/Boss-Key",
        update_feed: "https://ivanhanloth.github.io/Boss-Key/releases.json",
      };
    default:
      return null;
  }
}

// ---- 全局状态 ----
const state = {
  config: null,
  bound: [], // WindowInfo[]
  available: [], // WindowInfo[]
  icons: {}, // exe 路径 → PNG data URI（null 表示查询过但无图标）
};

const $ = (id) => document.getElementById(id);

const SETTING_KEYS = [
  "mute_after_hide",
  "hide_current",
  "click_to_hide",
  "hide_icon_after_hide",
  "path_match",
  "send_before_hide",
  "show_float_window",
  "freeze_after_hide",
  "enhanced_freeze",
  "middle_button_hide",
  "side_button1_hide",
  "side_button2_hide",
  "top_left_hide",
  "top_right_hide",
  "bottom_left_hide",
  "bottom_right_hide",
  "allow_move_restore",
  "auto_hide_enabled",
];

// ---- 初始化 ----
async function init() {
  setupTabs();
  setupButtons();
  setupRecorders();

  state.config = await invoke("load_config");
  applyConfigToUi(state.config);
  state.bound = (state.config.hide_binding || []).slice();
  await refreshWindows();

  const autostart = await invoke("autostart_status");
  $("autostart-toggle").checked = !!autostart;

  const info = await invoke("app_info");
  $("about-name").textContent = info.name;
  $("about-version").textContent = "版本 " + info.version;
  $("about-website").href = info.website;

  await refreshCoreStatus();
}

async function refreshCoreStatus() {
  const status = await invoke("core_status");
  const el = $("core-status");
  el.textContent = status.running ? "核心运行中" : "核心未运行";
  el.classList.toggle("online", status.running);
  el.classList.toggle("offline", !status.running);

  const elev = $("core-elevation");
  if (!status.running) {
    elev.textContent = "核心未运行";
  } else {
    elev.textContent = status.elevated ? "管理员" : "普通用户";
  }
  $("btn-elevate").disabled = status.running && status.elevated;
}

function applyConfigToUi(config) {
  $("hide-hotkey").value = config.hotkey.hide_hotkey;
  $("close-hotkey").value = config.hotkey.close_hotkey;
  for (const key of SETTING_KEYS) {
    const el = $(key);
    if (el) el.checked = !!config.setting[key];
  }
  $("auto_hide_time").value = config.setting.auto_hide_time || 5;
}

function collectConfig() {
  const c = state.config;
  c.hotkey.hide_hotkey = $("hide-hotkey").value.trim() || "Ctrl+Q";
  c.hotkey.close_hotkey = $("close-hotkey").value.trim() || "Win+Esc";
  for (const key of SETTING_KEYS) {
    const el = $(key);
    if (el) c.setting[key] = el.checked;
  }
  c.setting.auto_hide_time = Math.max(1, parseInt($("auto_hide_time").value, 10) || 5);
  c.hide_binding = state.bound;
  return c;
}

// ---- 标签页 ----
function setupTabs() {
  document.querySelectorAll(".tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
      document.querySelectorAll(".panel").forEach((p) => p.classList.remove("active"));
      tab.classList.add("active");
      $("tab-" + tab.dataset.tab).classList.add("active");
    });
  });
}

// ---- 窗口绑定 ----
function sameWindow(a, b) {
  return a.hwnd === b.hwnd && a.process === b.process;
}

async function refreshWindows() {
  const all = await invoke("list_windows");
  state.available = all.filter((w) => !state.bound.some((b) => sameWindow(b, w)));
  renderLists();
  // 图标异步加载，取到后重绘；失败不影响列表本身。
  loadIcons([...state.available, ...state.bound]).catch(() => {});
}

async function loadIcons(windows) {
  const paths = [...new Set(windows.map((w) => w.path))].filter(
    (p) => p && !(p in state.icons)
  );
  if (paths.length === 0) return;
  const fetched = await invoke("window_icons", { paths });
  for (const p of paths) {
    state.icons[p] = fetched[p] || null; // 记住“无图标”，避免重复请求
  }
  renderLists();
}

function renderLists() {
  renderGrouped($("available-list"), state.available, "available");
  renderGrouped($("bound-list"), state.bound, "bound");
}

function renderGrouped(container, windows, kind) {
  container.innerHTML = "";
  const groups = {};
  for (const w of windows) {
    (groups[w.process] = groups[w.process] || []).push(w);
  }
  const names = Object.keys(groups).sort();
  if (names.length === 0) {
    container.innerHTML = '<p class="hint">（空）</p>';
    return;
  }
  for (const proc of names) {
    const group = document.createElement("div");
    group.className = "proc-group";
    const head = document.createElement("div");
    head.className = "proc-name";
    const iconUri = state.icons[groups[proc][0].path];
    if (iconUri) {
      const icon = document.createElement("img");
      icon.className = "proc-icon";
      icon.src = iconUri;
      icon.alt = "";
      head.appendChild(icon);
    }
    const procName = document.createElement("span");
    procName.textContent = proc;
    head.appendChild(procName);
    group.appendChild(head);
    for (const w of groups[proc]) {
      const item = document.createElement("label");
      item.className = "win-item";
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.dataset.hwnd = w.hwnd;
      cb.dataset.kind = kind;
      const title = document.createElement("span");
      title.className = "title";
      title.textContent = w.title;
      const meta = document.createElement("span");
      meta.className = "meta";
      meta.textContent = "PID " + w.PID;
      item.append(cb, title, meta);
      group.appendChild(item);
    }
    container.appendChild(group);
  }
}

function checkedWindows(kind) {
  const list = kind === "available" ? state.available : state.bound;
  const checked = [];
  document
    .querySelectorAll('input[data-kind="' + kind + '"]:checked')
    .forEach((cb) => {
      const w = list.find((x) => String(x.hwnd) === cb.dataset.hwnd);
      if (w) checked.push(w);
    });
  return checked;
}

function setupButtons() {
  $("btn-add").addEventListener("click", () => {
    const picked = checkedWindows("available");
    state.bound.push(...picked);
    state.available = state.available.filter((w) => !picked.some((p) => sameWindow(p, w)));
    renderLists();
  });

  $("btn-remove").addEventListener("click", () => {
    const picked = checkedWindows("bound");
    state.available.push(...picked);
    state.bound = state.bound.filter((w) => !picked.some((p) => sameWindow(p, w)));
    renderLists();
  });

  $("btn-refresh").addEventListener("click", refreshWindows);

  $("btn-save").addEventListener("click", onSave);

  $("autostart-toggle").addEventListener("change", async (e) => {
    try {
      await invoke("set_autostart", { enabled: e.target.checked });
      toast(e.target.checked ? "已开启开机自启" : "已关闭开机自启");
    } catch (err) {
      e.target.checked = !e.target.checked;
      toast("设置开机自启失败：" + err, true);
    }
  });

  $("btn-restore").addEventListener("click", async () => {
    try {
      await invoke("show_all_windows");
      toast("已请求显示所有隐藏窗口");
    } catch (err) {
      toast("操作失败：" + err, true);
    }
  });

  $("btn-check-update").addEventListener("click", checkUpdate);

  $("btn-elevate").addEventListener("click", async () => {
    try {
      const accepted = await invoke("restart_core_elevated");
      if (accepted) {
        toast("核心正在以管理员身份重启…");
        setTimeout(refreshCoreStatus, 1500);
      } else {
        toast("已取消提权", true);
      }
    } catch (err) {
      toast("提权重启失败：" + err, true);
    }
  });
}

async function onSave() {
  const config = collectConfig();
  try {
    await invoke("save_config", { config });
    toast("设置已保存并应用");
  } catch (err) {
    toast("保存失败：" + err, true);
  }
}

// ---- 热键录制 ----
const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

function keyName(e) {
  const k = e.key;
  if (k.length === 1) {
    const up = k.toUpperCase();
    if (/[A-Z0-9]/.test(up)) return up;
  }
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(k)) return k;
  const map = {
    Escape: "Esc",
    " ": "Space",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  };
  return map[k] || null;
}

function setupRecorders() {
  document.querySelectorAll(".record-btn").forEach((btn) => {
    btn.addEventListener("click", () => startRecording(btn));
  });
}

function startRecording(btn) {
  const input = $(btn.dataset.target);
  btn.classList.add("recording");
  btn.textContent = "按下组合键…";

  const handler = (e) => {
    e.preventDefault();
    if (MODIFIER_KEYS.has(e.key)) return;
    const main = keyName(e);
    if (!main) return;

    const parts = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Win");
    parts.push(main);

    input.value = parts.join("+");
    stop();
  };

  const stop = () => {
    window.removeEventListener("keydown", handler, true);
    btn.classList.remove("recording");
    btn.textContent = "录制";
  };

  window.addEventListener("keydown", handler, true);
  // 10 秒后自动取消
  setTimeout(stop, 10000);
}

// ---- 检查更新（前端直接拉取 GitHub Pages 源）----
async function checkUpdate() {
  const result = $("update-result");
  result.textContent = "检查中…";
  result.classList.remove("error");
  try {
    const info = await invoke("app_info");
    const resp = await fetch(info.update_feed, { cache: "no-store" });
    if (!resp.ok) throw new Error("HTTP " + resp.status);
    const releases = await resp.json();
    if (!Array.isArray(releases) || releases.length === 0) {
      throw new Error("未获取到版本信息");
    }
    releases.sort((a, b) => new Date(b.published_at) - new Date(a.published_at));
    const latest = releases[0];
    const name = latest.name || latest.tag_name || "最新版本";
    const url = latest.html_url || info.website;
    result.innerHTML = '最新版本：<a href="' + url + '" target="_blank" rel="noreferrer">' + name + "</a>";
  } catch (err) {
    result.textContent = "检查更新失败：" + err;
    result.classList.add("error");
  }
}

// ---- 提示条 ----
let toastTimer = null;
function toast(msg, isError) {
  const el = $("toast");
  el.textContent = msg;
  el.classList.toggle("error", !!isError);
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove("show"), 2600);
}

init().catch((e) => {
  document.body.innerHTML = '<p style="padding:20px">初始化失败：' + e + "</p>";
});
