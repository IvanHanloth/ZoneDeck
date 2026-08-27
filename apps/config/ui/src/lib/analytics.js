// 匿名使用统计：记录哪些功能真被用到，用来判断该往哪里投入。
//
// 事件名须在 Rust 侧 analytics::EVENTS 登记，未登记的后端直接丢弃；属性只放开关、
// 计数、固定枚举与热键组合，窗口标题、进程名、文件路径、正则式与反馈正文一概不报。

import { invoke } from "./ipc.js";

/**
 * 记一次事件。未获授权时后端会丢弃，调用方无需自己判断。
 * 不 await：埋点不该拖慢交互，失败也不该冒泡成界面错误。
 */
export function track(event, props) {
  invoke("analytics_track", { event, props: props ?? null }).catch(() => {});
}

/** 应用授权状态：同意或撤回，由首次征询弹窗与设置页开关调用。 */
export function setConsent(granted) {
  return invoke("analytics_set_consent", { granted });
}

/**
 * 把攒着的事件发出去；关窗前调一次，否则最后一批要等下次启动补发。
 * 网络慢时不拖着窗口不放——队列是落盘的，没发完的下次启动会补上。
 */
export function flush(timeoutMs = 800) {
  return Promise.race([
    invoke("analytics_flush").catch(() => {}),
    new Promise((resolve) => setTimeout(resolve, timeoutMs)),
  ]);
}

/** 五个热键动作，各有「组合键 / 走钩子 / 拦截」三项设置。 */
const HOTKEY_ACTIONS = ["hide", "close", "hide_only", "show_only", "hide_foreground"];
const MOUSE_BUTTONS = ["left", "middle", "right"];
const CORNERS = ["top_left", "top_right", "bottom_left", "bottom_right"];

const HOTKEY_WATCHED = HOTKEY_ACTIONS.flatMap((a) => [
  [`hotkey.${a}_hotkey`, "hotkey", a],
  [`hotkey.${a}_hook`, "hotkey", `${a}_hook`],
  [`hotkey.${a}_intercept`, "hotkey", `${a}_intercept`],
]);

const MOUSE_WATCHED = MOUSE_BUTTONS.flatMap((b) => [
  [`setting.mouse.${b}.enabled`, "mouse", b],
  [`setting.mouse.${b}.clicks`, "mouse", `${b}_clicks`],
  [`setting.mouse.${b}.modifiers`, "mouse", `${b}_modifiers`],
]);

/**
 * 关心的配置项：[路径, 分组, item 标识]。改动时报一条 `setting_changed`。
 * 值原样上报，因此表里只放开关、计数、固定枚举与热键组合——窗口规则、进程规则
 * 与白名单的内容不在表内，它们只在快照里报条数。
 */
export const WATCHED = [
  ["setting.hide_current", "hide", "hide_current"],
  ["setting.minimize_before_hide", "hide", "minimize_before"],
  ["setting.send_before_hide", "hide", "send_before"],
  ["setting.mute_after_hide", "hide", "mute"],
  ["setting.hide_icon_after_hide", "hide", "hide_tray_icon"],
  ["setting.auto_hide_enabled", "hide", "auto_hide"],
  ["setting.auto_hide_time", "hide", "auto_hide_time"],
  ["setting.show_float_window", "hide", "float_window"],

  ...HOTKEY_WATCHED,
  ...MOUSE_WATCHED,
  ["setting.middle_button_hide", "mouse", "middle_shortcut"],
  ["setting.mouse.allow_click_restore", "mouse", "click_restore"],

  ...CORNERS.map((c) => [`setting.${c}_hide`, "corner", c]),
  ["setting.corner_fast_only", "corner", "fast_only"],
  ["setting.allow_move_restore", "corner", "move_restore"],

  ["setting.freeze_after_hide", "power", "freeze"],
  ["setting.enhanced_freeze", "power", "enhanced_freeze"],
  ["setting.freeze_whole_tree", "power", "whole_tree"],
  ["setting.trim_memory_after_freeze", "power", "trim_memory"],
  ["setting.power_scope", "power", "freeze_scope"],
  ["setting.efficiency_after_hide", "power", "efficiency"],
  ["setting.efficiency_scope", "power", "efficiency_scope"],

  ["setting.tray_enabled", "tray", "enabled"],
  ["setting.tray_show_tooltip", "tray", "tooltip"],
  ["setting.tray_clicks.left", "tray", "click_left"],
  ["setting.tray_clicks.double", "tray", "click_double"],
  ["setting.tray_clicks.right", "tray", "click_right"],
];

/** 本模块会用到的事件名，供跨语言一致性测试比对。 */
export const CONFIG_EVENTS = ["features", "setting_changed"];

function at(config, path) {
  return path.split(".").reduce((node, key) => (node == null ? undefined : node[key]), config);
}

/** 组合键留空即未设；报 "none" 比报空串好统计。 */
function combo(value) {
  const text = String(value ?? "").trim();
  return text || "none";
}

function count(list, predicate) {
  return Array.isArray(list) ? list.filter(predicate ?? (() => true)).length : 0;
}

const isRegex = (rule) => !!rule?.regex;

/**
 * 启动时的功能采用快照：哪些功能开着、规则与白名单各有几条、热键设成了什么。
 * 「多少人在用某个功能」只能靠它算——变更事件算出来的是改动次数，不是人数。
 */
export function trackFeatures(config, env = {}) {
  if (!config) return;
  track("features", featureProps(config, env));
}

/** 快照具体带哪些字段；单独抽出来，便于审阅与测试。 */
export function featureProps(config, env = {}) {
  const s = config.setting ?? {};
  const h = config.hotkey ?? {};
  const mouse = s.mouse ?? {};
  const clicks = s.tray_clicks ?? {};

  return {
    locale: env.locale ?? "unknown",
    install: env.install ?? "unknown",
    elevated: !!env.elevated,
    autostart: !!env.autostart,

    // 隐藏行为：只有开关本身，不含隐藏了什么
    hide_current: !!s.hide_current,
    minimize_before: !!s.minimize_before_hide,
    send_before: !!s.send_before_hide,
    mute: !!s.mute_after_hide,
    hide_tray_icon: !!s.hide_icon_after_hide,
    auto_hide: !!s.auto_hide_enabled,
    float_window: !!s.show_float_window,

    // 触发方式的规模
    corners: CORNERS.filter((c) => s[`${c}_hide`]).length,
    mouse_triggers: MOUSE_BUTTONS.filter((b) => mouse[b]?.enabled).length,
    hooks: HOTKEY_ACTIONS.filter((a) => h[`${a}_hook`]).length,
    intercepts: HOTKEY_ACTIONS.filter((a) => h[`${a}_intercept`]).length,

    // 各动作实际设成了什么组合键
    combo_hide: combo(h.hide_hotkey),
    combo_close: combo(h.close_hotkey),
    combo_hide_only: combo(h.hide_only_hotkey),
    combo_show_only: combo(h.show_only_hotkey),
    combo_hide_foreground: combo(h.hide_foreground_hotkey),

    // 规则与白名单：只有条数，没有标题、进程名与正则式
    window_rules: count(config.window_rules),
    window_regex: count(config.window_rules, isRegex),
    process_rules: count(config.process_rules),
    process_regex: count(config.process_rules, isRegex),
    whitelist: count(config.whitelist),
    whitelist_regex: count(config.whitelist, isRegex),

    freeze: !!s.freeze_after_hide,
    enhanced_freeze: !!s.enhanced_freeze,
    whole_tree: !!s.freeze_whole_tree,
    trim_memory: !!s.trim_memory_after_freeze,
    freeze_scope: s.power_scope ?? "self",
    efficiency: !!s.efficiency_after_hide,
    efficiency_scope: s.efficiency_scope ?? "self",

    tray: !!s.tray_enabled,
    click_left: clicks.left ?? "",
    click_double: clicks.double ?? "",
    click_right: clicks.right ?? "",
  };
}

/**
 * 比较前后两份配置，把关心的改动记成事件。首次调用（`before` 为空）只登记基线，
 * 不上报——否则每次启动都会把整份设置当成「刚改过」重报一遍。
 * 返回本次的快照，供下次调用传回。
 */
export function trackConfigChanges(before, after) {
  if (!after) return before;
  if (!before) return after;
  for (const [path, group, item] of WATCHED) {
    const now = at(after, path);
    if (now === at(before, path)) continue;
    track("setting_changed", { group, item, value: now });
  }
  return after;
}
