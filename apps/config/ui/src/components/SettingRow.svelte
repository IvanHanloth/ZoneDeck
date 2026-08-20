<script>
  // icon 传 unplugin-icons 组件本身；iconColor 留空时走 --muted。
  let {
    icon: Icon = null,
    iconColor = "",
    label,
    description = "",
    control,
    disabled = false,
  } = $props();
</script>

<div class="row" class:disabled>
  <div class="main">
    {#if Icon}
      <span class="icon" style:color={iconColor || null} aria-hidden="true">
        <Icon width="17" height="17" />
      </span>
    {/if}
    <div class="text">
      <div class="label">{label}</div>
      {#if description}<div class="desc">{description}</div>{/if}
    </div>
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
  /* 置灰：淡化图标、标题与描述，禁用态交由控件自身处理。 */
  .row.disabled .icon,
  .row.disabled .label,
  .row.disabled .desc {
    opacity: 0.45;
  }
  /* 图标与文字同属一块，描述换行时图标仍跟标题那一行对齐。 */
  .main {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }
  .icon {
    flex: none;
    display: inline-flex;
    margin-top: 1px;
    color: var(--muted);
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
