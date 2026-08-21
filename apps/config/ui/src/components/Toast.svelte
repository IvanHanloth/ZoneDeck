<script>
  import IconInfo from "~icons/lucide/info";
  import IconError from "~icons/lucide/circle-x";
  import { toastState } from "../lib/state.svelte.js";
</script>

<div
  class="toast"
  class:show={toastState.visible}
  class:error={toastState.error}
  role="status"
  aria-live="polite"
>
  <span class="icon" aria-hidden="true">
    {#if toastState.error}
      <IconError width="16" height="16" />
    {:else}
      <IconInfo width="16" height="16" />
    {/if}
  </span>
  <span class="msg">{toastState.message}</span>
</div>

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 56px;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(80vw, 480px);
    padding: 10px 16px;
    border-radius: var(--r-overlay);
    border: 1px solid var(--stroke);
    background: color-mix(in srgb, var(--tint) 12%, var(--flyout-solid));
    box-shadow: var(--shadow-flyout);
    color: var(--text);
    font-size: 13px;
    line-height: 18px;
    opacity: 0;
    pointer-events: none;
    translate: -50% 12px;
    transition:
      opacity var(--dur-normal) var(--ease-standard),
      translate var(--dur-normal) var(--ease-out);
    z-index: 900;
    --tint: var(--accent);
  }
  .toast.show {
    opacity: 1;
    translate: -50% 0;
  }
  .toast.error {
    --tint: var(--danger);
  }

  .icon {
    flex: none;
    display: inline-flex;
    color: var(--tint);
  }
  .msg {
    min-width: 0;
  }
</style>
