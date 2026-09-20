import {describe, expect, it} from "vitest";

import {LANGS, setLangPref} from "./i18n.svelte.js";
import {privacyUrl} from "./links.js";

describe("privacyUrl", () => {
    it("按界面语言取文档站上对应的那份", () => {
        setLangPref("zh-CN");
        expect(privacyUrl()).toBe("https://zonedeck.ivan-hanloth.cn/privacy");
        setLangPref("en");
        expect(privacyUrl()).toBe("https://zonedeck.ivan-hanloth.cn/en/privacy");
        setLangPref("zh-TW");
        expect(privacyUrl()).toBe("https://zonedeck.ivan-hanloth.cn/zh-tw/privacy");
    });

    // 漏登记一种语言时它会回落到简体那份，两种语言撞到同一个地址，据此发现。
    it("每种界面语言各有一份，没有漏登记的", () => {
        const urls = new Set(
            LANGS.map((lang) => {
                setLangPref(lang);
                return privacyUrl();
            }),
        );
        expect(urls.size).toBe(LANGS.length);
    });
});
