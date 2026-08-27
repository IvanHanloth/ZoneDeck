// 三态主题：auto（跟随系统）→ light → dark 循环。

import { t } from "./i18n.svelte.js";

export const THEMES = ["auto", "light", "dark"];
const STORAGE_KEY = "zonedeck-theme";

/** 循环切换主题偏好。未知值回到 auto。 */
export function nextTheme(current) {
  const i = THEMES.indexOf(current);
  return THEMES[(i + 1) % THEMES.length] ?? "auto";
}

/** 把偏好解析为实际配色；auto 时跟随系统。 */
export function resolveTheme(preference, systemDark) {
  if (preference === "light" || preference === "dark") return preference;
  return systemDark ? "dark" : "light";
}

/** 主题图标名（lucide），由界面映射到具体组件。 */
export function themeIcon(preference) {
  return { auto: "contrast", light: "sun", dark: "moon" }[preference] ?? "contrast";
}

export function themeLabel(preference) {
  const key = { auto: "theme.auto", light: "theme.light", dark: "theme.dark" }[preference];
  return key ? t(key) : t("theme.fallback");
}

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
    /* 忽略 */
  }
}

/** 将解析后的主题写到根元素，CSS 据此切换变量。 */
export function applyTheme(preference) {
  const systemDark =
    typeof matchMedia !== "undefined" &&
    matchMedia("(prefers-color-scheme: dark)").matches;
  const theme = resolveTheme(preference, systemDark);
  document.documentElement.dataset.theme = theme;
  syncWindowTheme(theme);
}

/**
 * 把主题同步给 DWM。Mica 的底色由系统主题决定，不跟应用内的偏好走，
 * 不同步的话「系统亮 + 应用暗」会出现亮底 Mica 配暗色卡片。
 * 走动态 import：非 Tauri 环境（浏览器预览、单测）不加载 Tauri API。
 */
function syncWindowTheme(theme) {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) return;
  import("@tauri-apps/api/window")
    .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(theme))
    .catch(() => {
      /* 权限未放行或窗口已销毁时忽略 */
    });
}
