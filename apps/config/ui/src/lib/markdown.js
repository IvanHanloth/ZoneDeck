// 极简 Markdown 渲染，供公告与更新日志使用。只覆盖 GitHub 风格的常用子集，
// 不引入任何依赖。
//
// 内容来自 Verhub 远端，因此这里**先转义再拼标签**：输出里出现的标签全部由本
// 模块生成，源文本里的原始 HTML 只会被当成字面量显示，无需再过一遍消毒。

const ESCAPE = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };

const FENCE = /^ {0,3}(`{3,}|~{3,})/;
const HR = /^ {0,3}([-*_])[ \t]*(?:\1[ \t]*){2,}$/;
const HEADING = /^ {0,3}(#{1,6})[ \t]+(.*?)[ \t]*#*[ \t]*$/;
const QUOTE = /^ {0,3}> ?(.*)$/;
const ITEM = /^([ \t]*)(?:([-*+])|(\d{1,9})[.)])[ \t]+(.*)$/;
const TASK = /^\[([ xX])\][ \t]+/;

// 行内解析的占位符哨兵。NUL 不会出现在正文里，也不属于 \s，因此裸链接之类的
// 规则不会越过它去改写已经成型的标签。
const SENTINEL = String.fromCharCode(0);
const PLACEHOLDER = new RegExp(`${SENTINEL}(\\d+)${SENTINEL}`, "g");

const escapeHtml = (s) => s.replace(/[&<>"]/g, (c) => ESCAPE[c]);

/** 只放行 http(s) 与 mailto：javascript: / data: 等一律按纯文本处理。 */
function safeUrl(raw) {
  const url = raw.trim();
  return /^(?:https?:\/\/|mailto:)/i.test(url) ? url : null;
}

const startsBlock = (line) =>
  FENCE.test(line) || HR.test(line) || HEADING.test(line) || QUOTE.test(line) || ITEM.test(line);

/** 渲染行内标记，返回 HTML 片段。 */
function inline(text) {
  const stash = [];
  // 已成型的标签先寄存成占位符，避免被后续规则二次改写。
  const hold = (html) => `${SENTINEL}${stash.push(html) - 1}${SENTINEL}`;
  let s = escapeHtml(text);

  s = s.replace(/(`+)([^`]+?)\1/g, (_, __, code) => hold(`<code>${code}</code>`));

  // CSP 只允许 self / data: 图源，远端图片加载不出来，统一退化成链接。
  s = s.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, (m, alt, dest) => anchor(dest, alt, hold) ?? m);
  s = s.replace(/\[([^\]]*)\]\(([^)]+)\)/g, (m, label, dest) => anchor(dest, label, hold) ?? m);

  // 裸链接。前面必须是行首/空白/左括号，因此紧挨占位符的文本不会被重复包裹。
  s = s.replace(/(^|[\s(])(https?:\/\/\S+)/g, (_, pre, raw) => {
    const trailing = raw.match(/[.,;:!?)]+$/)?.[0] ?? "";
    const url = raw.slice(0, raw.length - trailing.length);
    return pre + hold(`<a href="${url}">`) + url + hold("</a>") + trailing;
  });

  s = s
    .replace(/\*\*([^\s*](?:[^*]*[^\s*])?)\*\*/g, "<strong>$1</strong>")
    .replace(/__([^\s_](?:[^_]*[^\s_])?)__/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^\s*](?:[^*]*[^\s*])?)\*/g, "$1<em>$2</em>")
    .replace(/(^|[^\w_])_([^\s_](?:[^_]*[^\s_])?)_/g, "$1<em>$2</em>")
    .replace(/~~([^~]+)~~/g, "<del>$1</del>");

  return s.replace(PLACEHOLDER, (_, i) => stash[+i]);
}

/** 拼一个链接；目标协议不安全时返回 null，交由调用方原样保留。 */
function anchor(dest, label, hold) {
  const url = safeUrl(dest.split(/[ \t]/)[0]);
  if (!url) return null;
  return hold(`<a href="${url}">`) + (label || url) + hold("</a>");
}

/** 收集一段连续的列表行，返回 [html, 下一个未消费的行号]。 */
function list(lines, start) {
  const items = [];
  let i = start;
  while (i < lines.length) {
    const m = lines[i].match(ITEM);
    if (m) {
      items.push({
        indent: m[1].replace(/\t/g, "    ").length,
        ordered: !!m[3],
        start: m[3] ? Number(m[3]) : 0,
        text: [m[4]],
      });
      i++;
      continue;
    }
    if (!lines[i].trim()) {
      // 列表项之间允许空行，空行之后不再是列表项才算结束。
      if (lines[i + 1] && ITEM.test(lines[i + 1])) {
        i++;
        continue;
      }
      break;
    }
    if (items.length && /^[ \t]/.test(lines[i])) {
      items[items.length - 1].text.push(lines[i].trim());
      i++;
      continue;
    }
    break;
  }

  let html = "";
  let pos = 0;
  while (pos < items.length) {
    const [chunk, next] = buildList(items, pos);
    html += chunk;
    pos = next;
  }
  return [html, i];
}

/** 把扁平的 items 按缩进还原成嵌套列表，返回 [html, 下一个未消费的下标]。 */
function buildList(items, pos) {
  const { indent, ordered, start } = items[pos];
  const tag = ordered ? "ol" : "ul";
  let html = ordered && start > 1 ? `<ol start="${start}">` : `<${tag}>`;
  let i = pos;

  while (i < items.length && items[i].indent >= indent) {
    if (items[i].indent > indent) {
      const [sub, next] = buildList(items, i);
      // 子列表挂进上一个 <li> 内部。
      html = html.endsWith("</li>") ? `${html.slice(0, -5)}${sub}</li>` : html + sub;
      i = next;
      continue;
    }
    if (items[i].ordered !== ordered) break; // 换了列表类型，交给上层另起一个
    html += item(items[i]);
    i++;
  }
  return [`${html}</${tag}>`, i];
}

function item(it) {
  const task = it.text[0].match(TASK);
  if (task) {
    const rest = [it.text[0].slice(task[0].length), ...it.text.slice(1)];
    const checked = task[1] === " " ? "" : " checked";
    return `<li class="task"><input type="checkbox" disabled${checked}>${rest.map(inline).join("<br>")}</li>`;
  }
  return `<li>${it.text.map(inline).join("<br>")}</li>`;
}

function blocks(lines) {
  const out = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (!line.trim()) {
      i++;
      continue;
    }

    const fence = line.match(FENCE);
    if (fence) {
      const close = fence[1][0].repeat(3);
      const body = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith(close)) body.push(lines[i++]);
      i++; // 吃掉收尾栅栏；栅栏缺失时正好越界结束
      out.push(`<pre><code>${escapeHtml(body.join("\n"))}</code></pre>`);
      continue;
    }

    if (HR.test(line)) {
      out.push("<hr>");
      i++;
      continue;
    }

    const h = line.match(HEADING);
    if (h) {
      out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`);
      i++;
      continue;
    }

    if (QUOTE.test(line)) {
      const body = [];
      while (i < lines.length && lines[i].trim()) {
        const m = lines[i].match(QUOTE);
        body.push(m ? m[1] : lines[i]); // 无 > 前缀的续行也算引用内容
        i++;
      }
      out.push(`<blockquote>${blocks(body)}</blockquote>`);
      continue;
    }

    if (ITEM.test(line)) {
      const [html, next] = list(lines, i);
      out.push(html);
      i = next;
      continue;
    }

    // 段落。软换行按 GitHub 评论的习惯渲染成 <br>，而不是并成一行。
    const para = [];
    while (i < lines.length && lines[i].trim() && !startsBlock(lines[i])) {
      para.push(lines[i].trim());
      i++;
    }
    out.push(`<p>${para.map(inline).join("<br>")}</p>`);
  }

  return out.join("");
}

/** 把 Markdown 源文本渲染成 HTML 字符串。 */
export function renderMarkdown(source) {
  if (!source) return "";
  return blocks(String(source).replace(/\r\n?/g, "\n").split("\n"));
}
