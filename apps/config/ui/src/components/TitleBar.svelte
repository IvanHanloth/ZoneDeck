<script>
  import { t } from "../lib/i18n.svelte.js";
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
    <img class="logo" src="/logo.svg" alt="" data-tauri-drag-region />
    <span class="name" data-tauri-drag-region>ZoneDeck</span>
  </div>

  <div class="controls">
    <button
      class="tb-btn"
      title={t("titlebar.minimize")}
      aria-label={t("titlebar.minimize")}
      onclick={() => win.minimize()}
    >
      <svg width="10" height="10" viewBox="0 0 10 10">
        <path d="M0 5h10" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
    <button
      class="tb-btn"
      title={app.maximized ? t("titlebar.restore") : t("titlebar.maximize")}
      aria-label={app.maximized ? t("titlebar.restore") : t("titlebar.maximize")}
      onclick={() => win.toggleMaximize()}
    >
      {#if app.maximized}
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="0.5" y="2.5" width="7" height="7" />
          <path d="M2.5 2.5V0.5h7v7h-2" />
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="0.5" y="0.5" width="9" height="9" />
        </svg>
      {/if}
    </button>
    <button
      class="tb-btn close"
      title={t("titlebar.close")}
      aria-label={t("titlebar.close")}
      onclick={() => win.close()}
    >
      <svg width="10" height="10" viewBox="0 0 10 10">
        <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-h);
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    flex: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 16px;
  }
  .logo {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .name {
    font-size: 12px;
    line-height: 16px;
  }

  .controls {
    display: flex;
    flex: none;
  }
  /* Win11 规格：46 宽、满标题栏高、10px 图标 */
  .tb-btn {
    width: 46px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text);
    transition:
      background var(--dur-fast) var(--ease-standard),
      color var(--dur-fast) var(--ease-standard);
  }
  .tb-btn:hover {
    background: var(--subtle-hover);
  }
  .tb-btn:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
  }
  /* 关闭键的红是 Windows 外壳的固定规范色，不走主题令牌 */
  .tb-btn.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .tb-btn.close:active {
    background: #b2271a;
    color: rgba(255, 255, 255, 0.7);
  }
  .tb-btn:focus-visible {
    outline-offset: -3px;
  }
</style>
