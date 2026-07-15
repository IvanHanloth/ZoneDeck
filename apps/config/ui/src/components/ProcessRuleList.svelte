<script>
  // 「隐藏进程」规则（粗粒度）：按可执行文件路径隐藏该程序的所有窗口，而非单个窗口。
  import IconPlus from "~icons/lucide/plus";
  import IconTrash from "~icons/lucide/trash-2";
  import IconRegex from "~icons/lucide/regex";
  import IconBox from "~icons/lucide/box";
  import ScopeSelect from "./ScopeSelect.svelte";
  import { isRegexRule } from "../lib/grouping.js";
  import { app } from "../lib/state.svelte.js";

  let { rules = $bindable([]), onadd, onaddregex } = $props();
  let selected = $state([]);

  function remove() {
    if (!selected.length) return;
    rules = rules.filter((_, i) => !selected.includes(i));
    selected = [];
  }
</script>

<div class="list-box">
  <div class="list-title">
    <span class="title-text"><IconBox width="15" height="15" /> 隐藏进程</span>
    <span class="count">{rules.length}</span>
    <div class="tools">
      <button class="mini primary" title="把左侧选中窗口所属进程加入" onclick={() => onadd?.()}>
        <IconPlus width="14" height="14" /> 添加进程
      </button>
      <button class="mini" title="添加进程正则规则" onclick={() => onaddregex?.()}>
        <IconRegex width="14" height="14" /> 正则
      </button>
      <button class="mini" title="移除选中规则" onclick={remove} disabled={!selected.length}>
        <IconTrash width="14" height="14" /> 移除
      </button>
    </div>
  </div>

  <div class="rule-list" role="listbox" aria-label="隐藏进程规则">
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
            <span class="regex-tag">
              <IconRegex width="12" height="12" />
              {rule.by_name ? "文件名正则" : "路径正则"}
            </span>
            <input
              class="regex-input"
              placeholder={rule.by_name ? "文件名正则，如 .*WeChat\\.exe" : "路径正则，如 .*WeChat.*"}
              bind:value={rule.regex}
              spellcheck="false"
            />
          {:else}
            {#if app.icons[rule.path]}
              <img class="ic" src={app.icons[rule.path]} alt="" />
            {:else}
              <span class="ic fallback"><IconBox width="14" height="14" /></span>
            {/if}
            <span class="rproc">{rule.process || "（未知进程）"}</span>
            <span class="rpath" title={rule.path}>
              {rule.by_name ? "（任意目录下的同名程序）" : rule.path}
            </span>
          {/if}

          <select
            class="by"
            bind:value={rule.by_name}
            title="匹配依据：完整路径，或只看可执行文件名（同名程序在任意目录都命中）"
            aria-label="匹配依据"
          >
            <option value={false}>路径</option>
            <option value={true}>文件名</option>
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
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }
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
</style>
