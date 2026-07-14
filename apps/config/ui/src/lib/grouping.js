// 窗口列表分组与集合运算。纯函数，便于单元测试。

/** 两个 WindowInfo 是否指同一窗口（枚举快照内 hwnd + 进程名唯一）。 */
export function sameWindow(a, b) {
  return a.hwnd === b.hwnd && a.process === b.process;
}

/** 按进程名分组并排序，返回 [{ process, path, windows }]。 */
export function groupByProcess(windows) {
  const groups = new Map();
  for (const w of windows) {
    const key = w.process || "（未知进程）";
    if (!groups.has(key)) {
      groups.set(key, { process: key, path: w.path || "", windows: [] });
    }
    groups.get(key).windows.push(w);
  }
  return [...groups.values()].sort((a, b) =>
    a.process.localeCompare(b.process, "zh-CN"),
  );
}

/** 把 picked 从 from 移到 to，返回新的 { from, to }（不修改入参）。 */
export function moveWindows(from, to, picked) {
  const moved = from.filter((w) => picked.some((p) => sameWindow(p, w)));
  return {
    from: from.filter((w) => !picked.some((p) => sameWindow(p, w))),
    to: [...to, ...moved],
  };
}

/** 全量窗口里排除已绑定的，得到"可添加"列表。 */
export function availableWindows(all, bound) {
  return all.filter((w) => !bound.some((b) => sameWindow(b, w)));
}

/** 需要请求图标的去重路径列表（跳过已缓存的，含"无图标"负缓存）。 */
export function iconPathsToFetch(windows, cache) {
  return [...new Set(windows.map((w) => w.path))].filter(
    (p) => p && !(p in cache),
  );
}

/** 按标题 / 进程名 / 路径模糊搜索（不区分大小写）。空查询返回全部。 */
export function filterWindows(windows, query) {
  const q = (query || "").trim().toLowerCase();
  if (!q) return windows;
  return windows.filter((w) =>
    [w.title, w.process, w.path]
      .filter(Boolean)
      .some((s) => s.toLowerCase().includes(q)),
  );
}

/** 拆成「可见」与「后台（不可见）」两组。visible 缺省视为 true（兼容旧数据）。 */
export function splitByVisibility(windows) {
  const visible = [];
  const hidden = [];
  for (const w of windows) {
    (w.visible === false ? hidden : visible).push(w);
  }
  return { visible, hidden };
}

/** 与核心 NO_TITLE 常量一致，用于识别无标题窗口。 */
export const NO_TITLE = "无标题窗口";

// ---- 规则构造与追溯（与核心 matching.rs 语义同构） ----

/** 转义正则元字符，把任意字面量安全地嵌进正则里。 */
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

/** 由现有窗口构造一条「进程」精确规则（按可执行文件路径）。 */
export function processRuleFromWindow(w) {
  return {
    process: w.process,
    path: w.path,
    by_name: false,
    // 「隐藏整个程序」默认连它的无标题窗口一起藏；后台不可见窗口则不动。
    include_untitled: true,
    include_background: false,
  };
}

/**
 * 新的「窗口」正则规则（高级模式：标题正则）。
 * 默认值是一条「包含式」正则——优先用左侧选中窗口的标题作为种子。
 */
export function newWindowRegexRule(seedTitle) {
  const seed = seedTitle && seedTitle !== NO_TITLE ? seedTitle : "关键词";
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

/**
 * 新的「进程」正则规则（高级模式）。默认作用于完整路径，种子取选中窗口的进程名——
 * 「包含式」正则对路径和文件名两种模式都成立。
 */
export function newProcessRegexRule(seedProcess) {
  return {
    process: "",
    path: "",
    regex: containsPattern(seedProcess || "程序名.exe"),
    by_name: false,
    include_untitled: true,
    include_background: false,
  };
}

/** 是否为正则（高级）规则。 */
export function isRegexRule(rule) {
  return rule && rule.regex != null;
}

/** 一条窗口规则是否与某窗口指向同一目标（用于去重，仅针对精确规则）。 */
function windowRuleCoversWindow(rule, w) {
  if (isRegexRule(rule)) return false;
  if (rule.hwnd && rule.hwnd === w.hwnd) return true;
  return !!rule.path && rule.path === w.path && rule.title === w.title;
}

/**
 * 把选中的窗口追加为「窗口」精确规则，跳过已被现有规则覆盖的（去重）。
 * 不修改入参，返回新数组。
 */
export function addWindowRules(existing, pickedWindows) {
  const result = existing.slice();
  for (const w of pickedWindows) {
    if (result.some((r) => windowRuleCoversWindow(r, w))) continue;
    result.push(windowRuleFromWindow(w));
  }
  return result;
}

/**
 * 把选中窗口的「进程」按可执行文件路径去重后追加为进程规则。
 * 已存在同路径规则、或路径为空的窗口都会跳过。不修改入参。
 */
export function addProcessRules(existing, pickedWindows) {
  const result = existing.slice();
  const seen = new Set(result.filter((r) => !isRegexRule(r)).map((r) => r.path));
  for (const w of pickedWindows) {
    if (!w.path || seen.has(w.path)) continue;
    seen.add(w.path);
    result.push(processRuleFromWindow(w));
  }
  return result;
}

/**
 * 追溯一条窗口精确规则在当前存活窗口中的状态：
 * - `regex`：正则规则（不参与句柄追溯）；
 * - `live`：句柄仍命中原窗口；
 * - `reacquired`：句柄失效，但按标题 + 进程路径追溯到新窗口；
 * - `missing`：窗口已关闭 / 进程已重启且无法追溯。
 */
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

/**
 * 现有窗口列表过滤：默认只显示有标题的可见窗口。
 * showBackground 放开后台（不可见）窗口，showUntitled 放开无标题窗口，最后按 search 过滤。
 */
export function applyListFilters(
  windows,
  { showBackground = false, showUntitled = false, search = "" } = {},
) {
  let list = windows;
  if (!showBackground) list = list.filter((w) => w.visible !== false);
  if (!showUntitled) list = list.filter((w) => w.title && w.title !== NO_TITLE);
  return filterWindows(list, search);
}
