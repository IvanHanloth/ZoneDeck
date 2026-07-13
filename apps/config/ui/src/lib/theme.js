// 三态主题：auto（跟随系统）→ light → dark 循环。
// 纯逻辑（cycle/resolve）与 DOM 应用分离，便于单元测试。

export const THEMES = ["auto", "light", "dark"];
const STORAGE_KEY = "bosskey-theme";

/** 循环切换主题偏好。未知值回到 auto。 */
export function nextTheme(current) {
  const i = THEMES.indexOf(current);
  return THEMES[(i + 1) % THEMES.length] ?? "auto";
}

/** 把偏好解析为实际配色：auto 时跟随系统。 */
export function resolveTheme(preference, systemDark) {
  if (preference === "light" || preference === "dark") return preference;
  return systemDark ? "dark" : "light";
}

/** 主题图标（标题栏按钮显示）。 */
export function themeIcon(preference) {
  return { auto: "◐", light: "☀", dark: "🌙" }[preference] ?? "◐";
}

export function themeLabel(preference) {
  return (
    { auto: "主题：跟随系统", light: "主题：浅色", dark: "主题：深色" }[
      preference
    ] ?? "主题"
  );
}

// ---- DOM 侧（浏览器环境） ----

export function loadPreference() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return THEMES.includes(saved) ? saved : "auto";
  } catch {
    return "auto";
  }
}

export function savePreference(preference) {
  try {
    localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    /* 隐私模式等场景下忽略 */
  }
}

/** 将解析后的主题写到根元素，CSS 据此切换变量。 */
export function applyTheme(preference) {
  const systemDark =
    typeof matchMedia !== "undefined" &&
    matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.dataset.theme = resolveTheme(preference, systemDark);
}
