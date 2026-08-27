import { describe, expect, it } from "vitest";

import { setLangPref } from "./i18n.svelte.js";
import { downloadUrl, formatTime } from "./verhub.js";

describe("downloadUrl", () => {
  it("优先取 windows 平台的链接", () => {
    const version = {
      download_links: [
        { platform: "linux", url: "https://example.com/a.tar.gz" },
        { platform: "windows", url: "https://example.com/a.exe" },
      ],
    };
    expect(downloadUrl(version)).toBe("https://example.com/a.exe");
  });

  it("没有 windows 链接时取第一条", () => {
    const version = {
      download_links: [{ platform: "linux", url: "https://example.com/a.tar.gz" }],
    };
    expect(downloadUrl(version)).toBe("https://example.com/a.tar.gz");
  });

  it("没有 download_links 时回退 download_url", () => {
    expect(downloadUrl({ download_url: "https://example.com/a.exe" })).toBe(
      "https://example.com/a.exe",
    );
  });

  it("明文 http 链接一律不采用", () => {
    expect(downloadUrl({ download_links: [{ platform: "windows", url: "http://evil/a.exe" }] })).toBe(
      "",
    );
    expect(downloadUrl({ download_url: "http://evil/a.exe" })).toBe("");
  });

  it("http 的 windows 链接不会挡住可用的 https 链接", () => {
    const version = {
      download_links: [
        { platform: "windows", url: "http://evil/a.exe" },
        { platform: "linux", url: "https://example.com/a.tar.gz" },
      ],
    };
    expect(downloadUrl(version)).toBe("https://example.com/a.tar.gz");
  });

  it("非 http 协议同样被拒", () => {
    expect(downloadUrl({ download_url: "javascript:alert(1)" })).toBe("");
    expect(downloadUrl({ download_url: "file:///C:/evil.exe" })).toBe("");
  });

  it("缺参数或字段畸形时返回空串", () => {
    expect(downloadUrl(null)).toBe("");
    expect(downloadUrl(undefined)).toBe("");
    expect(downloadUrl({})).toBe("");
    expect(downloadUrl({ download_links: [{ platform: "windows" }] })).toBe("");
    expect(downloadUrl({ download_links: [null] })).toBe("");
  });
});

describe("formatTime", () => {
  const ts = Date.UTC(2026, 7, 26) / 1000;

  it("日期格式跟随界面语言", () => {
    setLangPref("zh-CN");
    const zh = formatTime(ts);
    setLangPref("en");
    const en = formatTime(ts);
    expect(zh).toContain("2026");
    expect(en).toContain("2026");
    expect(zh).not.toBe(en);
  });

  it("没有时间戳时返回空串", () => {
    expect(formatTime(0)).toBe("");
    expect(formatTime(null)).toBe("");
    expect(formatTime(undefined)).toBe("");
  });
});
