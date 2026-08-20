<script>
  // Win11 InfoBar：替代裸奔的说明段落，按语义色分四态。
  import IconInfo from "~icons/lucide/info";
  import IconOk from "~icons/lucide/circle-check";
  import IconWarn from "~icons/lucide/triangle-alert";
  import IconError from "~icons/lucide/circle-x";

  let {
    severity = "informational",
    title = "",
    disabled = false,
    children,
  } = $props();

  const ICONS = {
    informational: IconInfo,
    success: IconOk,
    warning: IconWarn,
    error: IconError,
  };
  const Icon = $derived(ICONS[severity] ?? IconInfo);
</script>

<div class="bar {severity}" class:disabled role="status">
  <span class="icon" aria-hidden="true"><Icon width="16" height="16" /></span>
  <div class="body">
    {#if title}<div class="title">{title}</div>{/if}
    <div class="msg">{@render children?.()}</div>
  </div>
</div>

<style>
  .bar {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--r-card);
    border: 1px solid var(--stroke);
    background: color-mix(in srgb, var(--tint) 12%, var(--card));
  }
  .bar.informational {
    --tint: var(--accent);
  }
  .bar.success {
    --tint: var(--ok);
  }
  .bar.warning {
    --tint: var(--warn);
  }
  .bar.error {
    --tint: var(--danger);
  }
  .bar.disabled {
    opacity: 0.45;
  }

  .icon {
    flex: none;
    display: inline-flex;
    margin-top: 1px;
    color: var(--tint);
  }
  .body {
    min-width: 0;
    flex: 1;
  }
  .title {
    font-weight: 600;
    line-height: 20px;
  }
  .msg {
    font-size: 12px;
    line-height: 18px;
    color: var(--text-2);
  }
  .title + .msg {
    margin-top: 2px;
  }
</style>
