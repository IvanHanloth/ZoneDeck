<script>
  import IconX from "~icons/lucide/x";

  let { title = "", open = $bindable(false), children, footer } = $props();

  function close() {
    open = false;
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
      <header class="dhead">
        <h3>{title}</h3>
        <button class="x" title="关闭" aria-label="关闭" onclick={close}>
          <IconX width="15" height="15" />
        </button>
      </header>
      <div class="dbody">{@render children?.()}</div>
      {#if footer}
        <footer class="dfoot">{@render footer?.()}</footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 24px;
  }
  .dialog {
    width: min(640px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .dhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .dhead h3 {
    font-size: 14px;
    font-weight: 600;
  }
  .x {
    color: var(--muted);
    font-size: 13px;
    padding: 2px 6px;
    border-radius: 6px;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  .dbody {
    padding: 14px 16px;
    overflow-y: auto;
    min-height: 0;
  }
  .dfoot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
</style>
