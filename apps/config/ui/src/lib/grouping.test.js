import { describe, expect, it } from "vitest";
import {
  addProcessRules,
  addWindowRules,
  applyListFilters,
  containsPattern,
  escapeRegex,
  filterWindows,
  groupByProcess,
  iconPathsToFetch,
  NO_TITLE,
  newProcessRegexRule,
  newWindowRegexRule,
  processRuleFromWindow,
  splitByVisibility,
  traceWindowRule,
  windowRuleFromWindow,
} from "./grouping.js";

const win = (title, hwnd, process, path = "C:\\" + process, visible = true) => ({
  title,
  hwnd,
  process,
  PID: hwnd,
  path,
  visible,
});

describe("groupByProcess", () => {
  it("按进程聚合并携带路径", () => {
    const groups = groupByProcess([
      win("微信", 1, "WeChat.exe"),
      win("传输助手", 2, "WeChat.exe"),
      win("记事本", 3, "notepad.exe"),
    ]);
    expect(groups.map((g) => g.process)).toEqual(["notepad.exe", "WeChat.exe"]);
    expect(groups[1].windows).toHaveLength(2);
    expect(groups[0].path).toBe("C:\\notepad.exe");
  });

  it("空进程名归入未知分组", () => {
    const groups = groupByProcess([win("孤儿窗口", 9, "")]);
    expect(groups[0].process).toBe("（未知进程）");
  });
});

describe("filterWindows", () => {
  const windows = [
    win("微信", 1, "WeChat.exe"),
    win("记事本", 2, "notepad.exe"),
    win("Visual Studio Code", 3, "Code.exe"),
  ];

  it("空查询返回全部", () => {
    expect(filterWindows(windows, "")).toHaveLength(3);
    expect(filterWindows(windows, "   ")).toHaveLength(3);
  });

  it("按标题模糊匹配、不区分大小写", () => {
    expect(filterWindows(windows, "记事").map((w) => w.hwnd)).toEqual([2]);
    expect(filterWindows(windows, "code").map((w) => w.hwnd)).toEqual([3]);
  });

  it("按进程名匹配", () => {
    expect(filterWindows(windows, "wechat.exe").map((w) => w.hwnd)).toEqual([1]);
  });

  it("无匹配返回空", () => {
    expect(filterWindows(windows, "钉钉")).toEqual([]);
  });
});

describe("splitByVisibility", () => {
  it("按 visible 拆成可见/后台两组", () => {
    const list = [
      win("前台", 1, "a.exe", "C:\\a.exe", true),
      win("后台", 2, "b.exe", "C:\\b.exe", false),
      win("前台2", 3, "c.exe", "C:\\c.exe", true),
    ];
    const { visible, hidden } = splitByVisibility(list);
    expect(visible.map((w) => w.hwnd)).toEqual([1, 3]);
    expect(hidden.map((w) => w.hwnd)).toEqual([2]);
  });

  it("visible 缺省视为可见（兼容旧数据）", () => {
    const { visible, hidden } = splitByVisibility([
      { title: "x", hwnd: 1, process: "p.exe", PID: 1, path: "" },
    ]);
    expect(visible).toHaveLength(1);
    expect(hidden).toHaveLength(0);
  });
});

describe("applyListFilters", () => {
  const windows = [
    win("可见有标题", 1, "a.exe", "C:\\a.exe", true),
    win("后台有标题", 2, "b.exe", "C:\\b.exe", false),
    { title: NO_TITLE, hwnd: 3, process: "c.exe", PID: 3, path: "C:\\c.exe", visible: true },
    { title: NO_TITLE, hwnd: 4, process: "d.exe", PID: 4, path: "C:\\d.exe", visible: false },
  ];

  it("默认只显示有标题的可见窗口", () => {
    expect(applyListFilters(windows).map((w) => w.hwnd)).toEqual([1]);
  });

  it("showBackground 放开后台窗口", () => {
    expect(
      applyListFilters(windows, { showBackground: true }).map((w) => w.hwnd),
    ).toEqual([1, 2]);
  });

  it("showUntitled 放开无标题窗口", () => {
    expect(
      applyListFilters(windows, { showUntitled: true }).map((w) => w.hwnd),
    ).toEqual([1, 3]);
  });

  it("两者全开显示全部，再叠加搜索", () => {
    expect(
      applyListFilters(windows, { showBackground: true, showUntitled: true }),
    ).toHaveLength(4);
    expect(
      applyListFilters(windows, {
        showBackground: true,
        showUntitled: true,
        search: "a.exe",
      }).map((w) => w.hwnd),
    ).toEqual([1]);
  });
});

describe("规则构造与去重", () => {
  it("addWindowRules 追加选中窗口并跳过已覆盖项", () => {
    const existing = [windowRuleFromWindow(win("微信", 1, "WeChat.exe"))];
    const picked = [win("微信", 1, "WeChat.exe"), win("记事本", 2, "notepad.exe")];
    const result = addWindowRules(existing, picked);
    expect(result).toHaveLength(2, "已存在的微信不重复添加");
    expect(result[1].title).toBe("记事本");
    expect(existing).toHaveLength(1, "入参不应被修改");
  });

  it("addProcessRules 按路径去重", () => {
    const existing = [processRuleFromWindow(win("窗口一", 1, "game.exe", "C:\\game.exe"))];
    const picked = [
      win("窗口二", 2, "game.exe", "C:\\game.exe"), // 同路径，跳过
      win("微信", 3, "WeChat.exe", "C:\\WeChat.exe"),
      win("无路径", 4, "x.exe", ""), // 空路径，跳过
    ];
    const result = addProcessRules(existing, picked);
    expect(result.map((r) => r.path)).toEqual(["C:\\game.exe", "C:\\WeChat.exe"]);
  });

  it("正则进程规则不参与路径去重种子", () => {
    const existing = [newProcessRegexRule()];
    const picked = [win("微信", 1, "WeChat.exe", "C:\\WeChat.exe")];
    expect(addProcessRules(existing, picked)).toHaveLength(2);
  });
});

describe("正则规则默认值", () => {
  it("escapeRegex 转义元字符", () => {
    expect(escapeRegex("WeChat.exe")).toBe("WeChat\\.exe");
    expect(escapeRegex("a+b(c)")).toBe("a\\+b\\(c\\)");
  });

  it("containsPattern 生成包含式正则", () => {
    expect(containsPattern("微信")).toBe(".*微信.*");
    // 生成的正则应真的能匹配含该字面量的字符串
    expect(new RegExp(containsPattern("WeChat.exe")).test("C:\\WeChat.exe")).toBe(true);
    expect(new RegExp(containsPattern("WeChat.exe")).test("C:\\WeChatXexe")).toBe(false);
  });

  it("窗口正则规则用选中窗口的标题作种子", () => {
    expect(newWindowRegexRule("微信").regex).toBe(".*微信.*");
    expect(newWindowRegexRule().regex).toBe(".*关键词.*");
    expect(newWindowRegexRule(NO_TITLE).regex).toBe(".*关键词.*", "无标题不作种子");
  });

  it("新规则带默认匹配范围", () => {
    const w = newWindowRegexRule("x");
    expect(w.include_untitled).toBe(false);
    expect(w.include_background).toBe(false);

    const p = newProcessRegexRule("game.exe");
    expect(p.regex).toBe(".*game\\.exe.*");
    expect(p.by_name).toBe(false);
    expect(p.include_untitled).toBe(false);
    expect(p.include_background).toBe(false);
  });

  it("进程精确规则默认按路径、不含无标题窗口", () => {
    const r = processRuleFromWindow(win("窗口", 1, "game.exe", "C:\\game.exe"));
    expect(r.by_name).toBe(false);
    expect(r.include_untitled).toBe(false);
    expect(r.include_background).toBe(false);
  });
});

describe("traceWindowRule", () => {
  const rule = windowRuleFromWindow(win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"));

  it("句柄命中为 live", () => {
    expect(traceWindowRule(rule, [win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")])).toBe("live");
  });

  it("句柄失效但标题+路径一致为 reacquired", () => {
    expect(traceWindowRule(rule, [win("微信", 99, "WeChat.exe", "C:\\WeChat.exe")])).toBe(
      "reacquired",
    );
  });

  it("找不到为 missing", () => {
    expect(traceWindowRule(rule, [win("记事本", 5, "notepad.exe")])).toBe("missing");
  });

  it("正则规则返回 regex", () => {
    expect(traceWindowRule({ regex: "^微信" }, [])).toBe("regex");
  });
});

describe("iconPathsToFetch", () => {
  it("去重、跳过空路径与已缓存路径（含负缓存）", () => {
    const windows = [
      win("一", 1, "a.exe", "C:\\a.exe"),
      win("二", 2, "a.exe", "C:\\a.exe"),
      win("三", 3, "b.exe", "C:\\b.exe"),
      win("四", 4, "c.exe", ""),
      win("五", 5, "d.exe", "C:\\d.exe"),
    ];
    const cache = { "C:\\b.exe": "data:...", "C:\\d.exe": null };
    expect(iconPathsToFetch(windows, cache)).toEqual(["C:\\a.exe"]);
  });
});
