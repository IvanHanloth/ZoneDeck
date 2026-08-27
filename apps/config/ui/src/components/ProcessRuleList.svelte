<script>
  // 「进程隐藏」规则：按可执行文件路径隐藏该程序的所有窗口。
  import IconPlus from "~icons/lucide/plus";
  import IconTrash from "~icons/lucide/trash-2";
  import IconRegex from "~icons/lucide/regex";
  import IconBox from "~icons/lucide/box";
  import ScopeSelect from "./ScopeSelect.svelte";
  import CheckBox from "./fluent/CheckBox.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import { isRegexRule } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  // 行内的正则输入框、匹配方式与范围下拉自己处理点击，不参与选中
  const CONTROLS = "input, select, button, textarea, label, a, [role='group']";

  const byOptions = $derived([
    { value: false, label: t("processRules.byPath") },
    { value: true, label: t("processRules.byName") },
  ]);

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
    class="lv-body"
    role="listbox"
    aria-multiselectable="true"
    aria-label={t("processRules.aria")}
  >
    {#if rules.length === 0}
      <p class="hint lv-empty">{t("common.empty")}</p>
    {:else}
      {#each rules as rule, i (i)}
        <div
          class="lv-row rule-row"
          class:sel={selected.includes(i)}
          role="option"
          aria-selected={selected.includes(i)}
          tabindex="0"
          onclick={(e) => onRowClick(e, i)}
          onkeydown={(e) => onRowKey(e, i)}
        >
          <CheckBox
            small
            checked={selected.includes(i)}
            onchange={() => toggle(i)}
            ariaLabel={t("windowRules.selectRule")}
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

          <ComboBox
            compact
            bind:value={rule.by_name}
            options={byOptions}
            title={t("processRules.byTitle")}
            ariaLabel={t("processRules.byAria")}
          />
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
  .rule-row {
    cursor: default;
  }
  .rule-row:focus-visible {
    outline: 2px solid var(--focus-outer);
    outline-offset: -2px;
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
  /* 路径从右往左省略：末段的文件名比盘符更值钱 */
  .rpath {
    flex: 1;
    color: var(--text-2);
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
    border: 1px solid var(--stroke);
    border-bottom-color: var(--stroke-strong);
    border-radius: var(--r-control);
    background: var(--control);
    color: var(--text);
    padding: 4px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    user-select: text;
    cursor: text;
  }
  .regex-input:focus {
    outline: none;
    background: var(--control-focus);
    border-bottom-color: var(--accent);
  }
  /* 保存时判定为「可能过宽」，见 BroadRegexModal */
  .regex-input.broad {
    border-color: var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, var(--control));
  }
</style>
