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
  hotkey: {
    hide_hotkey: "Ctrl+Q",
    close_hotkey: "Win+Esc",
    hide_only_hotkey: "",
    show_only_hotkey: "",
    hide_foreground_hotkey: "",
    hide_hook: false,
    close_hook: false,
    hide_only_hook: false,
    show_only_hook: false,
    hide_foreground_hook: false,
    hide_intercept: false,
    close_intercept: false,
    hide_only_intercept: false,
    show_only_intercept: false,
    hide_foreground_intercept: false,
  },
  setting: {
    mute_after_hide: true,
    send_before_hide: false,
    resume_media_after_show: false,
    minimize_before_hide: false,
    hide_current: true,
    hide_icon_after_hide: false,
    hide_config_after_hide: true,
    tray_enabled: true,
    tray_clicks: { left: "toggle", double: "settings", right: "menu" },
    tray_badges: { red: "hidden", green: "auto_hide", yellow: "hide_current", blue: "freeze" },
    tray_show_tooltip: true,
    freeze_after_hide: false,
    enhanced_freeze: false,
    power_scope: "self",
    efficiency_after_hide: false,
    efficiency_scope: "self",
    trim_memory_after_freeze: false,
    show_float_window: false,
    mouse: {
      left: { enabled: false, clicks: 1, modifiers: "" },
      middle: { enabled: true, clicks: 2, modifiers: "" },
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
    log_level: "warn",
    autostart_admin: false,
    language: "auto",
  },
  notifications: {
    on_start: true,
    on_quit: true,
    on_autostart: true,
    on_hide: false,
    on_show: false,
    on_recovery_mismatch: true,
  },
  verhub: {
    include_preview: false,
    seen_announcement_id: "",
    analytics: null,
    analytics_consent_sent: false,
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
  whitelist: [
    {
      process: "explorer.exe",
      path: "",
      by_name: true,
      ignore_hide: true,
      ignore_freeze: true,
      ignore_mute: false,
    },
  ],
};

const mockWindows = [
  { title: "微信", hwnd: 101, process: "WeChat.exe", PID: 2001, path: "C:\\WeChat.exe" },
  { title: "文件传输助手", hwnd: 102, process: "WeChat.exe", PID: 2001, path: "C:\\WeChat.exe" },
  { title: "王者荣耀", hwnd: 201, process: "TiMi.exe", PID: 3002, path: "D:\\Games\\TiMi.exe" },
  { title: "记事本", hwnd: 301, process: "notepad.exe", PID: 4003, path: "C:\\Windows\\notepad.exe" },
  { title: "此电脑", hwnd: 401, process: "explorer.exe", PID: 5004, path: "C:\\Windows\\explorer.exe" },
];

/** mock 下的核心监控状态。 */
let mockMonitoring = true;
/** mock 下的自动隐藏开关（随 save_config 更新，供状态轮询回读联动）。 */
let mockAutoHide = mockConfig.setting.auto_hide_enabled;
/** mock 下的开机自启状态（有状态，供预览联动 UI）。 */
let mockAutostart = false;
/** mock 下的能效统计；重置按钮在预览里也能看出效果。 */
let mockPowerStats = {
  schema: 1,
  since: Math.floor(Date.now() / 1000) - 86400 * 30,
  updated_at: Math.floor(Date.now() / 1000),
  freeze_count: 1284,
  efficiency_count: 932,
  // 与次数对得上：每次平均隐藏 45 分钟。
  freeze_seconds: 1284 * 2700,
  efficiency_seconds: 932 * 2700,
  memory_freed_bytes: 1024 ** 3 * 46.2,
};

function mockInvoke(cmd, args) {
  switch (cmd) {
    case "load_config":
      return { config: structuredClone(mockConfig), fallback: null, schema_note: null };
    case "list_windows":
      return structuredClone(mockWindows);
    case "window_icons":
      return {};
    case "autostart_status":
      return { enabled: mockAutostart, method: mockAutostart ? "task" : null };
    case "core_status":
      return {
        running: true,
        hidden: false,
        elevated: false,
        monitoring: mockMonitoring,
        auto_hide_enabled: mockAutoHide,
      };
    case "save_config":
      mockAutoHide = !!args?.config?.setting?.auto_hide_enabled;
      return null;
    case "set_hotkeys_enabled":
      mockMonitoring = !!args?.enabled;
      return true;
    case "start_core":
    case "restart_core":
      return true;
    case "pssuspend_available":
      return false;
    case "power_stats":
      return structuredClone(mockPowerStats);
    case "reset_power_stats":
      mockPowerStats = {
        schema: 1,
        since: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
        freeze_count: 0,
        efficiency_count: 0,
        freeze_seconds: 0,
        efficiency_seconds: 0,
        memory_freed_bytes: 0,
      };
      return null;
    // 预览环境拿不到键盘布局，界面回落显示位置名。
    case "key_labels":
      return {};
    // 预览环境注册不了全局热键，一律报空闲。
    case "hotkey_taken":
      return false;
    case "whitelist_builtins":
      return [{ key: "core", names: ["ZoneDeck.exe", "core.exe"] }];
    // 预览环境没有 regex crate，用 JS 近似；真实判定在后端。
    case "regex_breadth":
      return (args?.patterns ?? []).map((p) => {
        try {
          const re = new RegExp(p);
          return ["", "文档 A", "C:\\Program Files\\a.exe", "窗口", "abc123"].filter((s) =>
            re.test(s),
          ).length * 40;
        } catch {
          return null;
        }
      });
    case "startup_action":
      return null;
    // 浏览器预览没有 DWM，一律走不透明底色。
    case "backdrop_kind":
      return "solid";
    case "set_autostart":
      mockAutostart = !!args?.enabled;
      return null;
    case "quit_core":
    case "show_windows":
    case "show_all_windows":
    case "open_log_dir":
    case "open_program_dir":
      return null;
    case "app_info":
      return {
        name: "ZoneDeck",
        version: "3.0.0",
        website: "https://github.com/IvanHanloth/ZoneDeck",
        author: "Ivan Hanloth",
        email: "ivan@hanloth.com",
        blog: "https://blog.ivan-hanloth.cn/",
        license: "MIT",
      };
    case "verhub_project_links":
      return {
        name: "ZoneDeck",
        website_url: "https://zonedeck.ivan-hanloth.cn/",
        repo_url: "https://github.com/IvanHanloth/ZoneDeck",
        docs_url: "https://zonedeck.ivan-hanloth.cn/guide/",
        author: "Ivan Hanloth",
        author_homepage_url: "https://www.ivan-hanloth.cn/",
        locale: args?.locale ?? null,
        fetched_at: Math.floor(Date.now() / 1000),
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
    case "verhub_announcements": {
      // 按 locale 取译文、缺译文回落默认内容，与服务端一致。
      const texts = {
        "zh-CN": {
          title: "ZoneDeck 3.0 发布",
          content: "全新界面与核心，**鼠标按键触发**、崩溃恢复、进程冻结。详见 [更新日志](https://zonedeck.ivan-hanloth.cn/changelog/)。",
        },
        en: {
          title: "ZoneDeck 3.0 is out",
          content: "A new UI and core: **mouse button triggers**, crash recovery, process freezing. See the [changelog](https://zonedeck.ivan-hanloth.cn/changelog/).",
        },
        "zh-TW": {
          title: "ZoneDeck 3.0 發布",
          content: "全新介面與核心，**滑鼠按鍵觸發**、當機還原、行程凍結。詳見 [更新日誌](https://zonedeck.ivan-hanloth.cn/changelog/)。",
        },
      };
      return [
        {
          id: "mock-1",
          ...(texts[args?.locale] ?? texts["zh-CN"]),
          is_pinned: true,
          is_hidden: false,
          author: "Ivan Hanloth",
          published_at: Date.now(),
        },
      ];
    }
    case "verhub_feedback_options":
      return { github_forward_available: true, contact_required_for_forward: true };
    // 预览环境不联网，埋点一律空转。
    case "analytics_track":
    case "analytics_set_consent":
    case "analytics_flush":
    case "verhub_submit_feedback":
    case "verhub_upload_log":
    case "open_external":
      return null;
    case "current_session_log":
      return [
        "[mock] 2026-07-14 12:00:00 [START] 核心启动 3.1.0（配置 schema v3.0.0.0，日志等级 warn）",
        "[mock] 2026-07-14 12:00:05 [WARN] 这是预览环境的假日志",
      ].join("\n");
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
  minimize: () => IN_TAURI && getCurrentWindow().minimize(),
  toggleMaximize: () => IN_TAURI && getCurrentWindow().toggleMaximize(),
  close: () => IN_TAURI && getCurrentWindow().close(),
  /** 拦截关窗请求；浏览器预览时不拦截。 */
  onCloseRequested: (handler) =>
    IN_TAURI ? getCurrentWindow().onCloseRequested(handler) : Promise.resolve(() => {}),
  isMaximized: async () => (IN_TAURI ? getCurrentWindow().isMaximized() : false),
  startResize: (direction) =>
    IN_TAURI && getCurrentWindow().startResizeDragging(direction),
  onResized: (handler) =>
    IN_TAURI ? getCurrentWindow().onResized(handler) : Promise.resolve(() => {}),
};
