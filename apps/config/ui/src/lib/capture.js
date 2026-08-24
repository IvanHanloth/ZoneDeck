// 录制期独占键盘：优先走后端的低级键盘钩子，浏览器预览与装钩子失败时回落到 DOM 事件。
//
// 回落路径只能靠 preventDefault，拦不住其他进程的全局热键——这一点必须让用户知道，
// 故失败时会调用 onDegraded。

import { IN_TAURI, invoke, onAppEvent } from "./ipc.js";
import { isModifierKey, keyName, modifiersFromEvent } from "./hotkey.js";

/** DOM 事件 → 与后端 key-capture 一致的状态形状。 */
function stateFromEvent(event, down) {
  const modifiers = modifiersFromEvent(event);
  if (!down || isModifierKey(event)) {
    return { modifiers, key: null, down, unsupported: false };
  }
  const key = keyName(event);
  return { modifiers, key, down, unsupported: key === null };
}

function domFallback(onState) {
  // 用 stopImmediatePropagation：录制期间按键事件归录制器独占，
  // 否则 ContentDialog 挂在 window 上的 Esc 处理器会抢在前头把对话框关掉，
  // Win+Esc 这种合法组合就录不进去。
  const swallow = (e, down) => {
    e.preventDefault();
    e.stopImmediatePropagation();
    onState(stateFromEvent(e, down));
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

/**
 * 开始录制，返回停止函数（幂等）。
 *
 * @param onState 每次按下 / 抬起调用，参数为 `{ modifiers, key, down, unsupported }`。
 *   `key` 为 null 表示此刻只按着修饰键或主键已抬起。
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
  release = () => {
    offState();
    offLost();
  };

  invoke("start_key_capture").catch((err) => {
    if (stopped) return;
    // 钩子没装上就只剩 DOM 事件这一条路，功能降级但仍可录。
    release();
    release = domFallback(onState);
    onDegraded?.(err);
  });

  return stop;
}
