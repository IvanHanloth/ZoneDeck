// 启动屏的收尾逻辑。

/** 在首帧绘制之后执行回调。 */
export function afterFirstPaint(fn) {
  requestAnimationFrame(() => requestAnimationFrame(fn));
}

/** 淡出并移除启动屏；重复调用无副作用。 */
export function hideSplash() {
  const el = document.getElementById("splash");
  if (!el || el.classList.contains("gone")) return;
  el.classList.add("gone");
  el.addEventListener("transitionend", () => el.remove(), { once: true });
  setTimeout(() => el.remove(), 400);
}
