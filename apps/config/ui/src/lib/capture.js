// 录制期独占键盘：优先走后端的低级键盘钩子，浏览器预览与装钩子失败时回落到 DOM 事件。
//
// 回落路径只能靠 preventDefault，拦不住其他进程的全局热键，故失败时会调用 onDegraded。

import { IN_TAURI, invoke, onAppEvent } from "./ipc.js";
import { isModifierKey, keyName, MAX_KEYS, modifiersFromEvent } from "./hotkey.js";

/** DOM 事件 → 与后端 key-capture 一致的状态形状。 */
function stateFromEvent(event, down, held) {
  const modifiers = modifiersFromEvent(event);
  if (isModifierKey(event)) {
    return { modifiers, keys: [...held], down, unsupported: false };
  }
  const key = keyName(event);
  // 超出热键容量的主键不再收，与后端 HeldKeys 对齐。
  if (key && down) {
    if (held.size < MAX_KEYS) held.add(key);
  } else if (key) {
    held.delete(key);
  }
  return { modifiers, keys: [...held], down, unsupported: down && key === null };
}

function domFallback(onState) {
  // 用 stopImmediatePropagation 让录制期间的按键事件归录制器独占，
  // 不被 ContentDialog 挂在 window 上的 Esc 处理器抢走。
  const held = new Set();
  const swallow = (e, down) => {
    e.preventDefault();
    e.stopImmediatePropagation();
    onState(stateFromEvent(e, down, held));
  };
  const onKeydown = (e) => swallow(e, true);
  const onKeyup = (e) => swallow(e, false);
  window.addEventListener("keydown", onKeydown, true);
  window.addEventListener("keyup", onKeyup, true);
  return () => {
    window.removeEventListener("keydown", onKeydown, true);
    window.removeEventListener("keyup", onKeyup, true);
  };
}

/** 录制状态是不是裸 Esc；键盘被独占时它是唯一的键盘退路。 */
export function isBareEscape(state) {
  return !!state.down && !state.modifiers && state.keys?.length === 1 && state.keys[0] === "Esc";
}

/**
 * 开始录制，返回停止函数（幂等）。
 *
 * @param onState 每次按下 / 抬起调用，参数为 `{ modifiers, keys, down, unsupported }`。
 *   `keys` 是此刻按住的主键，只按着修饰键时为空数组。
 * @param onLost 录制被后端中断（窗口失焦）时调用。
 * @param onDegraded 没能独占键盘、已回落到 DOM 事件时调用，参数为原因。
 */
export function startCapture({ onState, onLost, onDegraded } = {}) {
  let stopped = false;
  let release = () => {};

  const stop = () => {
    if (stopped) return;
    stopped = true;
    release();
    release = () => {};
    if (IN_TAURI) invoke("stop_key_capture").catch(() => {});
  };

  if (!IN_TAURI) {
    release = domFallback(onState);
    return stop;
  }

  // 先订阅再开录，避免开录到订阅生效之间漏掉按键。
  const offState = onAppEvent("key-capture", (e) => onState?.(e.payload));
  const offLost = onAppEvent("key-capture-stopped", () => onLost?.());

  // 钩子装上后会把按键吞掉，DOM 这边一个都收不到；反过来，这里真收到了按键
  // 就等于钩子被挡住了（安全软件的防键盘记录会拦掉别的进程的低级键盘钩子）。
  // 两条路天然互斥，故 DOM 监听常驻兜底，钩子失效时录制照常能用，只是不独占。
  let hooked = false;
  let warned = false;
  const offDom = domFallback((state) => {
    if (hooked && !warned) {
      warned = true;
      onDegraded?.(new Error("keyboard hook is installed but receives nothing"));
    }
    onState?.(state);
  });
  release = () => {
    offState();
    offLost();
    offDom();
  };

  invoke("start_key_capture").then(
    () => {
      hooked = true;
    },
    (err) => {
      // 钩子没装上就只剩 DOM 事件这一条路，功能降级但仍可录。
      if (stopped || warned) return;
      warned = true;
      onDegraded?.(err);
    },
  );

  return stop;
}
