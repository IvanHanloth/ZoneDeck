// 热键录制：把键盘事件转换为核心可解析的热键字符串。

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

/** 该键盘事件按的是不是修饰键本身。 */
export function isModifierKey(event) {
  return MODIFIER_KEYS.has(event.key);
}

/** 拼接修饰键与主键；两者都可为空。 */
export function joinCombo(modifiers, key) {
  if (!key) return modifiers || "";
  return modifiers ? `${modifiers}+${key}` : key;
}

/** 事件当下按住的修饰键组合；无修饰键返回空串。 */
export function modifiersFromEvent(event) {
  const parts = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Win");
  return parts.join("+");
}

/** 由键盘事件构造完整组合键字符串；未完成时返回 null。 */
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
