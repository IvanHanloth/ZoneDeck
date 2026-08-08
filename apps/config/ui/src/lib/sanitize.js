// 保存前的配置数字字段修正。
//
// `type="number"` 输入框被清空时，Svelte 的 `bind:value` 会得到 `null`；
// 后端对应字段为 u32，收到 `null` 会整单拒绝保存。故写盘前统一回落默认值并钳制范围。

import { MAX_CLICKS, MAX_MULTI_CLICK_MS, MIN_MULTI_CLICK_MS } from "./pointer.js";

export const DEFAULT_MULTI_CLICK_MS = 350;
export const MIN_AUTO_HIDE_TIME = 1;
export const MAX_AUTO_HIDE_TIME = 120;
const DEFAULT_LOG_RETENTION_DAYS = 7;
const MAX_LOG_RETENTION_DAYS = 365;

export const DEFAULT_AUTO_HIDE_TIME = 5;

/** 取整并钳到 [min, max]；非有限数字（null / NaN / 空串）回落 fallback。 */
export function clampInt(value, min, max, fallback) {
  const n =
    typeof value === "number" && Number.isFinite(value) ? Math.round(value) : fallback;
  return Math.min(max, Math.max(min, n));
}

const MOUSE_KEYS = ["left", "middle", "right", "side1", "side2"];

/** 就地修正 config 中的数字字段，返回同一对象。 */
export function sanitizeConfig(config) {
  const s = config?.setting;
  if (!s) return config;
  s.auto_hide_time = clampInt(
    s.auto_hide_time,
    MIN_AUTO_HIDE_TIME,
    MAX_AUTO_HIDE_TIME,
    DEFAULT_AUTO_HIDE_TIME,
  );
  s.log_retention_days = clampInt(
    s.log_retention_days,
    0,
    MAX_LOG_RETENTION_DAYS,
    DEFAULT_LOG_RETENTION_DAYS,
  );
  const m = s.mouse;
  if (m) {
    m.multi_click_ms = clampInt(
      m.multi_click_ms,
      MIN_MULTI_CLICK_MS,
      MAX_MULTI_CLICK_MS,
      DEFAULT_MULTI_CLICK_MS,
    );
    for (const key of MOUSE_KEYS) {
      const b = m[key];
      if (b) b.clicks = clampInt(b.clicks, 1, MAX_CLICKS, 1);
    }
  }
  return config;
}
