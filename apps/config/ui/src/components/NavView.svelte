<script>
  // Win11 NavigationView：图标 + 文字，选中项左侧一条 accent 竖条。
  // 窄窗口自动折叠成图标条，点汉堡可手动覆盖。
  import IconMenu from "~icons/lucide/menu";
  import { t } from "../lib/i18n.svelte.js";

  let {
    items = [],
    active = $bindable(""),
    collapsed = false,
    ariaLabel = "",
    ontoggle,
  } = $props();

  const main = $derived(items.filter((i) => !i.footer));
  const foot = $derived(items.filter((i) => i.footer));
</script>

{#snippet navItem(item)}
  {@const Icon = item.icon}
  {@const sel = active === item.id}
  <button
    class="item"
    class:sel
    title={collapsed ? t(item.labelKey) : null}
    aria-current={sel ? "page" : null}
    onclick={() => (active = item.id)}
  >
    <span class="pill" aria-hidden="true"></span>
    <span class="ico"><Icon width="16" height="16" /></span>
    <span class="lbl">{t(item.labelKey)}</span>
  </button>
{/snippet}

<nav class="nav" class:collapsed aria-label={ariaLabel}>
  <button
    class="toggle"
    title={t(collapsed ? "nav.expand" : "nav.collapse")}
    aria-label={t(collapsed ? "nav.expand" : "nav.collapse")}
    aria-expanded={!collapsed}
    onclick={() => ontoggle?.()}
  >
    <IconMenu width="16" height="16" />
  </button>

  <div class="list">
    {#each main as item (item.id)}{@render navItem(item)}{/each}
  </div>

  {#if foot.length}
    <div class="list foot">
      {#each foot as item (item.id)}{@render navItem(item)}{/each}
    </div>
  {/if}
</nav>

<style>
  .nav {
    flex: none;
    width: var(--nav-w);
    display: flex;
    flex-direction: column;
    padding-bottom: 4px;
    overflow: hidden;
    transition: width var(--dur-slow) var(--ease-standard);
  }
  .nav.collapsed {
    width: var(--nav-w-collapsed);
  }

  .toggle {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 36px;
    margin: 0 0 8px 4px;
    border-radius: var(--r-control);
    color: var(--text);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .toggle:hover {
    background: var(--subtle-hover);
  }
  .toggle:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 4px;
    min-height: 0;
    overflow-y: auto;
  }
  .list.foot {
    margin-top: auto;
    padding-top: 8px;
    overflow: visible;
  }

  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    height: 36px;
    padding: 0 12px;
    border-radius: var(--r-control);
    color: var(--text);
    white-space: nowrap;
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .item:hover {
    background: var(--subtle-hover);
  }
  .item:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
  }
  .item.sel {
    background: var(--subtle-hover);
  }
  /* 选中项的 hover / 按下反馈要压过选中底色，否则指到哪一项看不出来 */
  .item.sel:hover {
    background: var(--subtle-selected-hover);
  }
  .item.sel:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
  }

  /* 选中指示条：从中心纵向长出 */
  .pill {
    position: absolute;
    left: 0;
    top: 50%;
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: var(--accent);
    translate: 0 -50%;
    scale: 1 0;
    transition: scale var(--dur-slow) var(--ease-standard);
  }
  .item.sel .pill {
    scale: 1 1;
  }

  .ico {
    flex: none;
    display: inline-flex;
    color: var(--text-2);
  }
  .item.sel .ico {
    color: var(--text);
  }
  .lbl {
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 1;
    transition: opacity var(--dur-normal) var(--ease-standard);
  }
  .nav.collapsed .lbl {
    opacity: 0;
  }
  .nav.collapsed .item {
    padding: 0 10px;
  }
</style>
