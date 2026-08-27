import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown.js";

describe("renderMarkdown 块级", () => {
  it("空输入返回空串", () => {
    expect(renderMarkdown("")).toBe("");
    expect(renderMarkdown(null)).toBe("");
    expect(renderMarkdown(undefined)).toBe("");
  });

  it("标题按级数渲染", () => {
    expect(renderMarkdown("# 大标题")).toBe("<h1>大标题</h1>");
    expect(renderMarkdown("### 小标题")).toBe("<h3>小标题</h3>");
    expect(renderMarkdown("####### 七个井号不是标题")).toBe("<p>####### 七个井号不是标题</p>");
  });

  it("段落内的软换行渲染成 <br>，空行分段", () => {
    expect(renderMarkdown("第一行\n第二行\n\n另一段")).toBe("<p>第一行<br>第二行</p><p>另一段</p>");
  });

  it("分割线", () => {
    expect(renderMarkdown("---")).toBe("<hr>");
    expect(renderMarkdown("***")).toBe("<hr>");
    expect(renderMarkdown("--")).toBe("<p>--</p>");
  });

  it("引用块内部继续解析", () => {
    expect(renderMarkdown("> **重要**")).toBe("<blockquote><p><strong>重要</strong></p></blockquote>");
  });

  it("围栏代码块原样保留、不解析行内标记", () => {
    expect(renderMarkdown("```js\nconst a = **1**;\n```")).toBe("<pre><code>const a = **1**;</code></pre>");
  });

  it("未闭合的围栏吃到结尾", () => {
    expect(renderMarkdown("```\na\nb")).toBe("<pre><code>a\nb</code></pre>");
  });
});

describe("renderMarkdown 列表", () => {
  it("无序列表", () => {
    expect(renderMarkdown("- 甲\n- 乙")).toBe("<ul><li>甲</li><li>乙</li></ul>");
  });

  it("有序列表，起始序号非 1 时带 start", () => {
    expect(renderMarkdown("1. 甲\n2. 乙")).toBe("<ol><li>甲</li><li>乙</li></ol>");
    expect(renderMarkdown("3. 丙")).toBe('<ol start="3"><li>丙</li></ol>');
  });

  it("缩进还原成嵌套列表", () => {
    expect(renderMarkdown("- 甲\n  - 甲一\n- 乙")).toBe(
      "<ul><li>甲<ul><li>甲一</li></ul></li><li>乙</li></ul>",
    );
  });

  it("任务列表渲染成只读复选框", () => {
    expect(renderMarkdown("- [x] 做完了\n- [ ] 还没做")).toBe(
      '<ul><li class="task"><input type="checkbox" disabled checked>做完了</li>' +
        '<li class="task"><input type="checkbox" disabled>还没做</li></ul>',
    );
  });

  it("列表结束后回到段落", () => {
    expect(renderMarkdown("- 甲\n\n收尾")).toBe("<ul><li>甲</li></ul><p>收尾</p>");
  });
});

describe("renderMarkdown 行内", () => {
  it("粗体、斜体、删除线、行内代码", () => {
    expect(renderMarkdown("**粗** *斜* ~~删~~ `码`")).toBe(
      "<p><strong>粗</strong> <em>斜</em> <del>删</del> <code>码</code></p>",
    );
  });

  it("行内代码里的标记不再解析", () => {
    expect(renderMarkdown("`**不是粗体**`")).toBe("<p><code>**不是粗体**</code></p>");
  });

  it("蛇形命名里的下划线不当成斜体", () => {
    expect(renderMarkdown("some_var_name")).toBe("<p>some_var_name</p>");
  });

  it("链接与裸链接", () => {
    expect(renderMarkdown("[官网](https://zonedeck.ivan-hanloth.cn/)")).toBe(
      '<p><a href="https://zonedeck.ivan-hanloth.cn/">官网</a></p>',
    );
    expect(renderMarkdown("见 https://example.com 。")).toBe(
      '<p>见 <a href="https://example.com">https://example.com</a> 。</p>',
    );
  });

  it("裸链接不吞掉句末标点", () => {
    expect(renderMarkdown("见 https://example.com.")).toBe(
      '<p>见 <a href="https://example.com">https://example.com</a>.</p>',
    );
  });

  it("图片退化成链接（CSP 不允许远端图源）", () => {
    expect(renderMarkdown("![截图](https://example.com/a.png)")).toBe(
      '<p><a href="https://example.com/a.png">截图</a></p>',
    );
  });
});

describe("renderMarkdown 安全", () => {
  it("原始 HTML 一律转义", () => {
    expect(renderMarkdown("<script>alert(1)</script>")).toBe(
      "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>",
    );
    expect(renderMarkdown('<img src=x onerror="alert(1)">')).toBe(
      "<p>&lt;img src=x onerror=&quot;alert(1)&quot;&gt;</p>",
    );
  });

  it("非 https/mailto 协议的链接不生成 <a>", () => {
    expect(renderMarkdown("[点我](javascript:alert(1))")).toBe("<p>[点我](javascript:alert(1))</p>");
    expect(renderMarkdown("[点我](data:text/html,<script>)")).toContain("[点我](data:text/html,");
  });

  it("明文 http 链接不生成 <a>", () => {
    expect(renderMarkdown("[点我](http://example.com)")).toBe("<p>[点我](http://example.com)</p>");
    expect(renderMarkdown("见 http://example.com 。")).toBe("<p>见 http://example.com 。</p>");
  });

  it("链接目标里的引号被转义，无法逃出属性", () => {
    // 目标在第一个右括号处截断（不做括号配对），多出来的括号留在正文里。
    expect(renderMarkdown('[x](https://a.com/"onmouseover="alert(1))')).toBe(
      '<p><a href="https://a.com/&quot;onmouseover=&quot;alert(1">x</a>)</p>',
    );
  });

  it("代码块内容同样转义", () => {
    expect(renderMarkdown("```\n<b>x</b>\n```")).toBe("<pre><code>&lt;b&gt;x&lt;/b&gt;</code></pre>");
  });
});
