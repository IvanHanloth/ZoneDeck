// 界面语言：catalog 查表 + 语言解析；与核心共用语言标签。

import zhCN from "../locales/zh-CN.js";
import en from "../locales/en.js";
import zhTW from "../locales/zh-TW.js";

/** 配置中表示「跟随系统」的语言偏好值。 */
export const LANG_AUTO = "auto";

/** 语言标签 → catalog；键顺序即展示顺序。 */
const CATALOGS = {
  "zh-CN": zhCN,
  en,
  "zh-TW": zhTW,
};

/** 可选语言标签（不含 auto）。 */
export const LANGS = Object.keys(CATALOGS);

/** 语言的自称，不随界面语言变化。 */
export const LANG_NAMES = {
  "zh-CN": "简体中文",
  en: "English",
  "zh-TW": "繁體中文",
};

/** 按 BCP-47 标签解析语言，无法归类时返回 null；中文按 script/region 区分正简。 */
export function fromTag(tag) {
  if (!tag) return null;
  const parts = String(tag).trim().replace(/_/g, "-").toLowerCase().split("-").filter(Boolean);
  const [primary, ...rest] = parts;
  if (primary === "en") return "en";
  if (primary === "zh") {
    return rest.some((p) => ["hant", "tw", "hk", "mo"].includes(p)) ? "zh-TW" : "zh-CN";
  }
  return null;
}

/** 归一化配置里的语言偏好：合法标签归一为规范写法，其余回落到 auto。 */
export function normalizePref(pref) {
  if (String(pref ?? "").trim().toLowerCase() === LANG_AUTO) return LANG_AUTO;
  return fromTag(pref) ?? LANG_AUTO;
}

/** 解析实际生效的语言；pref 为 auto 或非法值时依据 systemTag 推断。 */
export function resolve(pref, systemTag) {
  if (String(pref ?? "").trim().toLowerCase() !== LANG_AUTO) {
    const lang = fromTag(pref);
    if (lang) return lang;
  }
  return fromTag(systemTag) ?? "zh-CN";
}

const current = $state({ lang: "zh-CN" });

/** 按配置里的语言偏好设定当前语言。 */
export function setLangPref(pref) {
  current.lang = resolve(pref, globalThis.navigator?.language);
  // 字体回退与断行规则依赖根元素的 lang。
  if (typeof document !== "undefined") {
    document.documentElement.lang = current.lang;
  }
}

/** 当前生效语言标签。 */
export function lang() {
  return current.lang;
}

/**
 * 取当前语言下的文案；`params` 用于替换 `{名字}` 占位符。
 * 缺失的键回落到简体中文，仍缺失时返回键本身。
 */
export function t(key, params) {
  const text = CATALOGS[current.lang]?.[key] ?? CATALOGS["zh-CN"][key] ?? key;
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (m, name) =>
    Object.hasOwn(params, name) ? String(params[name]) : m,
  );
}
