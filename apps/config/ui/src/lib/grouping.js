// 窗口列表分组与集合运算。

import { t } from "./i18n.svelte.js";

/** 按进程名分组并排序，返回 [{ process, path, windows }]。 */
export function groupByProcess(windows) {
  const groups = new Map();
  for (const w of windows) {
    const key = w.process || t("common.unknownProcess");
    if (!groups.has(key)) {
      groups.set(key, { process: key, path: w.path || "", windows: [] });
    }
    groups.get(key).windows.push(w);
  }
  return [...groups.values()].sort((a, b) =>
    a.process.localeCompare(b.process, "zh-CN"),
  );
}

/** 需要请求图标的去重路径列表，跳过已缓存的。 */
export function iconPathsToFetch(windows, cache) {
  return [...new Set(windows.map((w) => w.path))].filter(
    (p) => p && !(p in cache),
  );
}

/** 按标题 / 进程名 / 路径模糊搜索，不区分大小写；空查询返回全部。 */
export function filterWindows(windows, query) {
  const q = (query || "").trim().toLowerCase();
  if (!q) return windows;
  return windows.filter((w) =>
    [w.title, w.process, w.path]
      .filter(Boolean)
      .some((s) => s.toLowerCase().includes(q)),
  );
}

/** 拆成「可见」与「后台」两组；visible 缺省视为 true。 */
export function splitByVisibility(windows) {
  const visible = [];
  const hidden = [];
  for (const w of windows) {
    (w.visible === false ? hidden : visible).push(w);
  }
  return { visible, hidden };
}

/** 与核心 `zonedeck_common::NO_TITLE` 一致的占位标题；不随界面语言变化。 */
export const NO_TITLE = "无标题窗口";

/** 转义正则元字符。 */
export function escapeRegex(text) {
  return String(text).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** 「包含某个值」的正则模板，例如 微信 → `.*微信.*`。 */
export function containsPattern(value) {
  return `.*${escapeRegex(value)}.*`;
}

/** 由现有窗口构造一条「窗口」精确规则。 */
export function windowRuleFromWindow(w) {
  return {
    title: w.title,
    hwnd: w.hwnd,
    process: w.process,
    PID: w.PID,
    path: w.path,
    include_untitled: false,
    include_background: false,
  };
}

/** 能否据此窗口构造按进程匹配的规则：映像名与完整路径至少要有一个。 */
export function hasProcessIdentity(w) {
  return !!(w?.process || w?.path);
}

/** 由现有窗口构造一条「进程」精确规则；查不到完整路径时退回按文件名匹配。 */
export function processRuleFromWindow(w) {
  return {
    process: w.process,
    path: w.path,
    by_name: !w.path,
    include_untitled: false,
    include_background: false,
  };
}

/** 新的「窗口」正则规则（标题正则）。 */
export function newWindowRegexRule(seedTitle) {
  const seed = seedTitle && seedTitle !== NO_TITLE ? seedTitle : t("binding.regexSeedKeyword");
  return {
    title: NO_TITLE,
    hwnd: 0,
    process: "",
    PID: 0,
    path: "",
    regex: containsPattern(seed),
    include_untitled: false,
    include_background: false,
  };
}

/** 新的「进程」正则规则。 */
export function newProcessRegexRule(seedProcess) {
  return {
    process: "",
    path: "",
    regex: containsPattern(seedProcess || t("binding.regexSeedProcess")),
    by_name: false,
    include_untitled: false,
    include_background: false,
  };
}

/** 由现有窗口构造一条白名单条目；三个忽略开关置假，默认按文件名匹配。 */
export function whitelistRuleFromWindow(w) {
  return {
    process: w.process,
    path: w.path,
    by_name: true,
    ignore_hide: false,
    ignore_freeze: false,
    ignore_mute: false,
  };
}

/** 新的白名单正则条目（默认作用于文件名）。 */
export function newWhitelistRegexRule(seedProcess) {
  return {
    process: "",
    path: "",
    regex: containsPattern(seedProcess || t("binding.regexSeedProcess")),
    by_name: true,
    ignore_hide: false,
    ignore_freeze: false,
    ignore_mute: false,
  };
}

/** 精确规则的去重键：按文件名匹配看进程名，否则看路径。 */
function ruleKey(rule) {
  return (rule.by_name ? rule.process : rule.path)?.toLowerCase() || "";
}

/** 追加为白名单条目，跳过已有的同一目标，返回新数组。 */
export function addWhitelistRules(existing, pickedWindows) {
  const result = existing.slice();
  const seen = new Set(
    result.filter((r) => !isRegexRule(r)).map(ruleKey).filter(Boolean),
  );
  for (const w of pickedWindows) {
    if (!hasProcessIdentity(w)) continue;
    const rule = whitelistRuleFromWindow(w);
    const key = ruleKey(rule);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    result.push(rule);
  }
  return result;
}

export function isRegexRule(rule) {
  return rule && rule.regex != null;
}

/** 一条精确窗口规则是否与某窗口指向同一目标，用于去重。 */
function windowRuleCoversWindow(rule, w) {
  if (isRegexRule(rule)) return false;
  if (rule.hwnd && rule.hwnd === w.hwnd) return true;
  return !!rule.path && rule.path === w.path && rule.title === w.title;
}

/** 追加为「窗口」精确规则，跳过已覆盖项，返回新数组。 */
export function addWindowRules(existing, pickedWindows) {
  const result = existing.slice();
  for (const w of pickedWindows) {
    if (result.some((r) => windowRuleCoversWindow(r, w))) continue;
    result.push(windowRuleFromWindow(w));
  }
  return result;
}

/** 按匹配依据去重后追加为进程规则，跳过身份不明的窗口，返回新数组。 */
export function addProcessRules(existing, pickedWindows) {
  const result = existing.slice();
  const seen = new Set(
    result.filter((r) => !isRegexRule(r)).map(ruleKey).filter(Boolean),
  );
  for (const w of pickedWindows) {
    if (!hasProcessIdentity(w)) continue;
    const rule = processRuleFromWindow(w);
    const key = ruleKey(rule);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    result.push(rule);
  }
  return result;
}

/** 追溯窗口精确规则的状态：regex / live / reacquired / missing。 */
export function traceWindowRule(rule, liveWindows) {
  if (isRegexRule(rule)) return "regex";

  const byHwnd = liveWindows.find((w) => w.hwnd === rule.hwnd && rule.hwnd !== 0);
  if (byHwnd && (!rule.path || byHwnd.path === rule.path)) return "live";

  const usableTitle = rule.title && rule.title !== NO_TITLE;
  const traced =
    usableTitle &&
    liveWindows.find(
      (w) =>
        w.title === rule.title &&
        (rule.path ? w.path === rule.path : rule.process && w.process === rule.process),
    );
  return traced ? "reacquired" : "missing";
}

/** 现有窗口列表过滤：默认只显示有标题的可见窗口。 */
export function applyListFilters(
  windows,
  { showBackground = false, showUntitled = false, search = "" } = {},
) {
  let list = windows;
  if (!showBackground) list = list.filter((w) => w.visible !== false);
  if (!showUntitled) list = list.filter((w) => w.title && w.title !== NO_TITLE);
  return filterWindows(list, search);
}
