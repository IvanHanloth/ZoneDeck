// 启动屏（index.html 里内联的 #splash）的收尾逻辑。
//
// 时序：HTML 一解析就画出启动屏 → 首帧之后 show 窗口（此前窗口是隐藏的，见 tauri.conf.json）
//      → 配置加载完 hideSplash() 淡出。窗口一直到有内容可画才出现，所以看不到白屏。

/** 等两帧：第一帧提交样式，第二帧确保启动屏真的画到了屏幕上。 */
export function afterFirstPaint(fn) {
  requestAnimationFrame(() => requestAnimationFrame(fn));
}

/** 淡出并移除启动屏；重复调用无副作用。 */
export function hideSplash() {
  const el = document.getElementById("splash");
  if (!el || el.classList.contains("gone")) return;
  el.classList.add("gone");
  el.addEventListener("transitionend", () => el.remove(), { once: true });
  // transition 被 prefers-reduced-motion 关掉时不会有 transitionend，兜一个定时器。
  setTimeout(() => el.remove(), 400);
}
