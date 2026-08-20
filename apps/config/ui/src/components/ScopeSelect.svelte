<script>
  import { t } from "../lib/i18n.svelte.js";
  import IconFilter from "~icons/lucide/list-filter";
  import IconChevron from "~icons/lucide/chevron-down";
  import CheckBox from "./fluent/CheckBox.svelte";
  import Flyout from "./fluent/Flyout.svelte";

  let {
    includeUntitled = $bindable(false),
    includeBackground = $bindable(false),
  } = $props();

  let open = $state(false);
  let trigger = $state(null);

  const summary = $derived.by(() => {
    const parts = [];
    if (includeUntitled) parts.push(t("scope.untitled"));
    if (includeBackground) parts.push(t("scope.background"));
    return parts.length
      ? t("scope.summaryWith", { parts: parts.join(" + ") })
      : t("scope.visibleOnly");
  });
</script>

<button
  bind:this={trigger}
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

<Flyout bind:open anchor={trigger} align="end" minWidth={230} ariaLabel={t("scope.aria")}>
  <div class="opts">
    <div class="opt"><CheckBox block bind:checked={includeUntitled} label={t("scope.matchUntitled")} /></div>
    <div class="opt"><CheckBox block bind:checked={includeBackground} label={t("scope.matchBackground")} /></div>
  </div>
</Flyout>

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: none;
    max-width: 150px;
    min-height: 26px;
    padding: 2px 8px;
    border-radius: var(--r-control);
    border: 1px solid var(--stroke);
    border-bottom-color: var(--stroke-strong);
    background: var(--control);
    color: var(--text-2);
    font-size: 11.5px;
    white-space: nowrap;
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .trigger:hover {
    background: var(--control-hover);
    color: var(--text);
  }
  .trigger.on {
    color: var(--accent);
  }
  .txt {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .opts {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .opt {
    display: flex;
    padding: 7px 10px;
    border-radius: var(--r-control);
    font-size: 12px;
    line-height: 18px;
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .opt:hover {
    background: var(--subtle-hover);
  }
</style>
