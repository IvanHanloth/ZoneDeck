// 能效统计的换算与格式化。
//
// 后端只存原始累计量（次数、进程·秒、字节），电能与碳排放在这里估算。
// 系数集中在文件顶部，调整口径不必动 Rust。

/**
 * 每个被冻结进程平均省下的功率（瓦，墙上口径）。
 * CPU 侧估 0.5 W，÷0.87 折到墙上（80 Plus 金牌在 20% 负载下的认证阈值），
 * 再 ×1.5 计入 GPU、内存与 I/O 的连带停摆。
 */
export const FREEZE_WATTS = 0.9;

/** 效率模式省下的功率（瓦）。取冻结的四成：进程仍在跑，只是降频、改吃能效核心。 */
export const ECO_WATTS = 0.35;

/** 电网碳排放因子（kg CO₂/kWh）。官方 2023 年全国电力平均值为 0.5306，此处取 0.55。 */
export const CO2_KG_PER_KWH = 0.55;

/** 一棵成年树每年吸收的 CO₂（kg）。常引用区间 20~25，取下沿。 */
export const TREE_KG_PER_YEAR = 20;

/** 全零统计，读不到文件时用它，避免界面各处判空。 */
export const EMPTY_STATS = {
  since: 0,
  updated_at: 0,
  freeze_count: 0,
  efficiency_count: 0,
  freeze_seconds: 0,
  efficiency_seconds: 0,
  memory_freed_bytes: 0,
};

/** 非有限值（NaN / Infinity / 缺字段）一律按 0 计。 */
function num(value) {
  return Number.isFinite(value) && value > 0 ? value : 0;
}

/**
 * 由原始累计量推出估算指标。
 * @returns {{kwh: number, co2Kg: number, treeDays: number}}
 *   kwh 节省的电能，co2Kg 减少的碳排放，treeDays 相当于多少「棵树·天」的吸收量。
 */
export function derive(stats) {
  const s = stats ?? EMPTY_STATS;
  const wattSeconds =
    num(s.freeze_seconds) * FREEZE_WATTS + num(s.efficiency_seconds) * ECO_WATTS;
  // 瓦·秒 → 千瓦时：先除 3600 得瓦时，再除 1000。
  const kwh = wattSeconds / 3_600_000;
  const co2Kg = kwh * CO2_KG_PER_KWH;
  // 日常量级下「棵树·年」小到看不见，换成「棵树·天」才有感。
  return { kwh, co2Kg, treeDays: co2Kg / (TREE_KG_PER_YEAR / 365) };
}

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

/**
 * 字节数逐级进位到 1024。
 * @returns {{value: string, unit: string}} 数值与单位分开给，栅格里数字排大、单位排小。
 */
export function formatBytes(bytes) {
  let value = num(bytes);
  let unit = 0;
  while (value >= 1024 && unit < BYTE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // 已是最小单位时不给小数；上了 KB 才有必要保留一位。
  const digits = unit === 0 ? 0 : value >= 100 ? 0 : 1;
  return { value: value.toFixed(digits), unit: BYTE_UNITS[unit] };
}

/** 计数按本地习惯加千分位；单位（「次」）随界面语言，由调用方补。 */
export function formatCount(count, locale) {
  return num(count).toLocaleString(locale);
}

/** 电能：不足一度改用瓦时，否则读起来全是 0.0x。 */
export function formatEnergy(kwh) {
  const value = num(kwh);
  if (value >= 1) return { value: value.toFixed(2), unit: "kWh" };
  const wh = value * 1000;
  return { value: wh >= 10 ? String(Math.round(wh)) : wh.toFixed(1), unit: "Wh" };
}

/** 碳排放：不足一千克改用克，同上。 */
export function formatCo2(kg) {
  const value = num(kg);
  if (value >= 1) return { value: value.toFixed(1), unit: "kg" };
  const grams = value * 1000;
  return { value: grams >= 10 ? String(Math.round(grams)) : grams.toFixed(1), unit: "g" };
}

/** 数值统一的取位规则：不足 10 保一位小数，否则取整加千分位。 */
function scaled(value, locale) {
  return value < 10 ? value.toFixed(1) : Math.round(value).toLocaleString(locale);
}

/** 小时数的上限；再多就该论天了，免得数字长到四位。 */
const HOURS_CAP = 999;

/**
 * 「进程·秒」转成合适的时间单位。
 * @returns {{value: string, unitKey: string}} 单位给的是文案键，由调用方按界面语言翻译。
 */
export function formatDuration(seconds, locale) {
  const total = num(seconds);
  if (total < 60) {
    return { value: String(Math.round(total)), unitKey: "power.statsUnitSeconds" };
  }
  const minutes = total / 60;
  if (minutes < 60) {
    return { value: scaled(minutes, locale), unitKey: "power.statsUnitMinutes" };
  }
  const hours = minutes / 60;
  if (hours <= HOURS_CAP) {
    return { value: scaled(hours, locale), unitKey: "power.statsUnitHours" };
  }
  return { value: scaled(hours / 24, locale), unitKey: "power.statsUnitDays" };
}

/** 等效树木：小数值保一位有效数字，免得缩成 0.0；上了两位数就取整。 */
export function formatTreeDays(days) {
  const value = num(days);
  if (value === 0) return "0";
  if (value < 1) return String(Number(value.toPrecision(1)));
  if (value < 10) return value.toFixed(1);
  return Math.round(value).toLocaleString();
}

/** 统计起始时刻（Unix 秒）转本地日期；未开始统计时返回 null。 */
export function formatSince(since, locale) {
  const seconds = num(since);
  if (!seconds) return null;
  return new Date(seconds * 1000).toLocaleDateString(locale);
}
