// 热键录制：把键盘事件转换为核心可解析的热键字符串。

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

/** 热键字符串里的修饰键片段。 */
export const MODIFIER_NAMES = new Set(["Ctrl", "Alt", "Shift", "Win"]);

/** 一条热键最多带几个主键；与核心的 hotkey::MAX_KEYS 对齐。 */
export const MAX_KEYS = 4;

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

// event.code 与键盘布局无关，恰好和 OEM 虚拟键码的位置命名对得上；
// 小键盘也只有 code 分得清（关掉 NumLock 时 key 会变成方向键）。
const CODE_MAP = {
  Semicolon: "OEM_1",
  Equal: "OEM_PLUS",
  Comma: "OEM_COMMA",
  Minus: "OEM_MINUS",
  Period: "OEM_PERIOD",
  Slash: "OEM_2",
  Backquote: "OEM_3",
  BracketLeft: "OEM_4",
  Backslash: "OEM_5",
  BracketRight: "OEM_6",
  Quote: "OEM_7",
  IntlBackslash: "OEM_102",
  ContextMenu: "Apps",
  NumpadMultiply: "NumpadMultiply",
  NumpadAdd: "NumpadAdd",
  NumpadSubtract: "NumpadSubtract",
  NumpadDecimal: "NumpadDecimal",
  NumpadDivide: "NumpadDivide",
};

/** 事件主键名；不支持的键返回 null。 */
export function keyName(event) {
  const code = event.code;
  if (typeof code === "string") {
    if (CODE_MAP[code]) return CODE_MAP[code];
    if (/^Numpad[0-9]$/.test(code)) return code;
  }
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

/** 拆开热键字符串；两段都可为空。 */
export function splitCombo(combo) {
  const parts = (combo || "")
    .split("+")
    .map((p) => p.trim())
    .filter(Boolean);
  return {
    modifiers: parts.filter((p) => MODIFIER_NAMES.has(p)),
    keys: parts.filter((p) => !MODIFIER_NAMES.has(p)),
  };
}

/**
 * 该组合是否只有低级键盘钩子能承载。
 * RegisterHotKey 只收「修饰键 + 单个主键」，纯修饰键与多主键都表达不了。
 */
export function requiresHook(combo) {
  return splitCombo(combo).keys.length !== 1;
}

/** 该组合是不是不带主键的纯修饰键热键。 */
export function isModifierOnly(combo) {
  const { modifiers, keys } = splitCombo(combo);
  return keys.length === 0 && modifiers.length > 0;
}

/** 拼接修饰键与主键；`keys` 可为字符串、数组或空。 */
export function joinCombo(modifiers, keys) {
  const list = Array.isArray(keys) ? keys : [keys];
  return [modifiers, ...list].filter(Boolean).join("+");
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
