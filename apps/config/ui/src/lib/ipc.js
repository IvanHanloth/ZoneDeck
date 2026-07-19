// Tauri 桥接：真实环境走 invoke / window API，浏览器预览走 mock。

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const IN_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

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
    freeze_whole_tree: false,
    show_float_window: false,
    mouse: {
      left: { enabled: false, clicks: 1, modifiers: "" },
      middle: { enabled: true, clicks: 1, modifiers: "" },
      right: { enabled: false, clicks: 1, modifiers: "" },
      side1: { enabled: false, clicks: 1, modifiers: "" },
      side2: { enabled: false, clicks: 1, modifiers: "" },
      multi_click_ms: 350,
      allow_click_restore: true,
    },
    auto_hide_enabled: false,
    auto_hide_time: 5,
    top_left_hide: false,
    top_right_hide: false,
    bottom_left_hide: false,
    bottom_right_hide: false,
    allow_move_restore: false,
    corner_fast_only: true,
    log_retention_days: 7,
    autostart_admin: false,
    language: "auto",
  },
  notifications: {
    on_start: true,
    on_quit: true,
    on_autostart: true,
    on_hide: false,
    on_show: false,
  },
  verhub: {
    include_preview: false,
    seen_announcement_id: "",
  },
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

/** mock 下的核心监控状态。 */
let mockMonitoring = true;
/** mock 下的开机自启状态（有状态，供预览联动 UI）。 */
let mockAutostart = false;

function mockInvoke(cmd, args) {
  switch (cmd) {
    case "load_config":
      return structuredClone(mockConfig);
    case "list_windows":
      return structuredClone(mockWindows);
    case "window_icons":
      return {};
    case "autostart_status":
      return { enabled: mockAutostart, method: mockAutostart ? "task" : null };
    case "core_status":
      return { running: true, hidden: false, elevated: false, monitoring: mockMonitoring };
    case "set_hotkeys_enabled":
      mockMonitoring = !!args?.enabled;
      return true;
    case "start_core":
    case "restart_core":
      return true;
    case "pssuspend_available":
      return false;
    case "startup_action":
      return null;
    case "set_autostart":
      mockAutostart = !!args?.enabled;
      return null;
    case "quit_core":
    case "show_windows":
    case "show_all_windows":
    case "open_log_dir":
      return null;
    case "app_info":
      return {
        name: "Boss Key",
        version: "3.0.0",
        website: "https://github.com/IvanHanloth/Boss-Key",
        author: "Ivan Hanloth",
        email: "ivan@hanloth.com",
        blog: "https://blog.ivan-hanloth.cn/",
        license: "MIT",
      };
    case "verhub_check_update":
      return {
        should_update: false,
        required: false,
        reason_codes: [],
        current_version: "3.0.0",
        latest_version: null,
        target_version: null,
      };
    case "verhub_announcements":
      return [
        {
          id: "mock-1",
          title: "Boss Key 3.0 发布",
          content: "全新界面与核心，鼠标按键触发、崩溃恢复、进程冻结。",
          is_pinned: true,
          is_hidden: false,
          author: "Ivan Hanloth",
          published_at: Date.now(),
        },
      ];
    case "verhub_submit_feedback":
    case "verhub_upload_log":
    case "open_external":
      return null;
    case "recent_log_tail":
      return "[mock] 2026-07-14 12:00:00 WARN 这是预览环境的假日志";
    default:
      return null;
  }
}

export async function invoke(cmd, args) {
  if (IN_TAURI) return tauriInvoke(cmd, args);
  return mockInvoke(cmd, args);
}

/** 订阅后端事件，返回取消订阅函数。 */
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
