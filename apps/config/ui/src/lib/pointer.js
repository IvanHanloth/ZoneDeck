// 鼠标 / 屏幕四角图示的元数据：把「配置字段 ↔ 图形部件」的对应关系集中在这里，
// 组件只负责画图，选中状态一律由 setting 里的布尔字段驱动。

// 五颗键对应 setting.mouse 里的同名字段，每颗键各有 { enabled, clicks, modifiers }。
export const MOUSE_PARTS = [
  { key: "left", label: "左键" },
  { key: "middle", label: "中键（滚轮）" },
  { key: "right", label: "右键" },
  { key: "side1", label: "侧键 1（前进键）" },
  { key: "side2", label: "侧键 2（后退键）" },
];

export const MAX_CLICKS = 3;
export const MIN_MULTI_CLICK_MS = 150;
export const MAX_MULTI_CLICK_MS = 1000;

/** 把一颗键的触发条件写成人话，如「Ctrl + 三击」。 */
export function describeTrigger(button) {
  if (!button?.enabled) return "未启用";
  const clicks = ["单击", "双击", "三击"][Math.min(Math.max(button.clicks, 1), MAX_CLICKS) - 1];
  return button.modifiers ? `${button.modifiers} + ${clicks}` : clicks;
}

// cursor 为该角落在四角图示 viewBox(0 0 320 210) 中的光标停靠点。
export const CORNERS = [
  { key: "top_left_hide", label: "左上角", cursor: [26, 26] },
  { key: "top_right_hide", label: "右上角", cursor: [294, 26] },
  { key: "bottom_left_hide", label: "左下角", cursor: [26, 158] },
  { key: "bottom_right_hide", label: "右下角", cursor: [294, 158] },
];

export const CORNER_CENTER = [160, 92];

/** 取出当前已启用的部件（保持 items 的原始顺序）。 */
export function enabledParts(items, setting) {
  return items.filter((it) => Boolean(setting?.[it.key]));
}

/** 环形推进下标；列表为空时恒为 0。 */
export function nextIndex(length, index) {
  if (length <= 0) return 0;
  return (index + 1) % length;
}

/** 光标该停在哪儿：有选中的角就轮流停靠，否则停在屏幕中央。 */
export function cursorTarget(corners, index) {
  if (corners.length === 0) return CORNER_CENTER;
  return corners[index % corners.length].cursor;
}

/**
 * 四角演示的时间轴：按 CORNERS 的顺序（左上→右上→左下→右下）逐个角落演示，
 * 每个角先演一遍「移动过去 → 窗口隐藏」，再按 allowRestore 演恢复的方式。
 * 每帧描述某一瞬间的画面：光标停在哪、骨架屏窗口是否可见、往哪个角收起、配文是什么。
 *
 * fastOnly 对应核心的「仅快速移动才触发」：为真时冲向角落的那两帧会标 fast，
 * 界面据此把光标的位移动画改成一记快甩，配文也说「快速甩到」。
 */
export function buildTimeline(corners, allowRestore, fastOnly = true) {
  const frames = [];
  const reach = fastOnly ? "快速移动到" : "把鼠标移动到";
  const again = fastOnly ? "再快速移动一次" : "再移动一次";

  for (const c of corners) {
    frames.push({
      corner: c.key,
      cursor: c.cursor,
      visible: true,
      fast: fastOnly,
      caption: `${reach}${c.label}`,
      ms: 1100,
    });
    frames.push({
      corner: c.key,
      cursor: c.cursor,
      visible: false,
      caption: "窗口已隐藏",
      ms: 1000,
    });
    if (allowRestore) {
      frames.push({
        corner: c.key,
        cursor: CORNER_CENTER,
        visible: false,
        caption: "鼠标离开角落",
        ms: 800,
      });
      frames.push({
        corner: c.key,
        cursor: c.cursor,
        visible: false,
        fast: fastOnly,
        caption: `${again}`,
        ms: 1100,
      });
      frames.push({
        corner: c.key,
        cursor: c.cursor,
        visible: true,
        caption: "窗口已恢复",
        ms: 1000,
      });
    } else {
      frames.push({
        corner: c.key,
        cursor: CORNER_CENTER,
        visible: true,
        caption: "用热键或托盘菜单恢复窗口",
        ms: 1100,
      });
    }
  }
  return frames;
}
