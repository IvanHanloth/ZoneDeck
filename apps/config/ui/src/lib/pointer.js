// 鼠标 / 屏幕四角图示的元数据。

import { t } from "./i18n.svelte.js";

// 五颗键，对应 setting.mouse 的同名字段。
export const MOUSE_PARTS = [
  { key: "left", labelKey: "mouse.left" },
  { key: "middle", labelKey: "mouse.middle" },
  { key: "right", labelKey: "mouse.right" },
  { key: "side1", labelKey: "mouse.side1" },
  { key: "side2", labelKey: "mouse.side2" },
];

export const MAX_CLICKS = 3;
export const MIN_MULTI_CLICK_MS = 150;
export const MAX_MULTI_CLICK_MS = 1000;

/** 把一颗键的触发条件格式化为可读文本，如「Ctrl + 三击」。 */
export function describeTrigger(button) {
  if (!button?.enabled) return t("mouse.notEnabled");
  const clicks = t(
    ["mouse.singleClick", "mouse.doubleClick", "mouse.tripleClick"][
      Math.min(Math.max(button.clicks, 1), MAX_CLICKS) - 1
    ],
  );
  return button.modifiers ? `${button.modifiers} + ${clicks}` : clicks;
}

// cursor：该角在四角图示 viewBox(0 0 320 210) 中的光标停靠点。
export const CORNERS = [
  { key: "top_left_hide", labelKey: "corner.topLeft", cursor: [26, 26] },
  { key: "top_right_hide", labelKey: "corner.topRight", cursor: [294, 26] },
  { key: "bottom_left_hide", labelKey: "corner.bottomLeft", cursor: [26, 158] },
  { key: "bottom_right_hide", labelKey: "corner.bottomRight", cursor: [294, 158] },
];

export const CORNER_CENTER = [160, 92];

/** 取出当前已启用的部件（保持 items 的原始顺序）。 */
export function enabledParts(items, setting) {
  return items.filter((it) => Boolean(setting?.[it.key]));
}

/** 构造四角演示的时间轴帧序列。 */
export function buildTimeline(corners, allowRestore, fastOnly = true) {
  const frames = [];
  const reachKey = fastOnly ? "corner.reachFast" : "corner.reachNormal";
  const again = t(fastOnly ? "corner.againFast" : "corner.againNormal");

  for (const c of corners) {
    frames.push({
      corner: c.key,
      cursor: c.cursor,
      visible: true,
      fast: fastOnly,
      caption: t(reachKey, { corner: t(c.labelKey) }),
      ms: 1100,
    });
    frames.push({
      corner: c.key,
      cursor: c.cursor,
      visible: false,
      caption: t("corner.windowHidden"),
      ms: 1000,
    });
    if (allowRestore) {
      frames.push({
        corner: c.key,
        cursor: CORNER_CENTER,
        visible: false,
        caption: t("corner.cursorLeft"),
        ms: 800,
      });
      frames.push({
        corner: c.key,
        cursor: c.cursor,
        visible: false,
        fast: fastOnly,
        caption: again,
        ms: 1100,
      });
      frames.push({
        corner: c.key,
        cursor: c.cursor,
        visible: true,
        caption: t("corner.windowRestored"),
        ms: 1000,
      });
    } else {
      frames.push({
        corner: c.key,
        cursor: CORNER_CENTER,
        visible: true,
        caption: t("corner.restoreByHotkey"),
        ms: 1100,
      });
    }
  }
  return frames;
}
