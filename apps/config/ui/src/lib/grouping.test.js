import { describe, expect, it } from "vitest";
import {
  availableWindows,
  groupByProcess,
  iconPathsToFetch,
  moveWindows,
  sameWindow,
} from "./grouping.js";

const win = (title, hwnd, process, path = "C:\\" + process) => ({
  title,
  hwnd,
  process,
  PID: hwnd,
  path,
});

describe("groupByProcess", () => {
  it("按进程聚合并携带路径", () => {
    const groups = groupByProcess([
      win("微信", 1, "WeChat.exe"),
      win("传输助手", 2, "WeChat.exe"),
      win("记事本", 3, "notepad.exe"),
    ]);
    expect(groups.map((g) => g.process)).toEqual(["notepad.exe", "WeChat.exe"]);
    expect(groups[1].windows).toHaveLength(2);
    expect(groups[0].path).toBe("C:\\notepad.exe");
  });

  it("空进程名归入未知分组", () => {
    const groups = groupByProcess([win("孤儿窗口", 9, "")]);
    expect(groups[0].process).toBe("（未知进程）");
  });
});

describe("moveWindows / availableWindows", () => {
  it("移动选中的窗口且不修改入参", () => {
    const a = [win("一", 1, "a.exe"), win("二", 2, "b.exe")];
    const b = [win("三", 3, "c.exe")];
    const { from, to } = moveWindows(a, b, [a[0]]);
    expect(from.map((w) => w.hwnd)).toEqual([2]);
    expect(to.map((w) => w.hwnd)).toEqual([3, 1]);
    expect(a).toHaveLength(2, "入参不应被修改");
  });

  it("availableWindows 排除已绑定项", () => {
    const all = [win("一", 1, "a.exe"), win("二", 2, "b.exe")];
    expect(availableWindows(all, [all[1]]).map((w) => w.hwnd)).toEqual([1]);
  });

  it("sameWindow 需要 hwnd 与进程同时相同", () => {
    expect(sameWindow(win("x", 1, "a.exe"), win("y", 1, "a.exe"))).toBe(true);
    expect(sameWindow(win("x", 1, "a.exe"), win("x", 1, "b.exe"))).toBe(false);
  });
});

describe("iconPathsToFetch", () => {
  it("去重、跳过空路径与已缓存路径（含负缓存）", () => {
    const windows = [
      win("一", 1, "a.exe", "C:\\a.exe"),
      win("二", 2, "a.exe", "C:\\a.exe"),
      win("三", 3, "b.exe", "C:\\b.exe"),
      win("四", 4, "c.exe", ""),
      win("五", 5, "d.exe", "C:\\d.exe"),
    ];
    const cache = { "C:\\b.exe": "data:...", "C:\\d.exe": null };
    expect(iconPathsToFetch(windows, cache)).toEqual(["C:\\a.exe"]);
  });
});
