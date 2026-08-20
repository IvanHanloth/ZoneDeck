<script>
  // Win11 ContentDialog：遮罩 + 居中对话框，底部按钮区自成一条。
  import { t } from "../../lib/i18n.svelte.js";
  import IconX from "~icons/lucide/x";

  let {
    title = "",
    open = $bindable(false),
    closable = true,
    children,
    footer,
  } = $props();

  function close() {
    if (closable) open = false;
  }
  function onKey(e) {
    if (e.key === "Escape") close();
  }
  // 仅点击遮罩本身时关闭。
  function onOverlayClick(e) {
    if (e.target === e.currentTarget) close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="overlay" onclick={onOverlayClick} role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1">
      <header class="head">
        <h3 class="type-subtitle">{title}</h3>
        {#if closable}
          <button
            class="x"
            title={t("common.close")}
            aria-label={t("common.close")}
            onclick={close}
          >
            <IconX width="14" height="14" />
          </button>
        {/if}
      </header>
      <div class="body">{@render children?.()}</div>
      {#if footer}
        <footer class="foot">{@render footer()}</footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 24px;
    animation: fade var(--dur-normal) var(--ease-standard);
  }
  /* 对话框底必须不透明：Win11 的 ContentDialog 走 SolidBackgroundFillColorBase，
     用半透明的层次色会把底下的页面透出来，正文根本读不清 */
  .dialog {
    width: min(640px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--solid-bg);
    border: 1px solid var(--stroke);
    border-radius: var(--r-overlay);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
    animation: dialog-in var(--dur-slow) var(--ease-standard);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 24px 24px 12px;
    flex: none;
  }
  .head h3 {
    min-width: 0;
  }
  .x {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    margin: -4px -8px 0 0;
    border-radius: var(--r-control);
    color: var(--text-2);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .x:hover {
    background: var(--subtle-hover);
    color: var(--text);
  }
  .x:active {
    background: var(--subtle-pressed);
  }

  .body {
    padding: 0 24px 24px;
    overflow-y: auto;
    min-height: 0;
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 24px;
    border-top: 1px solid var(--divider);
    background: var(--card-2);
    flex: none;
  }
  /* 不强制等宽：恢复工具那种五按钮的底栏会被拉爆 */
  .foot :global(.btn) {
    min-width: 100px;
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes dialog-in {
    from {
      opacity: 0;
      scale: 1.05;
    }
  }
</style>
