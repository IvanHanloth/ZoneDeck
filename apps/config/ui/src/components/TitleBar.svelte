<script>
  import { win } from "../lib/ipc.js";
  import { app } from "../lib/state.svelte.js";
</script>

<header
  class="titlebar"
  data-tauri-drag-region
  role="presentation"
  ondblclick={() => win.toggleMaximize()}
>
  <div class="brand" data-tauri-drag-region>
    <img class="logo" src="/icon.ico" alt="" data-tauri-drag-region />
    <span class="name" data-tauri-drag-region>Boss Key</span>
  </div>

  <div class="controls">
    <button class="tb-btn" title="最小化" aria-label="最小化" onclick={() => win.minimize()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 5h10" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button class="tb-btn" title={app.maximized ? "还原" : "最大化"} aria-label={app.maximized ? "还原" : "最大化"} onclick={() => win.toggleMaximize()}>
      {#if app.maximized}
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2">
          <rect x="0.6" y="2.6" width="6.8" height="6.8" rx="1" />
          <path d="M2.6 2.6V1.6a1 1 0 0 1 1-1h4.8a1 1 0 0 1 1 1v4.8a1 1 0 0 1-1 1h-1" />
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2">
          <rect x="0.6" y="0.6" width="8.8" height="8.8" rx="1" />
        </svg>
      {/if}
    </button>
    <button class="tb-btn close" title="关闭" aria-label="关闭" onclick={() => win.close()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-h);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 12px;
  }
  .logo {
    width: 18px;
    height: 18px;
    flex: none;
  }
  .name {
    font-weight: 600;
  }

  .controls {
    display: flex;
    height: 100%;
    flex: none;
  }
  .tb-btn {
    width: 46px;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    transition: background 0.12s, color 0.12s;
  }
  .tb-btn:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tb-btn.close:hover {
    background: var(--danger);
    color: #fff;
  }
  .tb-btn:focus-visible {
    outline-offset: -2px;
  }
</style>
