// 设置搜索：从文案 catalog 推导可搜条目，匹配后交由界面跳转并高亮。
//
// 条目不手写登记，而是按文案键的命名约定推导，新增设置项即自动可搜：
//   `<前缀>.<名字>` 且存在同名的 `<名字>Desc`  → 一条设置项（标题 + 说明）
//   `<前缀>.<名字>Card`（或就叫 `card`）        → 一个分组小标题
//   `tab.<页签>`                               → 一个页面
// 前缀决定条目属于哪一页。键的归属与语言无关，故骨架取自简中 catalog；
// 三份 catalog 键集一致由 i18n 单测保证。

import zhCN from "../locales/zh-CN.js";
import { t } from "./i18n.svelte.js";

/** 文案键前缀 → 所在页签。未列出的前缀不参与搜索。 */
const PREFIX_TAB = {
  binding: "binding",
  windowRules: "binding",
  processRules: "binding",
  whitelist: "whitelist",
  hotkeys: "hotkeys",
  hide: "hide",
  power: "power",
  // 提示设置已并入通用设置页
  notify: "options",
  tray: "options",
  options: "options",
  about: "about",
};

/** 条目种类的排序权重，同分时页面排在分组前、分组排在设置项前。 */
const KIND_RANK = { page: 0, group: 1, setting: 2 };

function buildIndex() {
  const keys = Object.keys(zhCN);
  const has = new Set(keys);
  const entries = [];

  for (const key of keys) {
    const dot = key.indexOf(".");
    if (dot < 0) continue;
    const prefix = key.slice(0, dot);
    const name = key.slice(dot + 1);
    // 带点的子键（options.logLevel.debug 之类的选项值）不是条目
    if (name.includes(".")) continue;

    if (prefix === "tab") {
      entries.push({ kind: "page", tab: name, labelKey: key });
      continue;
    }

    const tab = PREFIX_TAB[prefix];
    if (!tab) continue;

    if (name === "card" || name.endsWith("Card")) {
      entries.push({ kind: "group", tab, labelKey: key });
    } else if (has.has(`${key}Desc`)) {
      entries.push({ kind: "setting", tab, labelKey: key, descKey: `${key}Desc` });
    }
  }

  return entries;
}

/** 全部可搜条目；键集在运行期不会变，只构建一次。 */
export const INDEX = buildIndex();

/** 条目当前语言下的显示文案。 */
export function entryText(entry) {
  return {
    label: t(entry.labelKey),
    desc: entry.descKey ? t(entry.descKey) : "",
    page: t(`tab.${entry.tab}`),
  };
}

// 打分：命中标题优于命中说明，从头匹配优于中间匹配。
function score(query, { label, desc, page }, kind) {
  const l = label.toLowerCase();
  const d = desc.toLowerCase();
  const p = page.toLowerCase();

  let base;
  if (l === query) base = 0;
  else if (l.startsWith(query)) base = 10;
  else if (l.includes(query)) base = 20;
  else if (p.includes(query)) base = 30;
  else if (d.includes(query)) base = 40;
  else return null;

  return base + KIND_RANK[kind];
}

/**
 * 按关键字检索设置项，返回按相关度排序的前 `limit` 条。
 * 每条形如 `{ kind, tab, label, desc, page }`，界面据此跳转与高亮。
 */
export function search(query, limit = 8) {
  const q = String(query ?? "").trim().toLowerCase();
  if (!q) return [];

  const hits = [];
  for (const entry of INDEX) {
    const text = entryText(entry);
    const s = score(q, text, entry.kind);
    if (s !== null) hits.push({ ...entry, ...text, score: s });
  }

  hits.sort((a, b) => a.score - b.score || a.label.localeCompare(b.label));
  return hits.slice(0, limit);
}
