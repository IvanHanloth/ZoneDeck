// 热键录制：把键盘事件转换为核心可解析的热键字符串（如 "Ctrl+Alt+Q"）。
// 纯函数，便于单元测试。

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

const KEY_MAP = {
  Escape: "Esc",
  " ": "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

/** 事件主键名；不支持的键返回 null。 */
export function keyName(event) {
  const k = event.key;
  if (typeof k !== "string") return null;
  if (k.length === 1) {
    const up = k.toUpperCase();
    if (/[A-Z0-9]/.test(up)) return up;
  }
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(k)) return k;
  return KEY_MAP[k] || null;
}

/**
 * 由键盘事件构造完整组合键字符串。
 * 仅按下修饰键、或主键不受支持时返回 null（继续等待）。
 */
export function comboFromEvent(event) {
  if (MODIFIER_KEYS.has(event.key)) return null;
  const main = keyName(event);
  if (!main) return null;

  const parts = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Win");
  parts.push(main);
  return parts.join("+");
}
