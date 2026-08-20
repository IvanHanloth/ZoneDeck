<script>
  // 保存时发现「可能过宽」的正则；两个按钮都已写盘，区别只在此后还提不提醒。
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import IconTriangleAlert from "~icons/lucide/triangle-alert";
  import { BREADTH_SAMPLES } from "../lib/regexcheck.js";
  import { t } from "../lib/i18n.svelte.js";
  import { acknowledgeBroadRegex, dismissBroadRegex, app } from "../lib/state.svelte.js";

  const items = $derived(app.broadRegex ?? []);

  // 叉号 / Esc / 点遮罩关闭等同「我知道了」，标红保留。
  function onOpenChange(v) {
    if (!v) dismissBroadRegex();
  }

  // 文案键须为字面量，供 scripts/i18n-check.ps1 静态检查。
  const SOURCE = {
    window: "windowRules.title",
    process: "processRules.title",
    whitelist: "whitelist.title",
  };
</script>

<ContentDialog title={t("broadRegex.title")} bind:open={() => !!app.broadRegex, onOpenChange}>
  <p class="lead">
    <IconTriangleAlert width="15" height="15" />
    {t("broadRegex.lead", { total: BREADTH_SAMPLES })}
  </p>

  <ul class="items">
    {#each items as item (item.pattern)}
      <li>
        <span class="source">{t(SOURCE[item.kind])}</span>
        <code>{item.pattern}</code>
        <span class="hits">{t("broadRegex.hits", { hits: item.hits, total: BREADTH_SAMPLES })}</span>
      </li>
    {/each}
  </ul>

  <p class="hint">{t("broadRegex.saved")}</p>

  {#snippet footer()}
    <button class="btn" onclick={dismissBroadRegex}>{t("broadRegex.dismiss")}</button>
    <button class="btn primary" onclick={acknowledgeBroadRegex}>{t("broadRegex.keep")}</button>
  {/snippet}
</ContentDialog>

<style>
  .lead {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--warn);
    line-height: 1.6;
    margin-bottom: 10px;
  }
  .items {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .items li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: 1px solid var(--stroke);
    border-radius: 7px;
    background: var(--card-2);
  }
  .source {
    flex: none;
    font-size: 11.5px;
    color: var(--text-2);
  }
  .items code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, monospace;
    font-size: 12px;
    color: var(--danger);
  }
  .hits {
    flex: none;
    font-size: 11.5px;
    color: var(--text-2);
  }
  .hint {
    margin-top: 10px;
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-2);
  }
  .btn.primary {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  .btn.primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }
</style>
