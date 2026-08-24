<script>
  // Win11 SettingsCard：一项一卡。icon 传 unplugin-icons 组件本身。
  // variant="sub" 用于 Expander 内的子行：无独立卡面，靠分隔线区分。
  let {
    icon: Icon = null,
    iconColor = "",
    label,
    description = "",
    control,
    disabled = false,
    variant = "card",
  } = $props();
</script>

<!-- data-setting 供顶部搜索定位到具体某一项，标题即锚点，无需逐个登记 id -->
<div class="card {variant}" class:disabled data-setting={label}>
  <div class="main">
    {#if Icon}
      <span class="icon" style:color={iconColor || null} aria-hidden="true">
        <Icon width="20" height="20" />
      </span>
    {/if}
    <div class="text">
      <div class="label">{label}</div>
      {#if description}<div class="desc">{description}</div>{/if}
    </div>
  </div>
  {#if control}<div class="control">{@render control()}</div>{/if}
</div>

<style>
  /* Win11 SettingsCard 规格：无副标题 52，有副标题 68，左右 16 */
  .card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    min-height: 52px;
    padding: 10px 16px;
  }
  .card:has(.desc) {
    min-height: 68px;
  }
  .card.card {
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
  }
  /* Expander 子行：靠分隔线分隔，不再画卡面 */
  .card.sub {
    padding: 10px 16px 10px 20px;
    border-top: 1px solid var(--divider);
  }

  /* 置灰只淡化图标与文字，控件禁用态交由控件自身处理 */
  .card.disabled .icon,
  .card.disabled .label,
  .card.disabled .desc {
    color: var(--text-disabled);
  }

  .main {
    display: flex;
    align-items: center;
    gap: 20px;
    flex: 1;
    min-width: 0;
  }
  .icon {
    flex: none;
    display: inline-flex;
    width: 20px;
    color: var(--text-2);
  }
  .text {
    min-width: 0;
  }
  .label {
    line-height: 20px;
  }
  .desc {
    margin-top: 2px;
    font-size: 12px;
    line-height: 16px;
    color: var(--text-2);
  }
  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }
</style>
