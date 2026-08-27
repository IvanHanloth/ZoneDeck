import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { CONFIG_EVENTS, WATCHED, featureProps, trackConfigChanges } from "./analytics.js";

// 拦下 IPC，好看清到底发了哪些事件。
const { calls } = vi.hoisted(() => ({ calls: [] }));
vi.mock("./ipc.js", () => ({
  IN_TAURI: false,
  invoke: (cmd, args) => {
    calls.push({ cmd, args });
    if (cmd === "core_status") {
      return Promise.resolve({
        running: false,
        hidden: false,
        elevated: false,
        monitoring: true,
        auto_hide_enabled: false,
      });
    }
    return Promise.resolve(null);
  },
}));

const { app, setAnalyticsConsent } = await import("./state.svelte.js");

const tracked = (event) =>
  calls.filter((c) => c.cmd === "analytics_track" && c.args?.event === event);

const SRC = fileURLToPath(new URL("..", import.meta.url));
const RUST = fileURLToPath(new URL("../../../src-tauri/src/analytics.rs", import.meta.url));

/** Rust 侧登记的事件白名单。 */
function rustEvents() {
  const source = readFileSync(RUST, "utf8");
  const block = source.match(/pub const EVENTS: &\[&str\] = &\[([\s\S]*?)\];/);
  return [...block[1].matchAll(/"([a-z_]+)"/g)].map((m) => m[1]);
}

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return walk(path);
    return /\.(js|svelte)$/.test(path) && !path.endsWith(".test.js") ? [path] : [];
  });
}

/** 界面实际会上报的事件名：配置项表 + 各处的 track("…") 调用。 */
function usedEvents() {
  const names = new Set(CONFIG_EVENTS);
  for (const file of walk(SRC)) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(/\btrack\(\s*"([a-z_]+)"/g)) names.add(m[1]);
  }
  return names;
}

describe("事件白名单", () => {
  it("界面上报的事件都在 Rust 侧登记过", () => {
    const registered = new Set(rustEvents());
    const missing = [...usedEvents()].filter((name) => !registered.has(name));
    expect(missing, "这些事件后端会丢弃，须先加进 analytics::EVENTS").toEqual([]);
  });

  it("Rust 侧登记的事件都还有人在用", () => {
    const used = usedEvents();
    const stale = rustEvents().filter((name) => !used.has(name));
    expect(stale, "界面已不再上报，从 analytics::EVENTS 里删掉").toEqual([]);
  });
});

describe("trackConfigChanges", () => {
  const base = () => ({
    setting: { language: "auto", freeze_after_hide: false },
    hotkey: { hide_hotkey: "" },
    whitelist: [{ process: "explorer.exe" }],
  });

  it("首次调用只登记基线，不把整份设置当成刚改过", () => {
    const config = base();
    expect(trackConfigChanges(null, config)).toBe(config);
  });

  it("没有配置时保留原基线", () => {
    const before = base();
    expect(trackConfigChanges(before, null)).toBe(before);
  });

  it("返回本次快照供下次比较", () => {
    const after = base();
    after.setting.language = "en";
    expect(trackConfigChanges(base(), after)).toBe(after);
  });
});

describe("featureProps", () => {
  const config = () => ({
    setting: {
      mute_after_hide: true,
      auto_hide_enabled: true,
      top_left_hide: true,
      bottom_right_hide: true,
      freeze_after_hide: true,
      power_scope: "tree",
      mouse: { left: { enabled: true }, right: { enabled: true }, middle: {} },
      tray_clicks: { left: "toggle", double: "menu", right: "settings" },
    },
    hotkey: { hide_hotkey: "Ctrl+Q", close_hotkey: "", hide_hook: true },
    window_rules: [{ title: "微信" }, { regex: "^记事本" }],
    process_rules: [{ process: "TiMi.exe" }],
    whitelist: [{ process: "explorer.exe" }, { regex: "steam.*" }],
  });

  it("规则与白名单只报条数，用正则的那些单独计数", () => {
    const p = featureProps(config());
    expect(p.window_rules).toBe(2);
    expect(p.window_regex).toBe(1);
    expect(p.process_rules).toBe(1);
    expect(p.process_regex).toBe(0);
    expect(p.whitelist).toBe(2);
    expect(p.whitelist_regex).toBe(1);
  });

  it("窗口标题、进程名与正则式一个都不出现在属性里", () => {
    const json = JSON.stringify(featureProps(config()));
    for (const leak of ["微信", "记事本", "TiMi.exe", "explorer.exe", "steam"]) {
      expect(json, leak).not.toContain(leak);
    }
  });

  it("热键报实际组合，没设的报 none", () => {
    const p = featureProps(config());
    expect(p.combo_hide).toBe("Ctrl+Q");
    expect(p.combo_close).toBe("none");
    expect(p.hooks).toBe(1);
    expect(p.intercepts).toBe(0);
  });

  it("触发方式只报启用的个数", () => {
    const p = featureProps(config());
    expect(p.corners).toBe(2);
    expect(p.mouse_triggers).toBe(2);
  });

  it("静音归在隐藏选项里", () => {
    expect(featureProps(config()).mute).toBe(true);
    const entry = WATCHED.find(([path]) => path === "setting.mute_after_hide");
    expect(entry?.[1]).toBe("hide");
  });

  it("缺字段时回落默认，不抛错", () => {
    const p = featureProps({});
    expect(p.window_rules).toBe(0);
    expect(p.combo_hide).toBe("none");
    expect(p.freeze_scope).toBe("self");
    expect(p.locale).toBe("unknown");
  });
});

describe("setAnalyticsConsent", () => {
  beforeEach(() => {
    calls.length = 0;
    app.config = {
      setting: {},
      hotkey: {},
      verhub: { analytics: null, analytics_consent_sent: false },
    };
  });

  it("同意的那一刻报一次", async () => {
    await setAnalyticsConsent(true, "first_run");
    expect(tracked("analytics_consent")).toHaveLength(1);
    expect(app.config.verhub.analytics_consent_sent).toBe(true);
  });

  it("同一台设备反复开关不再重复报", async () => {
    await setAnalyticsConsent(true, "first_run");
    await setAnalyticsConsent(false, "settings");
    await setAnalyticsConsent(true, "settings");
    await setAnalyticsConsent(false, "settings");
    await setAnalyticsConsent(true, "settings");
    expect(tracked("analytics_consent")).toHaveLength(1);
  });

  it("已报过的设备重新启动也不会再报", async () => {
    app.config.verhub = { analytics: true, analytics_consent_sent: true };
    await setAnalyticsConsent(true, "settings");
    expect(tracked("analytics_consent")).toHaveLength(0);
  });

  it("拒绝时一条都不发", async () => {
    await setAnalyticsConsent(false, "first_run");
    expect(calls.filter((c) => c.cmd === "analytics_track")).toHaveLength(0);
    expect(app.config.verhub.analytics_consent_sent).toBe(false);
  });
});
