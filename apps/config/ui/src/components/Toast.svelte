<script>
  import { toastState } from "../lib/state.svelte.js";
</script>

<div
  class="toast"
  class:show={toastState.visible}
  class:error={toastState.error}
  role="status"
  aria-live="polite"
>
  {toastState.message}
</div>

<style>
  .toast {
    position: fixed;
    left: 50%;
    bottom: 64px;
    transform: translate(-50%, 12px);
    max-width: min(80vw, 480px);
    padding: 9px 18px;
    border-radius: 8px;
    background: var(--text);
    color: var(--bg);
    font-size: 13px;
    box-shadow: var(--shadow);
    opacity: 0;
    pointer-events: none;
    transition:
      opacity 0.2s,
      transform 0.2s;
    z-index: 900;
  }
  .toast.show {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  .toast.error {
    background: var(--danger);
    color: #fff;
  }
</style>
