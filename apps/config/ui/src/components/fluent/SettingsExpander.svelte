<script>
  // Win11 SettingsExpander：头部同 SettingsCard，展开区收纳依赖于本项的子设置。
  // 展开动画走 grid-template-rows 0fr→1fr，不必预先知道内容高度。
  import IconChevronDown from "~icons/lucide/chevron-down";
  import { t } from "../../lib/i18n.svelte.js";

  let {
    icon: Icon = null,
    iconColor = "",
    label,
    description = "",
    control,
    children,
    open = $bindable(false),
    autoExpand = undefined,
    disabled = false,
  } = $props();

  // 主开关一开，把它管着的子设置露出来；关掉时不自动收起，子项置灰留在原地。
  // prevAuto 首轮是 undefined，初始就为 true 时不会强行展开。
  let prevAuto;
  $effect(() => {
    const on = autoExpand;
    if (on && prevAuto === false) open = true;
    prevAuto = on;
  });
</script>

<div class="expander" class:open data-setting={label}>
  <div class="head" class:disabled>
    <!-- 整条头部都是展开热区（同 Win11），按钮铺满后垫在内容底下：
         正文与箭头改为 pointer-events:none 把点击让给它，只有右侧控件浮在上面
         保持可操作。 -->
    <button
      class="hit"
      aria-expanded={open}
      aria-label={t(open ? "common.collapse" : "common.expand")}
      onclick={() => (open = !open)}
    ></button>
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
    <span class="chev" aria-hidden="true">
      <IconChevronDown width="14" height="14" />
    </span>
  </div>

  <div class="wrap" aria-hidden={!open}>
    <div class="body">{@render children?.()}</div>
  </div>
</div>

<style>
  .expander {
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
    overflow: hidden;
  }

  .head {
    position: relative;
    display: flex;
    align-items: center;
    gap: 20px;
    min-height: 52px;
    padding: 10px 10px 10px 16px;
  }
  .head:has(.desc) {
    min-height: 68px;
  }
  .head.disabled .icon,
  .head.disabled .label,
  .head.disabled .desc {
    color: var(--text-disabled);
  }

  .hit {
    position: absolute;
    inset: 0;
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .head:hover .hit {
    background: var(--subtle-hover);
  }
  .hit:active {
    background: var(--subtle-pressed);
  }
  .hit:focus-visible {
    outline-offset: -3px;
  }

  /* 垫在 .hit 之上只为盖住它的底色，点击一律穿透回 .hit */
  .main,
  .chev {
    position: relative;
    pointer-events: none;
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
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .chev {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    color: var(--text);
    transition: rotate var(--dur-slow) var(--ease-standard);
  }
  .expander.open .chev {
    rotate: 180deg;
  }

  .wrap {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-slow) var(--ease-standard);
  }
  .expander.open .wrap {
    grid-template-rows: 1fr;
  }
  .body {
    overflow: hidden;
    background: var(--card-2);
  }
  /* 展开区里非卡片的内容（如 InfoBar）跟着子项一起缩进 */
  .body > :global(:not(.card)) {
    margin: 12px var(--sub-pad-end) 12px var(--sub-pad-start);
  }
</style>
