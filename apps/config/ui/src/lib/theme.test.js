import { describe, expect, it } from "vitest";
import { nextTheme, resolveTheme, themeIcon } from "./theme.js";

describe("nextTheme", () => {
  it("按 auto → light → dark → auto 循环", () => {
    expect(nextTheme("auto")).toBe("light");
    expect(nextTheme("light")).toBe("dark");
    expect(nextTheme("dark")).toBe("auto");
  });

  it("未知值回到 auto", () => {
    expect(nextTheme("banana")).toBe("auto");
    expect(nextTheme(undefined)).toBe("auto");
  });
});

describe("resolveTheme", () => {
  it("显式偏好直接生效", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("auto 跟随系统配色", () => {
    expect(resolveTheme("auto", true)).toBe("dark");
    expect(resolveTheme("auto", false)).toBe("light");
  });
});

describe("themeIcon", () => {
  it("每种偏好有独立图标", () => {
    const icons = new Set(["auto", "light", "dark"].map(themeIcon));
    expect(icons.size).toBe(3);
  });
});
