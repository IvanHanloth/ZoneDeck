import { describe, expect, it } from "vitest";
import {
  CO2_KG_PER_KWH,
  ECO_WATTS,
  EMPTY_STATS,
  FREEZE_WATTS,
  TREE_KG_PER_YEAR,
  derive,
  formatBytes,
  formatCo2,
  formatCount,
  formatDuration,
  formatEnergy,
  formatSince,
  formatTreeDays,
} from "./powerstats.js";

/** 一小时冻结一个进程、一小时效率模式一个进程。 */
const oneHourEach = { ...EMPTY_STATS, freeze_seconds: 3600, efficiency_seconds: 3600 };

describe("derive", () => {
  it("按功率与时长算出电能，再折成碳排放与等效树木", () => {
    const { kwh, co2Kg, treeDays } = derive(oneHourEach);
    // 一小时 × 瓦数 = 瓦时，除以 1000 得度。
    expect(kwh).toBeCloseTo((FREEZE_WATTS + ECO_WATTS) / 1000, 10);
    expect(co2Kg).toBeCloseTo(kwh * CO2_KG_PER_KWH, 10);
    expect(treeDays).toBeCloseTo(co2Kg / (TREE_KG_PER_YEAR / 365), 10);
  });

  it("冻结的权重高于效率模式", () => {
    const frozen = derive({ ...EMPTY_STATS, freeze_seconds: 3600 });
    const eco = derive({ ...EMPTY_STATS, efficiency_seconds: 3600 });
    expect(frozen.kwh).toBeGreaterThan(eco.kwh);
  });

  it("全零输入得到全零，不出 NaN", () => {
    for (const stats of [EMPTY_STATS, null, undefined, {}]) {
      const d = derive(stats);
      expect(d.kwh).toBe(0);
      expect(d.co2Kg).toBe(0);
      expect(d.treeDays).toBe(0);
    }
  });

  it("脏数据（负数 / NaN / Infinity）按 0 计而非算出乱值", () => {
    const d = derive({
      freeze_seconds: -100,
      efficiency_seconds: Number.NaN,
      memory_freed_bytes: Number.POSITIVE_INFINITY,
    });
    expect(d.kwh).toBe(0);
    expect(Number.isFinite(d.co2Kg)).toBe(true);
  });
});

describe("formatBytes", () => {
  it("逐级进位到 1024", () => {
    expect(formatBytes(0)).toEqual({ value: "0", unit: "B" });
    expect(formatBytes(512)).toEqual({ value: "512", unit: "B" });
    expect(formatBytes(1024)).toEqual({ value: "1.0", unit: "KB" });
    expect(formatBytes(1024 ** 2)).toEqual({ value: "1.0", unit: "MB" });
    expect(formatBytes(1024 ** 3 * 46.2)).toEqual({ value: "46.2", unit: "GB" });
    expect(formatBytes(1024 ** 4 * 3)).toEqual({ value: "3.0", unit: "TB" });
  });

  it("字节整数不带小数，三位数以上不带小数", () => {
    expect(formatBytes(999)).toEqual({ value: "999", unit: "B" });
    expect(formatBytes(1024 * 512)).toEqual({ value: "512", unit: "KB" });
  });

  it("超出最大单位时不再进位", () => {
    expect(formatBytes(1024 ** 6).unit).toBe("PB");
  });
});

describe("数值格式化", () => {
  it("计数加千分位", () => {
    expect(formatCount(1284, "en-US")).toBe("1,284");
    expect(formatCount(0, "en-US")).toBe("0");
  });

  it("电能不足一度时换成瓦时", () => {
    expect(formatEnergy(4.3125)).toEqual({ value: "4.31", unit: "kWh" });
    expect(formatEnergy(1)).toEqual({ value: "1.00", unit: "kWh" });
    expect(formatEnergy(0.098)).toEqual({ value: "98", unit: "Wh" });
    expect(formatEnergy(0.0052)).toEqual({ value: "5.2", unit: "Wh" });
    expect(formatEnergy(0)).toEqual({ value: "0.0", unit: "Wh" });
  });

  it("碳排放不足一千克时换成克", () => {
    expect(formatCo2(2.44)).toEqual({ value: "2.4", unit: "kg" });
    expect(formatCo2(0.054)).toEqual({ value: "54", unit: "g" });
    expect(formatCo2(0.0031)).toEqual({ value: "3.1", unit: "g" });
  });

  it("等效树木极小时保住一位有效数字，不缩成 0.0", () => {
    expect(formatTreeDays(0)).toBe("0");
    expect(formatTreeDays(0.012)).toBe("0.01");
    expect(formatTreeDays(1.23)).toBe("1.2");
    expect(formatTreeDays(44.6)).toBe("45");
  });
});

describe("formatDuration", () => {
  const h = 3600;

  it("按量级挑单位", () => {
    expect(formatDuration(0)).toEqual({ value: "0", unitKey: "power.statsUnitSeconds" });
    expect(formatDuration(45)).toEqual({ value: "45", unitKey: "power.statsUnitSeconds" });
    expect(formatDuration(90)).toEqual({ value: "1.5", unitKey: "power.statsUnitMinutes" });
    expect(formatDuration(h * 2.5)).toEqual({ value: "2.5", unitKey: "power.statsUnitHours" });
    expect(formatDuration(h * 962, "en-US")).toEqual({
      value: "962",
      unitKey: "power.statsUnitHours",
    });
  });

  it("小时数超过三位就论天", () => {
    expect(formatDuration(h * 999, "en-US")).toEqual({
      value: "999",
      unitKey: "power.statsUnitHours",
    });
    expect(formatDuration(h * 1000, "en-US")).toEqual({
      value: "42",
      unitKey: "power.statsUnitDays",
    });
  });

  it("十以上取整加千分位，十以下保一位小数", () => {
    expect(formatDuration(60 * 9.44, "en-US").value).toBe("9.4");
    expect(formatDuration(h * 1234, "en-US")).toEqual({
      value: "51",
      unitKey: "power.statsUnitDays",
    });
  });

  it("脏数据按零处理", () => {
    for (const bad of [-1, Number.NaN, undefined, Number.POSITIVE_INFINITY]) {
      expect(formatDuration(bad).value).toBe("0");
    }
  });
});

describe("formatSince", () => {
  it("未开始统计时返回 null", () => {
    expect(formatSince(0)).toBe(null);
    expect(formatSince(undefined)).toBe(null);
  });

  it("Unix 秒按本地日期渲染", () => {
    // 2026-08-25T00:00:00Z
    expect(formatSince(1787616000, "en-US")).toMatch(/2026/);
  });
});
