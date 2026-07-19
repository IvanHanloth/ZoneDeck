import { describe, expect, it } from "vitest";
import zhCN from "../locales/zh-CN.js";
import en from "../locales/en.js";
import zhTW from "../locales/zh-TW.js";
import { LANGS, LANG_NAMES, fromTag, normalizePref, resolve, t } from "./i18n.svelte.js";

describe("fromTag", () => {
  it("简体中文的各种写法都归为 zh-CN", () => {
    for (const tag of ["zh", "zh-CN", "zh_CN", "zh-Hans", "zh-Hans-CN", "ZH-cn", " zh-SG "]) {
      expect(fromTag(tag), tag).toBe("zh-CN");
    }
  });

  it("繁体中文的各种写法都归为 zh-TW", () => {
    for (const tag of ["zh-TW", "zh_TW", "zh-Hant", "zh-Hant-TW", "zh-HK", "zh-MO"]) {
      expect(fromTag(tag), tag).toBe("zh-TW");
    }
  });

  it("英文的各种写法都归为 en", () => {
    for (const tag of ["en", "en-US", "en_GB", "EN"]) {
      expect(fromTag(tag), tag).toBe("en");
    }
  });

  it("无对应翻译的语言返回 null", () => {
    for (const tag of ["ja", "ko", "fr-FR", "", "   ", null, undefined]) {
      expect(fromTag(tag), String(tag)).toBe(null);
    }
  });
});

describe("normalizePref", () => {
  it("auto 原样保留，合法标签归一为规范写法", () => {
    expect(normalizePref("auto")).toBe("auto");
    expect(normalizePref("AUTO")).toBe("auto");
    expect(normalizePref("zh_tw")).toBe("zh-TW");
    expect(normalizePref("en-US")).toBe("en");
  });

  it("非法值回落到 auto", () => {
    expect(normalizePref("ja-JP")).toBe("auto");
    expect(normalizePref("")).toBe("auto");
  });
});

describe("resolve", () => {
  it("显式偏好压过系统语言", () => {
    expect(resolve("en", "zh-CN")).toBe("en");
    expect(resolve("zh-TW", "en-US")).toBe("zh-TW");
  });

  it("auto 跟随系统语言", () => {
    expect(resolve("auto", "en-US")).toBe("en");
    expect(resolve("auto", "zh-Hant-TW")).toBe("zh-TW");
  });

  it("系统语言缺失或无翻译时回落到简体中文", () => {
    expect(resolve("auto", undefined)).toBe("zh-CN");
    expect(resolve("auto", "ja-JP")).toBe("zh-CN");
  });
});

// 漏译会静默回落到简体中文，界面变中英混排，故逐键校验三份 catalog 对齐。
describe("catalog 对齐", () => {
  const zhKeys = Object.keys(zhCN);

  it.each([
    ["en", en],
    ["zh-TW", zhTW],
  ])("%s 与 zh-CN 的键集完全一致", (_name, catalog) => {
    expect(Object.keys(catalog).sort()).toEqual(zhKeys.slice().sort());
  });

  it.each([
    ["zh-CN", zhCN],
    ["en", en],
    ["zh-TW", zhTW],
  ])("%s 没有空文案", (_name, catalog) => {
    const empty = Object.entries(catalog)
      .filter(([, v]) => typeof v !== "string" || !v.trim())
      .map(([k]) => k);
    expect(empty).toEqual([]);
  });

  it("占位符在各语言间一致", () => {
    const holders = (s) => (s.match(/\{(\w+)\}/g) ?? []).sort();
    for (const key of zhKeys) {
      expect(holders(en[key]), key).toEqual(holders(zhCN[key]));
      expect(holders(zhTW[key]), key).toEqual(holders(zhCN[key]));
    }
  });

  it("每种可选语言都有自称", () => {
    expect(Object.keys(LANG_NAMES).sort()).toEqual(LANGS.slice().sort());
  });
});

describe("t", () => {
  it("默认取简体中文", () => {
    expect(t("common.close")).toBe("关闭");
  });

  it("替换占位符", () => {
    expect(t("restore.frozen", { n: 3 })).toBe("已冻结 3 个进程");
  });

  it("参数缺失时占位符原样保留", () => {
    expect(t("restore.frozen", {})).toBe("已冻结 {n} 个进程");
  });

  it("未知键返回键本身，便于开发期发现漏译", () => {
    expect(t("nope.not.a.key")).toBe("nope.not.a.key");
  });
});
