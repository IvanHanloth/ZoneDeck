// 页签清单：导航栏与搜索结果共用一份，图标即页面的身份标识。
import IconAppWindow from "~icons/lucide/app-window";
import IconShieldCheck from "~icons/lucide/shield-check";
import IconKeyboard from "~icons/lucide/keyboard";
import IconEyeOff from "~icons/lucide/eye-off";
import IconZap from "~icons/lucide/zap";
import IconSettings from "~icons/lucide/settings";
import IconInfo from "~icons/lucide/info";

/** 全部页签，footer 项排在导航栏底部。 */
export const NAV = [
  { id: "binding", labelKey: "tab.binding", icon: IconAppWindow },
  { id: "whitelist", labelKey: "tab.whitelist", icon: IconShieldCheck },
  { id: "hotkeys", labelKey: "tab.hotkeys", icon: IconKeyboard },
  { id: "hide", labelKey: "tab.hide", icon: IconEyeOff },
  { id: "power", labelKey: "tab.power", icon: IconZap },
  { id: "about", labelKey: "tab.about", icon: IconInfo, footer: true },
  { id: "options", labelKey: "tab.options", icon: IconSettings, footer: true },
];

/** 页签定义，未知 id 回落到首个页签。 */
export function navItem(id) {
  return NAV.find((n) => n.id === id) ?? NAV[0];
}
