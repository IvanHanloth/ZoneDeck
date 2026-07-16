<script>
  let { label, description = "", control, disabled = false } = $props();
</script>

<div class="row" class:disabled>
  <div class="text">
    <div class="label">{label}</div>
    {#if description}<div class="desc">{description}</div>{/if}
  </div>
  <div class="control">{@render control?.()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: none;
  }
  /* 置灰：淡化标题与描述，交由控件自身处理禁用态。 */
  .row.disabled .label,
  .row.disabled .desc {
    opacity: 0.45;
  }
  .text {
    min-width: 0;
  }
  .label {
    font-size: 13.5px;
  }
  .desc {
    margin-top: 3px;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .control {
    flex: none;
  }
</style>
