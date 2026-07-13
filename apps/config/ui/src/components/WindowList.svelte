<script>
  // 可勾选的窗口列表（按进程分组，含图标）。selected 双向绑定 hwnd 数组。
  import { groupByProcess } from "../lib/grouping.js";
  import { app } from "../lib/state.svelte.js";

  let { title, windows, selected = $bindable([]) } = $props();

  const groups = $derived(groupByProcess(windows));
</script>

<div class="list-box">
  <div class="list-title">
    {title}
    <span class="count">{windows.length}</span>
  </div>
  <div class="win-list" role="listbox" aria-label={title}>
    {#if groups.length === 0}
      <p class="hint empty">（空）</p>
    {:else}
      {#each groups as group (group.process)}
        <div class="proc-group">
          <div class="proc-name">
            {#if app.icons[group.path]}
              <img class="proc-icon" src={app.icons[group.path]} alt="" />
            {:else}
              <span class="proc-icon fallback">▣</span>
            {/if}
            <span class="name">{group.process}</span>
          </div>
          {#each group.windows as w (w.hwnd + "-" + w.process)}
            <label class="win-item" title={w.path}>
              <input type="checkbox" bind:group={selected} value={w.hwnd} />
              <span class="title">{w.title}</span>
              <span class="meta">PID {w.PID}</span>
            </label>
          {/each}
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
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .list-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    font-weight: 600;
    font-size: 13px;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    flex: none;
  }
  .count {
    font-weight: 500;
    font-size: 12px;
    background: var(--hover);
    border-radius: 99px;
    padding: 1px 8px;
  }

  .win-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .empty {
    text-align: center;
    padding: 24px 0;
  }

  .proc-group {
    margin-bottom: 6px;
  }
  .proc-name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    color: var(--muted);
    padding: 4px 6px;
    font-size: 12.5px;
  }
  .proc-icon {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .proc-icon.fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    opacity: 0.55;
  }
  .proc-name .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .win-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px 5px 20px;
    border-radius: 6px;
    cursor: pointer;
  }
  .win-item:hover {
    background: var(--hover);
  }
  .win-item input {
    accent-color: var(--accent);
    flex: none;
  }
  .win-item .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .win-item .meta {
    color: var(--muted);
    font-size: 12px;
    flex: none;
  }
</style>
