<script>
  // 分组：小标题排在卡片外（Win11 设置的分组样式），下接一摞卡片。
  let { title = "", children } = $props();
</script>

<section class="group">
  {#if title}<h3 class="type-body-strong">{title}</h3>{/if}
  <div class="stack">{@render children?.()}</div>
</section>

<style>
  .group {
    display: flex;
    flex-direction: column;
  }
  h3 {
    padding: 0 1px 8px;
    font-size: 14px;
    line-height: 20px;
    font-weight: 600;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
