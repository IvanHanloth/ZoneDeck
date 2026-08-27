<script>
  // 「窗口隐藏」规则：精确规则显示标题 + 进程 + 追溯状态。
  import IconPlus from "~icons/lucide/plus";
  import IconTrash from "~icons/lucide/trash-2";
  import IconRegex from "~icons/lucide/regex";
  import IconAppWindow from "~icons/lucide/app-window";
  import ScopeSelect from "./ScopeSelect.svelte";
  import CheckBox from "./fluent/CheckBox.svelte";
  import { NO_TITLE, isRegexRule, traceWindowRule } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  // 行内的正则输入框、范围下拉自己处理点击，不参与选中
  const CONTROLS = "input, select, button, textarea, label, a, [role='group']";

  const STATUS = {
    reacquired: { key: "windowRules.statusReacquired", cls: "warn" },
    missing: { key: "windowRules.statusMissing", cls: "danger" },
  };

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
    <span class="title-text"><IconAppWindow width="15" height="15" /> {t("windowRules.title")}</span>
    <span class="count">{rules.length}</span>
    <div class="tools">
      <button class="mini primary" title={t("windowRules.addTitle")} onclick={() => onadd?.()}>
        <IconPlus width="14" height="14" /> {t("windowRules.add")}
      </button>
      <button class="mini" title={t("windowRules.addRegexTitle")} onclick={() => onaddregex?.()}>
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
    aria-label={t("windowRules.aria")}
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
            <span class="regex-tag"><IconRegex width="12" height="12" /> {t("windowRules.titleRegexTag")}</span>
            <input
              class="regex-input"
              class:broad={app.broadPatterns.has(rule.regex)}
              title={app.broadPatterns.has(rule.regex) ? t("broadRegex.inputTitle") : ""}
              placeholder={t("windowRules.titleRegexPlaceholder")}
              bind:value={rule.regex}
              spellcheck="false"
            />
            <ScopeSelect
              bind:includeUntitled={rule.include_untitled}
              bind:includeBackground={rule.include_background}
            />
          {:else}
            {@const st = traceWindowRule(rule, app.available)}
            {#if app.icons[rule.path]}
              <img class="ic" src={app.icons[rule.path]} alt="" />
            {:else}
              <span class="ic fallback"><IconAppWindow width="14" height="14" /></span>
            {/if}
            <span class="rtitle" title={rule.path}>{rule.title === NO_TITLE ? t("common.noTitleWindow") : rule.title}</span>
            <span class="rproc">{rule.process}</span>
            {#if STATUS[st]}
              <span class="badge {STATUS[st].cls}">{t(STATUS[st].key)}</span>
            {:else}
              <span class="dot live" title={t("windowRules.live")}></span>
            {/if}
          {/if}
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
  .rtitle {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rproc {
    color: var(--text-2);
    font-size: 12px;
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  .badge {
    flex: none;
    font-size: 11px;
    padding: 1px 8px;
    border-radius: 99px;
  }
  .badge.warn {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 16%, transparent);
  }
  .badge.danger {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 16%, transparent);
  }
  .dot.live {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }
</style>
