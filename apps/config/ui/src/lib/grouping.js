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
