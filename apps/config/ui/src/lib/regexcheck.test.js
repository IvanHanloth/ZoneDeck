import { describe, expect, it, vi } from "vitest";
import { collectRegexPatterns, createBreadthGuard } from "./regexcheck.js";

const config = () => ({
  window_rules: [{ regex: ".*" }, { title: "微信", hwnd: 1 }],
  process_rules: [{ regex: "^项目" }],
  whitelist: [{ regex: ".*" }, { process: "a.exe" }],
});

describe("collectRegexPatterns", () => {
  it("只收集三个列表里的非空正则，并记下来源", () => {
    expect(collectRegexPatterns(config())).toEqual([
      { kind: "window", index: 0, pattern: ".*" },
      { kind: "process", index: 0, pattern: "^项目" },
      { kind: "whitelist", index: 0, pattern: ".*" },
    ]);
  });

  it("空正则、缺字段与非数组都不报错", () => {
    expect(collectRegexPatterns({ window_rules: [{ regex: "" }] })).toEqual([]);
    expect(collectRegexPatterns({ whitelist: null })).toEqual([]);
    expect(collectRegexPatterns(null)).toEqual([]);
  });
});

describe("createBreadthGuard", () => {
  it("重复的正则只送后端一次", async () => {
    const measure = vi.fn().mockResolvedValue([200, 3]);
    await createBreadthGuard(measure).inspect(config());
    expect(measure).toHaveBeenCalledWith([".*", "^项目"]);
  });

  it("只报命中数超过阈值的，且同一条正则只报一次", async () => {
    const guard = createBreadthGuard(vi.fn().mockResolvedValue([200, 3]));
    const { broad, toWarn } = await guard.inspect(config());
    expect(broad).toEqual([{ kind: "window", pattern: ".*", hits: 200 }]);
    expect(toWarn).toEqual(broad);
  });

  it("恰好等于阈值不算过宽", async () => {
    const guard = createBreadthGuard(vi.fn().mockResolvedValue([100, 101]), 100);
    const { broad } = await guard.inspect({
      window_rules: [{ regex: "a" }, { regex: "b" }],
    });
    expect(broad.map((f) => f.pattern)).toEqual(["b"]);
  });

  it("「仍然保存」后不再弹窗，也不再标红", async () => {
    const guard = createBreadthGuard(vi.fn().mockResolvedValue([200, 3]));
    guard.acknowledge([".*"]);
    const { broad, toWarn } = await guard.inspect(config());
    expect(broad).toEqual([]);
    expect(toWarn).toEqual([]);
  });

  it("「我知道了」后不再弹窗，但保持标红", async () => {
    const guard = createBreadthGuard(vi.fn().mockResolvedValue([200, 3]));
    guard.dismiss([".*"]);
    const { broad, toWarn } = await guard.inspect(config());
    expect(broad.map((f) => f.pattern)).toEqual([".*"]);
    expect(toWarn).toEqual([]);
  });

  it("改成另一条同样过宽的正则会重新弹窗", async () => {
    const guard = createBreadthGuard(vi.fn().mockResolvedValue([200]));
    guard.dismiss([".*"]);
    const { toWarn } = await guard.inspect({ window_rules: [{ regex: "[\\s\\S]*" }] });
    expect(toWarn.map((f) => f.pattern)).toEqual(["[\\s\\S]*"]);
  });

  it("编译失败的正则（null）不判过宽", async () => {
    const guard = createBreadthGuard(vi.fn().mockResolvedValue([null]));
    const { broad } = await guard.inspect({ window_rules: [{ regex: "(" }] });
    expect(broad).toEqual([]);
  });

  // 检查本身失败不该拦住保存。
  it("后端出错时视为无可疑项", async () => {
    const guard = createBreadthGuard(vi.fn().mockRejectedValue(new Error("断了")));
    expect(await guard.inspect(config())).toEqual({ broad: [], toWarn: [] });
  });

  it("没有正则时根本不调后端", async () => {
    const measure = vi.fn();
    const guard = createBreadthGuard(measure);
    expect(await guard.inspect({ window_rules: [] })).toEqual({ broad: [], toWarn: [] });
    expect(measure).not.toHaveBeenCalled();
  });
});
