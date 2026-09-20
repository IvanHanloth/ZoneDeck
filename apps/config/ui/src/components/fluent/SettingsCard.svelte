<script>
  // Win11 SettingsCard：一项一卡。icon 传 unplugin-icons 组件本身。
  // variant="sub" 用于 Expander 内的子行：无独立卡面，靠分隔线区分。
  // 传了 onclick 整张卡就是点击热区（同 Win11 的可点击卡片）；外链再加 external，
  // 右侧出现外链图标，告诉用户这一下会离开程序。
  import IconExternalLink from "~icons/lucide/external-link";

  let {
    icon: Icon = null,
    iconColor = "",
    label,
    description = "",
    control,
    disabled = false,
    variant = "card",
      onclick = null,
      external = false,
  } = $props();
</script>

<!-- data-setting 供顶部搜索定位到具体某一项，标题即锚点，无需逐个登记 id -->
<div class="card {variant}" class:disabled data-setting={label}>
    <!-- 同 SettingsExpander：按钮铺满整卡垫在内容底下，正文与外链图标让出点击，
         右侧控件浮在上面保持可操作。 -->
    {#if onclick}
        <button
                class="hit"
                aria-label={[label, description].filter(Boolean).join(" ")}
                {disabled}
                {onclick}
        ></button>
    {/if}
    <!-- 可点击时正文已由按钮的 aria-label 完整念出，留在无障碍树里只会重复一遍 -->
    <div aria-hidden={onclick ? "true" : null} class="main">
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
    {#if external}
        <span class="ext" aria-hidden="true"><IconExternalLink width="16" height="16"/></span>
    {/if}
</div>

<style>
  /* Win11 SettingsCard 规格：无副标题 52，有副标题 68，左右 16。
     控件列排不下时整列换到标题下一行，同 WinUI SettingsCard 的自适应 */
  .card {
      position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 20px;
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
  /* Expander 子行：靠分隔线分隔，不再画卡面；缩进对齐父项标题与控件列 */
  .card.sub {
    padding: 10px var(--sub-pad-end) 10px var(--sub-pad-start);
    border-top: 1px solid var(--divider);
  }

  /* 置灰只淡化图标与文字，控件禁用态交由控件自身处理 */
  .card.disabled .icon,
  .card.disabled .label,
  .card.disabled .desc {
    color: var(--text-disabled);
  }

  /* 整卡热区。圆角跟着卡面，否则悬停底色会溢出卡片的圆角 */
  .hit {
      position: absolute;
      inset: 0;
      border-radius: inherit;
      transition: background var(--dur-fast) var(--ease-standard);
  }

  .card:hover .hit:not(:disabled) {
      background: var(--subtle-hover);
  }

  .hit:active:not(:disabled) {
      background: var(--subtle-pressed);
  }

  .hit:focus-visible {
      outline-offset: -3px;
  }

  /* 垫在 .hit 之上只为盖住它的底色，点击一律穿透回 .hit */
  .card:has(.hit) .main,
  .ext {
      position: relative;
      pointer-events: none;
  }

  /* 控件同样要浮在 .hit 之上，但保持可操作 */
  .card:has(.hit) .control {
      position: relative;
  }

  .ext {
      flex: none;
      display: inline-flex;
      color: var(--text-2);
  }

  .card.disabled .ext {
      color: var(--text-disabled);
  }

  /* 标题列的下限：排不下时先让控件列换行，而不是一路把标题压没 */
  .main {
    display: flex;
    align-items: center;
    gap: 20px;
    flex: 1;
    min-width: 160px;
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
  /* 换行后仍靠右；整列比一行还宽时允许收缩，内部按钮跟着换行 */
  .control {
    flex: 0 1 auto;
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    margin-inline-start: auto;
  }
</style>
