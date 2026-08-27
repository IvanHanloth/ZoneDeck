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
  import CheckBox from "./fluent/CheckBox.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import { isRegexRule } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  // 行内的正则输入框、匹配方式下拉与三个模式开关自己处理点击，不参与选中
  const CONTROLS = "input, select, button, textarea, label, a, [role='group']";

  // 三个模式开关。文案键须为字面量，供 scripts/i18n-check.ps1 静态检查。
  const MODES = [
    { field: "ignore_hide", icon: IconEyeOff, label: "whitelist.ignoreHide" },
    { field: "ignore_freeze", icon: IconSnowflake, label: "whitelist.ignoreFreeze" },
    { field: "ignore_mute", icon: IconVolumeOff, label: "whitelist.ignoreMute" },
  ];

  const byOptions = $derived([
    { value: false, label: t("processRules.byPath") },
    { value: true, label: t("processRules.byName") },
  ]);

  // 三个开关相互独立，互不联动。
  function setMode(rule, field, on) {
    rule[field] = on;
  }

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

  <div
    class="lv-body"
    role="listbox"
    aria-multiselectable="true"
    aria-label={t("whitelist.aria")}
  >
    <!-- 内置项排在最前，不可编辑、不可删除。 -->
    {#each app.whitelistBuiltins as builtin (builtin.key)}
      <div class="lv-row rule-row builtin">
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
      <p class="hint lv-empty">{t("common.empty")}</p>
    {/if}

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
  .rule-row {
    cursor: default;
  }
  .rule-row:focus-visible {
    outline: 2px solid var(--focus-outer);
    outline-offset: -2px;
  }
  .rule-row.builtin {
    color: var(--text-2);
  }
  .lock {
    display: inline-flex;
    flex: none;
    width: 13px;
    color: var(--text-3);
  }
  /* 与匹配方式下拉同宽的占位，让内置行的开关列对齐 */
  .by-placeholder {
    flex: none;
    width: 72px;
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
  .regex-input.broad {
    border-color: var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, var(--control));
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
    width: 24px;
    height: 22px;
    border-radius: var(--r-control);
    border: 1px solid var(--stroke);
    border-bottom-color: var(--stroke-strong);
    background: var(--control);
    color: var(--text-2);
    transition:
      background var(--dur-fast) var(--ease-standard),
      color var(--dur-fast) var(--ease-standard);
  }
  .mode.on {
    color: var(--on-accent);
    background: var(--accent);
    border-color: transparent;
  }
  .mode:hover:not(.on) {
    background: var(--control-hover);
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
    outline: 2px solid var(--focus-outer);
    outline-offset: 2px;
    box-shadow: 0 0 0 1px var(--focus-inner);
  }
  /* 内置行的开关是只读展示 */
  .rule-row.builtin .mode {
    cursor: not-allowed;
  }
  .rule-row.builtin .mode:hover:not(.on) {
    background: var(--control);
    color: var(--text-2);
  }
</style>
