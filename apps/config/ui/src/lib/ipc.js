// Tauri 桥接：真实环境走 invoke / window API；浏览器预览走 mock，方便脱离 Tauri 调试。

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
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

// ---- 统一入口 ----

export async function invoke(cmd, args) {
  if (IN_TAURI) return tauriInvoke(cmd, args);
  return mockInvoke(cmd, args);
}

/** 窗口控制：浏览器预览时静默降级为 no-op。 */
export const win = {
  minimize: () => IN_TAURI && getCurrentWindow().minimize(),
  toggleMaximize: () => IN_TAURI && getCurrentWindow().toggleMaximize(),
  close: () => IN_TAURI && getCurrentWindow().close(),
  isMaximized: async () => (IN_TAURI ? getCurrentWindow().isMaximized() : false),
  startResize: (direction) =>
    IN_TAURI && getCurrentWindow().startResizeDragging(direction),
  onResized: (handler) =>
    IN_TAURI ? getCurrentWindow().onResized(handler) : Promise.resolve(() => {}),
};
