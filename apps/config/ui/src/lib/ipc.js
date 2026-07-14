// Tauri 桥接：真实环境走 invoke / window API；浏览器预览走 mock，方便脱离 Tauri 调试。

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const IN_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// ---- mock（浏览器预览） ----

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
    freeze_after_hide: false,
    enhanced_freeze: false,
    show_float_window: false,
    mouse: {
      left: { enabled: false, clicks: 1, modifiers: "" },
      middle: { enabled: false, clicks: 1, modifiers: "" },
      right: { enabled: false, clicks: 1, modifiers: "" },
      side1: { enabled: false, clicks: 1, modifiers: "" },
      side2: { enabled: false, clicks: 1, modifiers: "" },
      multi_click_ms: 400,
    },
    auto_hide_enabled: false,
    auto_hide_time: 5,
    top_left_hide: false,
    top_right_hide: false,
    bottom_left_hide: false,
    bottom_right_hide: false,
    allow_move_restore: false,
    log_retention_days: 7,
  },
  notifications: {
    on_start: true,
    on_quit: true,
    on_autostart: true,
    on_hide: false,
    on_show: false,
  },
  advanced_mode: false,
  window_rules: [
    {
      title: "微信",
      hwnd: 101,
      process: "WeChat.exe",
      PID: 2001,
      path: "C:\\WeChat.exe",
      include_untitled: false,
      include_background: false,
    },
  ],
  process_rules: [
    {
      process: "TiMi.exe",
      path: "D:\\Games\\TiMi.exe",
      by_name: false,
      include_untitled: true,
      include_background: false,
    },
  ],
};

const mockWindows = [
  { title: "微信", hwnd: 101, process: "WeChat.exe", PID: 2001, path: "C:\\WeChat.exe" },
  { title: "文件传输助手", hwnd: 102, process: "WeChat.exe", PID: 2001, path: "C:\\WeChat.exe" },
  { title: "王者荣耀", hwnd: 201, process: "TiMi.exe", PID: 3002, path: "D:\\Games\\TiMi.exe" },
  { title: "记事本", hwnd: 301, process: "notepad.exe", PID: 4003, path: "C:\\Windows\\notepad.exe" },
];

function mockInvoke(cmd) {
  switch (cmd) {
    case "load_config":
      return structuredClone(mockConfig);
    case "list_windows":
      return structuredClone(mockWindows);
    case "window_icons":
      return {};
    case "autostart_status":
      return false;
    case "core_status":
      return { running: true, hidden: false, elevated: false };
    case "start_core":
    case "restart_core":
      return true;
    case "pssuspend_available":
      return false;
    case "startup_action":
      return null;
    case "quit_core":
    case "show_windows":
    case "show_all_windows":
    case "set_autostart":
    case "set_hotkeys_enabled":
    case "open_log_dir":
      return null;
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

// ---- 统一入口 ----

export async function invoke(cmd, args) {
  if (IN_TAURI) return tauriInvoke(cmd, args);
  return mockInvoke(cmd, args);
}

/**
 * 订阅后端事件（如单实例插件转发的 open-restore）。返回同步的取消订阅函数；
 * 浏览器预览下降级为 no-op。
 */
export function onAppEvent(name, handler) {
  if (!IN_TAURI) return () => {};
  let off = () => {};
  let cancelled = false;
  listen(name, handler).then((fn) => {
    if (cancelled) fn();
    else off = fn;
  });
  return () => {
    cancelled = true;
    off();
  };
}

/** 窗口控制：浏览器预览时静默降级为 no-op。 */
export const win = {
  show: () => IN_TAURI && getCurrentWindow().show(),
  minimize: () => IN_TAURI && getCurrentWindow().minimize(),
  toggleMaximize: () => IN_TAURI && getCurrentWindow().toggleMaximize(),
  close: () => IN_TAURI && getCurrentWindow().close(),
  isMaximized: async () => (IN_TAURI ? getCurrentWindow().isMaximized() : false),
  startResize: (direction) =>
    IN_TAURI && getCurrentWindow().startResizeDragging(direction),
  onResized: (handler) =>
    IN_TAURI ? getCurrentWindow().onResized(handler) : Promise.resolve(() => {}),
};
