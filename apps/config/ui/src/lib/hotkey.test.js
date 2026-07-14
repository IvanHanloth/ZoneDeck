import { describe, expect, it } from "vitest";
import { comboFromEvent, keyName, modifiersFromEvent } from "./hotkey.js";

const ev = (key, mods = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("modifiersFromEvent", () => {
  it("没按修饰键时返回空串", () => {
    expect(modifiersFromEvent(ev("Control"))).toBe("");
  });

  it("按固定顺序拼接 Ctrl / Alt / Shift / Win", () => {
    expect(modifiersFromEvent(ev("Control", { ctrlKey: true }))).toBe("Ctrl");
    expect(
      modifiersFromEvent(ev("Shift", { shiftKey: true, ctrlKey: true, metaKey: true })),
    ).toBe("Ctrl+Shift+Win");
    expect(modifiersFromEvent(ev("Alt", { altKey: true, shiftKey: true }))).toBe("Alt+Shift");
  });
});

describe("keyName", () => {
  it("字母数字统一为大写", () => {
    expect(keyName(ev("q"))).toBe("Q");
    expect(keyName(ev("A"))).toBe("A");
    expect(keyName(ev("7"))).toBe("7");
  });

  it("支持功能键 F1–F24", () => {
    expect(keyName(ev("F1"))).toBe("F1");
    expect(keyName(ev("F12"))).toBe("F12");
    expect(keyName(ev("F24"))).toBe("F24");
    expect(keyName(ev("F25"))).toBeNull();
  });

  it("特殊键映射为核心可解析的名称", () => {
    expect(keyName(ev("Escape"))).toBe("Esc");
    expect(keyName(ev(" "))).toBe("Space");
    expect(keyName(ev("ArrowUp"))).toBe("Up");
  });

  it("不支持的键返回 null", () => {
    expect(keyName(ev("CapsLock"))).toBeNull();
    expect(keyName(ev("½"))).toBeNull();
  });
});

describe("comboFromEvent", () => {
  it("组合修饰键并按 Ctrl/Alt/Shift/Win 顺序输出", () => {
    expect(
      comboFromEvent(ev("q", { ctrlKey: true, altKey: true })),
    ).toBe("Ctrl+Alt+Q");
    expect(
      comboFromEvent(ev("Escape", { metaKey: true, shiftKey: true })),
    ).toBe("Shift+Win+Esc");
  });

  it("无修饰键时只有主键", () => {
    expect(comboFromEvent(ev("F5"))).toBe("F5");
  });

  it("仅按下修饰键时返回 null（等待主键）", () => {
    expect(comboFromEvent(ev("Control", { ctrlKey: true }))).toBeNull();
    expect(comboFromEvent(ev("Shift", { shiftKey: true }))).toBeNull();
  });

  it("主键不支持时返回 null", () => {
    expect(comboFromEvent(ev("CapsLock", { ctrlKey: true }))).toBeNull();
  });
});
