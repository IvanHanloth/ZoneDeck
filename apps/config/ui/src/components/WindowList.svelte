<script>
  import { onMount } from "svelte";
  // 标题栏样式对齐 WindowRuleList（隐藏窗口）。
  import IconAppWindow from "~icons/lucide/app-window";
  import IconSearch from "~icons/lucide/search";
  import IconX from "~icons/lucide/x";
  import IconRefresh from "~icons/lucide/refresh-cw";
  import IconChevronDown from "~icons/lucide/chevron-down";
  import { groupByProcess, splitByVisibility } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";
  import { NO_TITLE } from "../lib/grouping.js";
  import { app } from "../lib/state.svelte.js";

  let {
    title,
    windows,
    selected = $bindable([]),
    search = $bindable(""),
    showBackground = $bindable(false),
    showUntitled = $bindable(false),
    onrefresh,
  } = $props();

  let searchOpen = $state(false);
  let menuOpen = $state(false);
  let menuContainer;

  const parts = $derived(splitByVisibility(windows));

  function toggleSearch() {
    searchOpen = !searchOpen;
    if (!searchOpen) search = "";
  }

  onMount(() => {
    function handleClickOutside(e) {
      if (menuOpen && menuContainer && !menuContainer.contains(e.target)) {
        menuOpen = false;
      }
    }
    document.addEventListener("click", handleClickOutside);
    return () => document.removeEventListener("click", handleClickOutside);
  });
</script>

<div class="list-box">
  <div class="list-title">
    <span class="title-text"><IconAppWindow width="15" height="15" /> {title}</span>
    <span class="count">{windows.length}</span>
    <div class="tools">
      <button class="mini" title={t("windowList.refresh")} onclick={() => onrefresh?.()}>
        <IconRefresh width="14" height="14" />
      </button>
      <button
        class="mini"
        class:active={searchOpen}
        title={t("common.search")}
        aria-label={t("common.search")}
        onclick={toggleSearch}
      >
        <IconSearch width="14" height="14" />
      </button>
      <div class="menu-container" bind:this={menuContainer}>
        <button
          class="mini"
          class:active={menuOpen}
          onclick={() => menuOpen = !menuOpen}
          title={t("windowList.moreOptions")}
          aria-label={t("windowList.moreOptions")}
        >
          <IconChevronDown width="14" height="14" />
        </button>
        {#if menuOpen}
          <div class="dropdown-menu">
            <label class="menu-item">
              <input type="checkbox" bind:checked={showBackground} />
              <span>{t("windowList.backgroundProcesses")}</span>
            </label>
            <label class="menu-item">
              <input type="checkbox" bind:checked={showUntitled} />
              <span>{t("windowList.untitledWindows")}</span>
            </label>
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#if searchOpen}
    <div class="search">
      <IconSearch width="14" height="14" />
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        placeholder={t("windowList.searchPlaceholder", { title })}
        bind:value={search}
        spellcheck="false"
        autofocus
      />
      {#if search}
        <button class="clear" title={t("common.clear")} aria-label={t("common.clear")} onclick={() => (search = "")}>
          <IconX width="13" height="13" />
        </button>
      {/if}
    </div>
  {/if}

  <div class="win-list" role="listbox" aria-label={title}>
    {#if windows.length === 0}
      <p class="hint empty">{t("common.empty")}</p>
    {:else}
      {@render section(parts.visible, null)}
      {#if parts.hidden.length}
        {@render section(parts.hidden, t("windowList.hiddenSection"))}
      {/if}
    {/if}
  </div>
</div>

{#snippet section(list, label)}
  {#if list.length}
    {#if label}
      <div class="section-label">{label}<span>{list.length}</span></div>
    {/if}
    {#each groupByProcess(list) as group (group.process)}
      <div class="proc-group">
        <div class="proc-name">
          {#if app.icons[group.path]}
            <img class="proc-icon" src={app.icons[group.path]} alt="" />
          {:else}
            <span class="proc-icon fallback"><IconAppWindow width="14" height="14" /></span>
          {/if}
          <span class="name">{group.process}</span>
        </div>
        {#each group.windows as w (w.hwnd + "-" + w.process)}
          <label class="win-item" title={w.path}>
            <input type="checkbox" bind:group={selected} value={w.hwnd} />
            <span class="wtitle">{w.title === NO_TITLE ? t("common.noTitleWindow") : w.title}</span>
            <span class="meta">PID {w.PID}</span>
          </label>
        {/each}
      </div>
    {/each}
  {/if}
{/snippet}

<style>
  .list-box {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    flex: 1;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .list-title {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    font-weight: 600;
    font-size: 13px;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    flex: none;
    flex-wrap: wrap;
    row-gap: 6px;
  }
  .title-text {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--text);
  }
  .count {
    font-weight: 500;
    font-size: 12px;
    background: var(--hover);
    border-radius: 99px;
    padding: 1px 8px;
  }
  .tools {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .mini {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .mini:hover {
    background: var(--hover);
    border-color: var(--accent);
  }
  .mini.active {
    color: #fff;
    background: var(--accent);
    border-color: var(--accent);
  }

  .menu-container {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    padding: 4px 0;
    margin-top: 4px;
    min-width: 140px;
    z-index: 100;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    user-select: none;
    color: var(--text);
    font-size: 12px;
    transition: background 0.12s;
  }
  .menu-item:hover {
    background: var(--hover);
  }
  .menu-item input[type="checkbox"] {
    cursor: pointer;
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    flex: none;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--muted);
    flex: none;
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    border: none;
    background: none;
    padding: 0;
    color: var(--text);
  }
  .search input:focus {
    outline: none;
  }
  .clear {
    display: inline-flex;
    color: var(--muted);
  }
  .clear:hover {
    color: var(--text);
  }

  .win-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .empty {
    text-align: center;
    padding: 24px 0;
  }

  .section-label {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 8px 6px 4px;
    padding-top: 8px;
    border-top: 1px dashed var(--border);
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .section-label span {
    font-weight: 500;
    background: var(--hover);
    border-radius: 99px;
    padding: 0 6px;
  }

  .proc-group {
    margin-bottom: 6px;
  }
  .proc-name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    color: var(--muted);
    padding: 4px 6px;
    font-size: 12.5px;
  }
  .proc-icon {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .proc-icon.fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    opacity: 0.55;
  }
  .proc-name .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .win-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px 5px 20px;
    border-radius: 6px;
    cursor: pointer;
  }
  .win-item:hover {
    background: var(--hover);
  }
  .win-item input {
    accent-color: var(--accent);
    flex: none;
  }
  .win-item .wtitle {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .win-item .meta {
    color: var(--muted);
    font-size: 12px;
    flex: none;
  }
</style>
