<script>
  // 「进程隐藏」规则：按可执行文件路径隐藏该程序的所有窗口。
  import IconPlus from "~icons/lucide/plus";
  import IconTrash from "~icons/lucide/trash-2";
  import IconRegex from "~icons/lucide/regex";
  import IconBox from "~icons/lucide/box";
  import ScopeSelect from "./ScopeSelect.svelte";
  import { isRegexRule } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  // 行内的正则输入框、匹配方式与范围下拉自己处理点击，不参与选中
  const CONTROLS = "input, select, button, textarea, label, a, [role='group']";

  function toggle(i) {
    selected = selected.includes(i) ? selected.filter((x) => x !== i) : [...selected, i];
  }

  function onRowClick(e, i) {
    if (e.target.closest(CONTROLS)) return;
    toggle(i);
  }

  function onRowKey(e, i) {
    if (e.target !== e.currentTarget) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      toggle(i);
    }
  }

  function remove() {
    if (!selected.length) return;
    rules = rules.filter((_, i) => !selected.includes(i));
    selected = [];
  }
</script>

<div class="list-box">
  <div class="list-title">
    <span class="title-text"><IconBox width="15" height="15" /> {t("processRules.title")}</span>
    <span class="count">{rules.length}</span>
    <div class="tools">
      <button class="mini primary" title={t("processRules.addTitle")} onclick={() => onadd?.()}>
        <IconPlus width="14" height="14" /> {t("processRules.add")}
      </button>
      <button class="mini" title={t("processRules.addRegexTitle")} onclick={() => onaddregex?.()}>
        <IconRegex width="14" height="14" /> {t("windowRules.regex")}
      </button>
      <button class="mini" title={t("windowRules.removeTitle")} onclick={remove} disabled={!selected.length}>
        <IconTrash width="14" height="14" /> {t("windowRules.remove")}
      </button>
    </div>
  </div>

  <div
    class="rule-list"
    role="listbox"
    aria-multiselectable="true"
    aria-label={t("processRules.aria")}
  >
    {#if rules.length === 0}
      <p class="hint empty">{t("common.empty")}</p>
    {:else}
      {#each rules as rule, i (i)}
        <div
          class="rule-row"
          class:sel={selected.includes(i)}
          role="option"
          aria-selected={selected.includes(i)}
          tabindex="0"
          onclick={(e) => onRowClick(e, i)}
          onkeydown={(e) => onRowKey(e, i)}
        >
          <input
            type="checkbox"
            tabindex="-1"
            checked={selected.includes(i)}
            onchange={() => toggle(i)}
            aria-label={t("windowRules.selectRule")}
          />
          {#if isRegexRule(rule)}
            <span class="regex-tag">
              <IconRegex width="12" height="12" />
              {t(rule.by_name ? "processRules.nameRegexTag" : "processRules.pathRegexTag")}
            </span>
            <input
              class="regex-input"
              class:broad={app.broadPatterns.has(rule.regex)}
              title={app.broadPatterns.has(rule.regex) ? t("broadRegex.inputTitle") : ""}
              placeholder={t(
                rule.by_name
                  ? "processRules.nameRegexPlaceholder"
                  : "processRules.pathRegexPlaceholder",
              )}
              bind:value={rule.regex}
              spellcheck="false"
            />
          {:else}
            {#if app.icons[rule.path]}
              <img class="ic" src={app.icons[rule.path]} alt="" />
            {:else}
              <span class="ic fallback"><IconBox width="14" height="14" /></span>
            {/if}
            <span class="rproc">{rule.process || t("common.unknownProcess")}</span>
            <span class="rpath" title={rule.path}>
              {rule.by_name ? t("processRules.anyDirectory") : rule.path}
            </span>
          {/if}

          <select
            class="by"
            bind:value={rule.by_name}
            title={t("processRules.byTitle")}
            aria-label={t("processRules.byAria")}
          >
            <option value={false}>{t("processRules.byPath")}</option>
            <option value={true}>{t("processRules.byName")}</option>
          </select>
          <ScopeSelect
            bind:includeUntitled={rule.include_untitled}
            bind:includeBackground={rule.include_background}
          />
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .list-box {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    flex: 1;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .list-title {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    font-weight: 600;
    font-size: 13px;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    flex: none;
  }
  .title-text {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--text);
  }
  .count {
    font-weight: 500;
    font-size: 12px;
    background: var(--hover);
    border-radius: 99px;
    padding: 1px 8px;
  }
  .tools {
    margin-left: auto;
    display: flex;
    gap: 6px;
  }
  .mini {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .mini:hover:not(:disabled) {
    background: var(--hover);
    border-color: var(--accent);
  }
  .mini.primary {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  .mini.primary:hover:not(:disabled) {
    color: var(--on-accent);
    background: var(--accent-strong);
    border-color: var(--accent-strong);
  }
  .mini:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .rule-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }
  .empty {
    text-align: center;
    padding: 20px 8px;
  }

  .rule-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 6px;
    cursor: pointer;
  }
  .rule-row:hover {
    background: var(--hover);
  }
  .rule-row.sel {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .rule-row.sel:hover {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .rule-row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .rule-row input[type="checkbox"] {
    accent-color: var(--accent);
    flex: none;
    cursor: pointer;
  }
  .by {
    flex: none;
    padding: 2px 4px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11.5px;
  }
  .by:focus {
    outline: none;
    border-color: var(--accent);
  }
  .ic {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .ic.fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0.55;
  }
  .rproc {
    flex: none;
    font-weight: 500;
  }
  .rpath {
    flex: 1;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .regex-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
    font-size: 11.5px;
    color: var(--accent);
  }
  .regex-input {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--text);
    padding: 3px 8px;
    font-family: ui-monospace, monospace;
    font-size: 12px;
  }
  .regex-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  /* 保存时判定为「可能过宽」，见 BroadRegexModal */
  .regex-input.broad {
    border-color: var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, var(--surface-2));
  }
</style>
