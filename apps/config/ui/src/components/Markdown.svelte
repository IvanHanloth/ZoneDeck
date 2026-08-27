<script>
  // 渲染公告 / 更新日志里的 Markdown。
  import { renderMarkdown } from "../lib/markdown.js";
  import { toast } from "../lib/state.svelte.js";
  import { openExternal } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  let { source = "" } = $props();

  const html = $derived(renderMarkdown(source));

  // 链接一律交给系统浏览器，webview 内导航后回不到配置界面。
  function onClick(e) {
    const a = e.target.closest?.("a[href]");
    if (!a) return;
    e.preventDefault();
    openExternal(a.getAttribute("href")).catch((err) =>
      toast(t("options.openLinkFailed", { err }), true),
    );
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="md" onclick={onClick} onauxclick={(e) => e.preventDefault()}>
  {@html html}
</div>

<style>
  .md {
    font-size: 13px;
    line-height: 1.65;
    word-break: break-word;
  }
  /* 渲染结果由 {@html} 注入，拿不到作用域类名，只能用 :global。 */
  .md :global(> *:first-child) {
    margin-top: 0;
  }
  .md :global(> *:last-child) {
    margin-bottom: 0;
  }
  .md :global(p),
  .md :global(ul),
  .md :global(ol),
  .md :global(pre),
  .md :global(blockquote) {
    margin: 0 0 10px;
  }
  .md :global(h1),
  .md :global(h2),
  .md :global(h3),
  .md :global(h4),
  .md :global(h5),
  .md :global(h6) {
    margin: 16px 0 8px;
    font-weight: 600;
    line-height: 1.3;
  }
  .md :global(h1) {
    font-size: 17px;
  }
  .md :global(h2) {
    font-size: 15px;
  }
  .md :global(h3) {
    font-size: 14px;
  }
  .md :global(h4),
  .md :global(h5),
  .md :global(h6) {
    font-size: 13px;
  }
  .md :global(h1),
  .md :global(h2) {
    padding-bottom: 5px;
    border-bottom: 1px solid var(--divider);
  }
  .md :global(ul),
  .md :global(ol) {
    padding-left: 1.6em;
  }
  .md :global(li) {
    margin: 2px 0;
  }
  .md :global(li > ul),
  .md :global(li > ol) {
    margin: 2px 0;
  }
  .md :global(li.task) {
    list-style: none;
    margin-left: -1.4em;
  }
  .md :global(li.task > input) {
    margin-right: 6px;
    vertical-align: -1px;
  }
  .md :global(a) {
    color: var(--accent);
    text-decoration: none;
    cursor: pointer;
  }
  .md :global(a:hover) {
    text-decoration: underline;
  }
  .md :global(code) {
    padding: 0.15em 0.4em;
    border-radius: var(--r-control);
    background: var(--subtle-hover);
    font-family: var(--font-mono);
    font-size: 0.9em;
  }
  .md :global(pre) {
    padding: 10px 12px;
    border-radius: var(--r-control);
    background: var(--card-2);
    border: 1px solid var(--stroke);
    overflow-x: auto;
  }
  .md :global(pre code) {
    padding: 0;
    background: none;
    border-radius: 0;
    white-space: pre;
  }
  .md :global(blockquote) {
    padding-left: 12px;
    border-left: 3px solid var(--accent);
    color: var(--text-2);
  }
  .md :global(hr) {
    height: 1px;
    margin: 14px 0;
    border: 0;
    background: var(--divider);
  }
  .md :global(del) {
    color: var(--text-2);
  }
</style>
