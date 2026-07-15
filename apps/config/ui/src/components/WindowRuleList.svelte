<script>
  // 「隐藏窗口」规则（细粒度）：精确规则显示标题 + 进程 + 追溯状态。
  import IconPlus from "~icons/lucide/plus";
  import IconTrash from "~icons/lucide/trash-2";
  import IconRegex from "~icons/lucide/regex";
  import IconAppWindow from "~icons/lucide/app-window";
  import ScopeSelect from "./ScopeSelect.svelte";
  import { isRegexRule, traceWindowRule } from "../lib/grouping.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  const STATUS = {
    reacquired: { text: "已重启·已追溯", cls: "warn" },
    missing: { text: "未找到 / 已关闭", cls: "danger" },
  };

  function remove() {
    if (!selected.length) return;
    rules = rules.filter((_, i) => !selected.includes(i));
    selected = [];
  }
</script>

<div class="list-box">
  <div class="list-title">
    <span class="title-text"><IconAppWindow width="15" height="15" /> 隐藏窗口</span>
    <span class="count">{rules.length}</span>
    <div class="tools">
      <button class="mini primary" title="把左侧选中窗口加入" onclick={() => onadd?.()}>
        <IconPlus width="14" height="14" /> 添加窗口
      </button>
      <button class="mini" title="添加标题正则规则" onclick={() => onaddregex?.()}>
        <IconRegex width="14" height="14" /> 正则
      </button>
      <button class="mini" title="移除选中规则" onclick={remove} disabled={!selected.length}>
        <IconTrash width="14" height="14" /> 移除
      </button>
    </div>
  </div>

  <div class="rule-list" role="listbox" aria-label="隐藏窗口规则">
    {#if rules.length === 0}
      <p class="hint empty">（空）</p>
    {:else}
      {#each rules as rule, i (i)}
        <div class="rule-row">
          <input
            type="checkbox"
            bind:group={selected}
            value={i}
            aria-label="选中该规则"
          />
          {#if isRegexRule(rule)}
            <span class="regex-tag"><IconRegex width="12" height="12" /> 标题正则</span>
            <input
              class="regex-input"
              placeholder="标题正则，如 .*微信.*"
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
            <span class="rtitle" title={rule.path}>{rule.title}</span>
            <span class="rproc">{rule.process}</span>
            {#if STATUS[st]}
              <span class="badge {STATUS[st].cls}">{STATUS[st].text}</span>
            {:else}
              <span class="dot live" title="运行中"></span>
            {/if}
          {/if}
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
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }
  /* 选择器要压过 .mini:hover:not(:disabled)，否则主按钮悬浮时会退回半透明的 --hover 底色，
     白字落在近白的底上直接糊掉。 */
  .mini.primary:hover:not(:disabled) {
    color: #fff;
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
    color: var(--muted);
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
  .badge {
    flex: none;
    font-size: 11px;
    padding: 1px 7px;
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
