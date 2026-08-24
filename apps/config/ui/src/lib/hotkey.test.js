import { describe, expect, it } from "vitest";
import {
  isModifierKey,
  isModifierOnly,
  joinCombo,
  keyName,
  modifiersFromEvent,
  requiresHook,
  splitCombo,
} from "./hotkey.js";

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

  it("小键盘按 code 认，不受 NumLock 影响", () => {
    expect(keyName({ ...ev("0"), code: "Numpad0" })).toBe("Numpad0");
    expect(keyName({ ...ev("Insert"), code: "Numpad0" })).toBe("Numpad0");
    expect(keyName({ ...ev("+"), code: "NumpadAdd" })).toBe("NumpadAdd");
    // 主键盘的数字仍走 key。
    expect(keyName({ ...ev("0"), code: "Digit0" })).toBe("0");
  });

  it("OEM 符号键按 code 映射为位置名", () => {
    expect(keyName({ ...ev(";"), code: "Semicolon" })).toBe("OEM_1");
    expect(keyName({ ...ev("="), code: "Equal" })).toBe("OEM_PLUS");
    expect(keyName({ ...ev("`"), code: "Backquote" })).toBe("OEM_3");
    expect(keyName({ ...ev("ContextMenu"), code: "ContextMenu" })).toBe("Apps");
  });

  it("不支持的键返回 null", () => {
    expect(keyName(ev("CapsLock"))).toBeNull();
    expect(keyName(ev("½"))).toBeNull();
  });
});

describe("isModifierKey", () => {
  it("认得四个修饰键", () => {
    for (const k of ["Control", "Alt", "Shift", "Meta"]) {
      expect(isModifierKey(ev(k)), k).toBe(true);
    }
  });

  it("普通键不算修饰键", () => {
    expect(isModifierKey(ev("q"))).toBe(false);
    expect(isModifierKey(ev("CapsLock"))).toBe(false);
  });
});

describe("splitCombo", () => {
  it("把修饰键与主键分开", () => {
    expect(splitCombo("Ctrl+Shift+Q")).toEqual({ modifiers: ["Ctrl", "Shift"], keys: ["Q"] });
    expect(splitCombo("Q+W")).toEqual({ modifiers: [], keys: ["Q", "W"] });
    expect(splitCombo("Ctrl+Shift")).toEqual({ modifiers: ["Ctrl", "Shift"], keys: [] });
  });

  it("空组合两段都为空", () => {
    expect(splitCombo("")).toEqual({ modifiers: [], keys: [] });
    expect(splitCombo(null)).toEqual({ modifiers: [], keys: [] });
  });
});

describe("requiresHook", () => {
  it("修饰键 + 单个主键 RegisterHotKey 就够用", () => {
    expect(requiresHook("Ctrl+Q")).toBe(false);
    expect(requiresHook("F5")).toBe(false);
    expect(requiresHook("Ctrl+Numpad0")).toBe(false);
    expect(requiresHook("Ctrl+OEM_1")).toBe(false);
  });

  it("纯修饰键与多主键只有钩子承载得了", () => {
    expect(requiresHook("Ctrl+Shift")).toBe(true);
    expect(requiresHook("Q+W")).toBe(true);
    expect(requiresHook("")).toBe(true);
  });
});

describe("isModifierOnly", () => {
  it("只有修饰键、没有主键时为真", () => {
    expect(isModifierOnly("Ctrl+Shift")).toBe(true);
    expect(isModifierOnly("Win")).toBe(true);
  });

  it("带主键或整体为空时为假", () => {
    expect(isModifierOnly("Ctrl+Q")).toBe(false);
    expect(isModifierOnly("Q+W")).toBe(false);
    expect(isModifierOnly("")).toBe(false);
  });
});

describe("joinCombo", () => {
  it("两段都在时用加号拼接", () => {
    expect(joinCombo("Ctrl+Shift", ["Q"])).toBe("Ctrl+Shift+Q");
    expect(joinCombo("Ctrl", ["Q", "W"])).toBe("Ctrl+Q+W");
  });

  it("缺主键时只剩修饰键，缺修饰键时只剩主键", () => {
    expect(joinCombo("Ctrl+Shift", [])).toBe("Ctrl+Shift");
    expect(joinCombo("", ["F5"])).toBe("F5");
  });

  it("两段都为空时是空串", () => {
    expect(joinCombo("", [])).toBe("");
    expect(joinCombo("", null)).toBe("");
  });
});
