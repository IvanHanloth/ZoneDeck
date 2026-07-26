<script>
  import { t } from "../lib/i18n.svelte.js";
  import IconFilter from "~icons/lucide/list-filter";
  import IconChevron from "~icons/lucide/chevron-down";

  let {
    includeUntitled = $bindable(false),
    includeBackground = $bindable(false),
  } = $props();

  let open = $state(false);
  let root;

  const summary = $derived.by(() => {
    const parts = [];
    if (includeUntitled) parts.push(t("scope.untitled"));
    if (includeBackground) parts.push(t("scope.background"));
    return parts.length
      ? t("scope.summaryWith", { parts: parts.join(" + ") })
      : t("scope.visibleOnly");
  });

  function onWindowDown(e) {
    if (open && root && !root.contains(e.target)) open = false;
  }
</script>

<svelte:window
  onpointerdown={onWindowDown}
  onkeydown={(e) => e.key === "Escape" && (open = false)}
/>

<div class="scope" bind:this={root}>
  <button
    class="trigger"
    class:on={includeUntitled || includeBackground}
    title={t("scope.title")}
    aria-haspopup="true"
    aria-expanded={open}
    onclick={(e) => {
      e.preventDefault();
      open = !open;
    }}
  >
    <IconFilter width="12" height="12" />
    <span class="txt">{summary}</span>
    <IconChevron width="11" height="11" />
  </button>

  {#if open}
    <div class="menu" role="group" aria-label={t("scope.aria")}>
      <label class="opt">
        <input type="checkbox" bind:checked={includeUntitled} />
        <span>{t("scope.matchUntitled")}</span>
      </label>
      <label class="opt">
        <input type="checkbox" bind:checked={includeBackground} />
        <span>{t("scope.matchBackground")}</span>
      </label>
    </div>
  {/if}
</div>

<style>
  .scope {
    position: relative;
    flex: none;
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 140px;
    padding: 2px 6px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .trigger:hover {
    background: var(--hover);
    border-color: var(--accent);
    color: var(--text);
  }
  .trigger.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .txt {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    z-index: 50;
    width: 220px;
    padding: 5px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .opt {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 7px 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12.5px;
    line-height: 1.35;
  }
  .opt:hover {
    background: var(--hover);
  }
  .opt input {
    accent-color: var(--accent);
    margin-top: 2px;
    flex: none;
  }
</style>
