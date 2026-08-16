import { describe, expect, it } from "vitest";

import {
  clampInt,
  DEFAULT_AUTO_HIDE_TIME,
  DEFAULT_MULTI_CLICK_MS,
  MAX_AUTO_HIDE_TIME,
  MIN_AUTO_HIDE_TIME,
  sanitizeConfig,
} from "./sanitize.js";

describe("clampInt", () => {
  it("清空输入产生的 null 回落默认值", () => {
    expect(clampInt(null, 1, 120, 5)).toBe(5);
  });

  it("NaN 与非数字回落默认值", () => {
    expect(clampInt(NaN, 1, 120, 5)).toBe(5);
    expect(clampInt("", 1, 120, 5)).toBe(5);
    expect(clampInt(undefined, 1, 120, 5)).toBe(5);
  });

  it("越界值钳到边界", () => {
    expect(clampInt(0, 1, 120, 5)).toBe(1);
    expect(clampInt(999, 1, 120, 5)).toBe(120);
  });

  it("小数取整，合法值原样保留", () => {
    expect(clampInt(2.6, 1, 120, 5)).toBe(3);
    expect(clampInt(15, 1, 120, 5)).toBe(15);
  });
});

describe("sanitizeConfig", () => {
  const config = (setting) => ({ setting });

  it("数字输入框清空后（null）保存不再失败：回落默认值", () => {
    const c = config({
      auto_hide_time: null,
      log_retention_days: null,
      mouse: {
        multi_click_ms: null,
        left: { enabled: true, clicks: null, modifiers: "" },
      },
    });
    sanitizeConfig(c);
    expect(c.setting.auto_hide_time).toBe(DEFAULT_AUTO_HIDE_TIME);
    expect(c.setting.log_retention_days).toBe(7);
    expect(c.setting.mouse.multi_click_ms).toBe(DEFAULT_MULTI_CLICK_MS);
    expect(c.setting.mouse.left.clicks).toBe(1);
    expect(c.setting.mouse.left.enabled).toBe(true);
  });

  it("越界数字被钳到合法范围", () => {
    const c = config({
      auto_hide_time: 99999,
      mouse: { multi_click_ms: 20, middle: { enabled: true, clicks: 9 } },
    });
    sanitizeConfig(c);
    expect(c.setting.auto_hide_time).toBe(MAX_AUTO_HIDE_TIME);
    expect(c.setting.mouse.multi_click_ms).toBe(150);
    expect(c.setting.mouse.middle.clicks).toBe(3);
  });

  it("合法配置原样保留", () => {
    const c = config({
      auto_hide_time: 15,
      log_retention_days: 0,
      mouse: { multi_click_ms: 500, right: { enabled: false, clicks: 2 } },
    });
    sanitizeConfig(c);
    expect(c.setting.auto_hide_time).toBe(15);
    expect(c.setting.log_retention_days).toBe(0);
    expect(c.setting.mouse.multi_click_ms).toBe(500);
    expect(c.setting.mouse.right.clicks).toBe(2);
  });

  it("config 或 setting 缺失时不抛错", () => {
    expect(() => sanitizeConfig(null)).not.toThrow();
    expect(() => sanitizeConfig({})).not.toThrow();
    expect(() => sanitizeConfig(config({ mouse: undefined }))).not.toThrow();
  });

  it("返回传入的同一对象，便于链式使用", () => {
    const c = config({ auto_hide_time: 5 });
    expect(sanitizeConfig(c)).toBe(c);
  });

  it("边界常量与输入框一致", () => {
    expect(MIN_AUTO_HIDE_TIME).toBe(1);
    expect(MAX_AUTO_HIDE_TIME).toBe(120);
  });
});
