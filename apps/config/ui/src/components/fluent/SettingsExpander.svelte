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

  // 主开关一开，把它管着的子设置露出来；关掉时不自动收起 ——
  // 子项会置灰留在原地，用户才看得见是什么把它们锁住了。
  // prevAuto 首轮是 undefined，所以初始就为 true 时不会强行展开。
  let prevAuto;
  $effect(() => {
    const on = autoExpand;
    if (on && prevAuto === false) open = true;
    prevAuto = on;
  });
</script>

<div class="expander" class:open>
  <div class="head" class:disabled>
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
    <button
      class="chev"
      aria-expanded={open}
      aria-label={t(open ? "common.collapse" : "common.expand")}
      title={t(open ? "common.collapse" : "common.expand")}
      onclick={() => (open = !open)}
    >
      <IconChevronDown width="14" height="14" />
    </button>
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

  .chev {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--r-control);
    color: var(--text);
    transition:
      background var(--dur-fast) var(--ease-standard),
      rotate var(--dur-slow) var(--ease-standard);
  }
  .chev:hover {
    background: var(--subtle-hover);
  }
  .chev:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
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
</style>
