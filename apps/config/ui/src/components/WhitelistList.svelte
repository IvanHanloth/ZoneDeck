<script>
  // 「白名单」：按进程声明在哪些模式下跳过；行尾是隐藏 / 冻结 / 静音三个开关。
  import IconPlus from "~icons/lucide/plus";
  import IconTrash from "~icons/lucide/trash-2";
  import IconRegex from "~icons/lucide/regex";
  import IconShieldCheck from "~icons/lucide/shield-check";
  import IconBox from "~icons/lucide/box";
  import IconLock from "~icons/lucide/lock";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconSnowflake from "~icons/lucide/snowflake";
  import IconVolumeOff from "~icons/lucide/volume-off";
  import { isRegexRule } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  // 三个模式开关。文案键须为字面量，供 scripts/i18n-check.ps1 静态检查。
  const MODES = [
    { field: "ignore_hide", icon: IconEyeOff, label: "whitelist.ignoreHide" },
    { field: "ignore_freeze", icon: IconSnowflake, label: "whitelist.ignoreFreeze" },
    { field: "ignore_mute", icon: IconVolumeOff, label: "whitelist.ignoreMute" },
  ];

  // 三个开关相互独立，互不联动。
  function setMode(rule, field, on) {
    rule[field] = on;
  }

  function remove() {
    if (!selected.length) return;
    rules = rules.filter((_, i) => !selected.includes(i));
    selected = [];
  }
</script>

<div class="list-box">
  <div class="list-title">
    <span class="title-text"><IconShieldCheck width="15" height="15" /> {t("whitelist.title")}</span>
    <span class="count">{rules.length + app.whitelistBuiltins.length}</span>
    <div class="tools">
      <button class="mini primary" title={t("whitelist.addTitle")} onclick={() => onadd?.()}>
        <IconPlus width="14" height="14" /> {t("whitelist.add")}
      </button>
      <button class="mini" title={t("whitelist.addRegexTitle")} onclick={() => onaddregex?.()}>
        <IconRegex width="14" height="14" /> {t("windowRules.regex")}
      </button>
      <button class="mini" title={t("windowRules.removeTitle")} onclick={remove} disabled={!selected.length}>
        <IconTrash width="14" height="14" /> {t("windowRules.remove")}
      </button>
    </div>
  </div>

  <div class="rule-list" role="listbox" aria-label={t("whitelist.aria")}>
    <!-- 内置项排在最前，不可编辑、不可删除。 -->
    {#each app.whitelistBuiltins as builtin (builtin.key)}
      <div class="rule-row builtin">
        <span class="lock" title={t("whitelist.builtinLocked")}>
          <IconLock width="13" height="13" />
        </span>
        <span class="ic fallback"><IconShieldCheck width="14" height="14" /></span>
        <span class="rproc">
          {t(builtin.key === "core" ? "whitelist.builtinCore" : "whitelist.builtinConfig")}
        </span>
        <span class="rpath ltr" title={t("whitelist.builtinLocked")}>{builtin.names.join("、")}</span>
        <span class="by-placeholder"></span>
        <div class="modes">
          {#each MODES as mode (mode.field)}
            {@const on = mode.field === "ignore_freeze"}
            <span
              class="mode"
              class:on
              title={t(on ? "whitelist.builtinLocked" : mode.label)}
            >
              <mode.icon width="13" height="13" />
            </span>
          {/each}
        </div>
      </div>
    {/each}

    {#if rules.length === 0 && app.whitelistBuiltins.length === 0}
      <p class="hint empty">{t("common.empty")}</p>
    {/if}

    {#each rules as rule, i (i)}
      <div class="rule-row">
        <input
          type="checkbox"
          bind:group={selected}
          value={i}
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
        <div class="modes">
          {#each MODES as mode (mode.field)}
            <label class="mode" class:on={rule[mode.field]} title={t(mode.label)}>
              <input
                type="checkbox"
                checked={rule[mode.field]}
                aria-label={t(mode.label)}
                onchange={(e) => setMode(rule, mode.field, e.currentTarget.checked)}
              />
              <mode.icon width="13" height="13" />
            </label>
          {/each}
        </div>
      </div>
    {/each}
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
  }
  .rule-row:hover {
    background: var(--hover);
  }
  .rule-row input[type="checkbox"] {
    accent-color: var(--accent);
    flex: none;
    cursor: pointer;
  }
  .rule-row.builtin {
    color: var(--muted);
  }
  .lock {
    display: inline-flex;
    flex: none;
    width: 13px;
    color: var(--muted);
  }
  /* 与 .by 下拉同宽的占位，让内置行的开关列对齐 */
  .by-placeholder {
    flex: none;
    width: 56px;
  }
  .by {
    flex: none;
    width: 56px;
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
  /* 内置行的次文本是映像名列表，按正常方向排版 */
  .rpath.ltr {
    direction: ltr;
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
  .regex-input.broad {
    border-color: var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, var(--surface-2));
  }

  .modes {
    flex: none;
    display: flex;
    gap: 3px;
  }
  .mode {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 20px;
    border-radius: 5px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--muted);
    cursor: pointer;
  }
  .mode.on {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  .mode:hover:not(.on) {
    border-color: var(--accent);
    color: var(--text);
  }
  /* 勾选态由外层 label 的配色表达 */
  .mode input[type="checkbox"] {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }
  .mode:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  /* 内置行的开关是只读展示 */
  .rule-row.builtin .mode {
    cursor: not-allowed;
  }
  .rule-row.builtin .mode:hover:not(.on) {
    border-color: var(--border);
    color: var(--muted);
  }
</style>
