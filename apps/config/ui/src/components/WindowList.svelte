<script>
  // 标题栏样式对齐 WindowRuleList。
  import IconAppWindow from "~icons/lucide/app-window";
  import IconSearch from "~icons/lucide/search";
  import IconX from "~icons/lucide/x";
  import IconRefresh from "~icons/lucide/refresh-cw";
  import IconChevronDown from "~icons/lucide/chevron-down";
  import CheckBox from "./fluent/CheckBox.svelte";
  import Flyout from "./fluent/Flyout.svelte";
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
  let menuBtn = $state(null);

  const parts = $derived(splitByVisibility(windows));

  function toggleSearch() {
    searchOpen = !searchOpen;
    if (!searchOpen) search = "";
  }
</script>

<div class="list-box">
  <div class="list-title">
    <span class="title-text"><IconAppWindow width="15" height="15" /> {title}</span>
    <span class="count">{windows.length}</span>
    <div class="tools">
      <button class="mini icon" title={t("windowList.refresh")} aria-label={t("windowList.refresh")} onclick={() => onrefresh?.()}>
        <IconRefresh width="14" height="14" />
      </button>
      <button
        class="mini icon"
        class:active={searchOpen}
        title={t("common.search")}
        aria-label={t("common.search")}
        onclick={toggleSearch}
      >
        <IconSearch width="14" height="14" />
      </button>
      <button
        bind:this={menuBtn}
        class="mini icon"
        class:active={menuOpen}
        aria-haspopup="true"
        aria-expanded={menuOpen}
        title={t("windowList.moreOptions")}
        aria-label={t("windowList.moreOptions")}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <IconChevronDown width="14" height="14" />
      </button>
      <Flyout bind:open={menuOpen} anchor={menuBtn} align="end" minWidth={180}>
        <div class="menu">
          <div class="menu-item"><CheckBox block bind:checked={showBackground} label={t("windowList.backgroundProcesses")} /></div>
          <div class="menu-item"><CheckBox block bind:checked={showUntitled} label={t("windowList.untitledWindows")} /></div>
        </div>
      </Flyout>
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

  <div class="lv-body" role="listbox" aria-label={title}>
    {#if windows.length === 0}
      <p class="hint lv-empty">{t("common.empty")}</p>
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
          <div class="lv-row win-item">
            <CheckBox block small bind:group={selected} value={w.hwnd} title={w.path}>
              <span class="wtitle">{w.title === NO_TITLE ? t("common.noTitleWindow") : w.title}</span>
              <span class="meta">PID {w.PID}</span>
            </CheckBox>
          </div>
        {/each}
      </div>
    {/each}
  {/if}
{/snippet}

<style>
  .menu {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .menu-item {
    display: flex;
    padding: 7px 10px;
    border-radius: var(--r-control);
    font-size: 12px;
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .menu-item:hover {
    background: var(--subtle-hover);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    border-bottom: 1px solid var(--divider);
    color: var(--text-2);
    flex: none;
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: var(--text);
    user-select: text;
    cursor: text;
  }
  .search input:focus {
    outline: none;
  }
  .clear {
    display: inline-flex;
    color: var(--text-2);
  }
  .clear:hover {
    color: var(--text);
  }

  .section-label {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 8px 6px 4px;
    padding-top: 8px;
    border-top: 1px solid var(--divider);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .section-label span {
    font-weight: 500;
    background: var(--control-alt);
    border-radius: 99px;
    padding: 0 6px;
  }

  .proc-group {
    margin-bottom: 6px;
  }
  .proc-name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: var(--text-2);
    padding: 4px 8px;
    font-size: 12px;
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
    opacity: 0.55;
  }
  .proc-name .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .win-item {
    margin-left: 14px;
  }
  .wtitle {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--text-2);
    font-size: 12px;
    flex: none;
  }
</style>
