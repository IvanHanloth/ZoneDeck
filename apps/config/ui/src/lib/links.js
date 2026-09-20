// 文档站上的隐私声明；三种语言各有一份，跟随界面语言。
// 地址写死不走 Verhub 项目信息：拉不到服务端时这条链接仍须可用。

import {openExternal} from "./verhub.js";
import {lang, t} from "./i18n.svelte.js";
import {toast} from "./state.svelte.js";

const PRIVACY_PAGES = {
    "zh-CN": "https://zonedeck.ivan-hanloth.cn/privacy",
    en: "https://zonedeck.ivan-hanloth.cn/en/privacy",
    "zh-TW": "https://zonedeck.ivan-hanloth.cn/zh-tw/privacy",
};

/** 当前界面语言对应的隐私声明地址。 */
export function privacyUrl() {
    return PRIVACY_PAGES[lang()] ?? PRIVACY_PAGES["zh-CN"];
}

/** 用系统默认浏览器打开隐私声明。 */
export async function openPrivacy() {
    try {
        await openExternal(privacyUrl());
    } catch (err) {
        toast(t("options.openLinkFailed", {err}), true);
    }
}
