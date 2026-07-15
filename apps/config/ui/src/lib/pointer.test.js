import { describe, expect, it } from "vitest";
import {
  CORNERS,
  CORNER_CENTER,
  buildTimeline,
  cursorTarget,
  describeTrigger,
  enabledParts,
  nextIndex,
} from "./pointer.js";

describe("enabledParts", () => {
  it("按原顺序返回被启用的角落", () => {
    const setting = { top_left_hide: false, top_right_hide: true, bottom_right_hide: true };
    expect(enabledParts(CORNERS, setting).map((c) => c.key)).toEqual([
      "top_right_hide",
      "bottom_right_hide",
    ]);
  });

  it("setting 缺失或全关时返回空数组", () => {
    expect(enabledParts(CORNERS, undefined)).toEqual([]);
    expect(enabledParts(CORNERS, {})).toEqual([]);
  });
});

describe("describeTrigger", () => {
  it("未启用的键直接说未启用", () => {
    expect(describeTrigger({ enabled: false, clicks: 3, modifiers: "Ctrl" })).toBe("未启用");
    expect(describeTrigger(undefined)).toBe("未启用");
  });

  it("连击次数说成单击 / 双击 / 三击", () => {
    expect(describeTrigger({ enabled: true, clicks: 1, modifiers: "" })).toBe("单击");
    expect(describeTrigger({ enabled: true, clicks: 2, modifiers: "" })).toBe("双击");
    expect(describeTrigger({ enabled: true, clicks: 3, modifiers: "" })).toBe("三击");
  });

  it("带修饰键时拼在前面", () => {
    expect(describeTrigger({ enabled: true, clicks: 3, modifiers: "Ctrl+Shift" })).toBe(
      "Ctrl+Shift + 三击",
    );
  });

  it("越界的连击次数被夹回 1..3", () => {
    expect(describeTrigger({ enabled: true, clicks: 0, modifiers: "" })).toBe("单击");
    expect(describeTrigger({ enabled: true, clicks: 9, modifiers: "" })).toBe("三击");
  });
});

describe("nextIndex", () => {
  it("环形推进", () => {
    expect(nextIndex(3, 0)).toBe(1);
    expect(nextIndex(3, 2)).toBe(0);
  });

  it("空列表恒为 0", () => {
    expect(nextIndex(0, 5)).toBe(0);
  });
});

describe("buildTimeline", () => {
  const tl = ($) =>
    buildTimeline(enabledParts(CORNERS, $.setting), $.restore, $.fast ?? true);

  it("没有选中的角时时间轴为空", () => {
    expect(buildTimeline([], true)).toEqual([]);
  });

  it("单个角：甩过去 → 隐藏 → 再甩一次 → 恢复", () => {
    const frames = tl({ setting: { top_left_hide: true }, restore: true });
    expect(frames.map((f) => f.visible)).toEqual([true, false, false, false, true]);
    expect(frames.every((f) => f.corner === "top_left_hide")).toBe(true);
    // 「鼠标离开角落」那帧光标回到中央，其余帧都停在该角
    expect(frames[2].cursor).toEqual(CORNER_CENTER);
    expect(frames[3].cursor).toEqual(CORNERS[0].cursor);
  });

  it("关闭「移动恢复」时不演示再次甩角，改为提示用热键恢复", () => {
    const frames = tl({ setting: { top_left_hide: true }, restore: false });
    expect(frames.map((f) => f.visible)).toEqual([true, false, true]);
    expect(frames[2].caption).toContain("热键");
  });

  it("「仅快速移动」时，冲向角落的帧标为 fast、配文也说快速甩", () => {
    const frames = tl({ setting: { top_left_hide: true }, restore: true, fast: true });
    expect(frames.map((f) => Boolean(f.fast))).toEqual([true, false, false, true, false]);
    expect(frames[0].caption).toContain("快速移动");
  });

  it("关掉「仅快速移动」后没有 fast 帧，配文改成普通移动", () => {
    const frames = tl({ setting: { top_left_hide: true }, restore: true, fast: false });
    expect(frames.some((f) => f.fast)).toBe(false);
    expect(frames[0].caption).toContain("移动到");
    expect(frames[0].caption).not.toContain("快速");
  });

  it("多个角按左上→右上→左下→右下逐个演示完再演下一个", () => {
    const frames = tl({
      setting: { bottom_right_hide: true, top_right_hide: true },
      restore: false,
    });
    expect(frames.map((f) => f.corner)).toEqual([
      "top_right_hide",
      "top_right_hide",
      "top_right_hide",
      "bottom_right_hide",
      "bottom_right_hide",
      "bottom_right_hide",
    ]);
  });
});

describe("cursorTarget", () => {
  it("没有选中的角时停在屏幕中央", () => {
    expect(cursorTarget([], 0)).toEqual(CORNER_CENTER);
  });

  it("在选中的角之间轮流停靠", () => {
    const picked = enabledParts(CORNERS, {
      top_right_hide: true,
      bottom_left_hide: true,
    });
    expect(cursorTarget(picked, 0)).toEqual(picked[0].cursor);
    expect(cursorTarget(picked, 1)).toEqual(picked[1].cursor);
    expect(cursorTarget(picked, 2)).toEqual(picked[0].cursor);
  });
});
