<script>
  import { t } from "../lib/i18n.svelte.js";
  // 录制热键期间暂停核心的全局热键监控，结束后恢复。
  import { onDestroy } from "svelte";
  import IconPencil from "~icons/lucide/pencil";
  import IconBan from "~icons/lucide/ban";
  import { comboFromEvent } from "../lib/hotkey.js";
  import { resumeMonitoring, suspendMonitoring } from "../lib/state.svelte.js";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";

  let {
    icon = null,
    label,
    value = $bindable(""),
    intercept = $bindable(false),
    interceptLabel = "",
    interceptTitle = "",
  } = $props();

  // 独立理由，避免多个录制器互相撤销停用。
  const REASON = { recorder: "hotkey" };

  let open = $state(false);
  // 对话框里的待定值，保存前不碰 value。
  let draft = $state("");

  const keys = $derived(value ? value.split("+") : []);
  const draftKeys = $derived(draft ? draft.split("+") : []);

  function onKeydown(e) {
    e.preventDefault();
    e.stopPropagation();
    const combo = comboFromEvent(e);
    if (!combo) return; // 修饰键或不支持的键，继续等待
    draft = combo;
  }

  function edit() {
    draft = value;
    open = true;
    suspendMonitoring(REASON);
    window.addEventListener("keydown", onKeydown, true);
  }

  function teardown() {
    window.removeEventListener("keydown", onKeydown, true);
    resumeMonitoring(REASON);
  }

  function save() {
    value = draft;
    open = false;
  }

  // 对话框走 Esc / 遮罩 / 取消关闭时同样要收摊子。
  $effect(() => {
    if (!open) teardown();
  });

  onDestroy(teardown);
</script>

<SettingsExpander {icon} {label}>
  {#snippet control()}
    {#if keys.length}
      <span class="keys">
        {#each keys as k, i (i)}<kbd class="key">{k}</kbd>{/each}
      </span>
    {:else}
      <span class="none">{t("recorder.disabled")}</span>
    {/if}
    <button
      class="btn icon"
      type="button"
      onclick={edit}
      title={t("recorder.edit")}
      aria-label={t("recorder.edit")}
    >
      <IconPencil width="14" height="14" />
    </button>
  {/snippet}

  <SettingsCard
    variant="sub"
    icon={IconBan}
    label={interceptLabel}
    description={interceptTitle}
    disabled={!value}
  >
    {#snippet control()}
      <ToggleSwitch bind:checked={intercept} disabled={!value} />
    {/snippet}
  </SettingsCard>
</SettingsExpander>

<ContentDialog bind:open title={label}>
  <p class="hint">{t("recorder.dialogHint")}</p>
  <div class="stage">
    {#if draftKeys.length}
      {#each draftKeys as k, i (i)}<kbd class="key big">{k}</kbd>{/each}
    {:else}
      <span class="waiting">{t("recorder.waiting")}</span>
    {/if}
  </div>

  {#snippet footer()}
    <button class="btn primary" type="button" onclick={save}>{t("common.save")}</button>
    <button class="btn" type="button" onclick={() => (draft = "")} disabled={!draft}>
      {t("common.clear")}
    </button>
    <button class="btn" type="button" onclick={() => (open = false)}>{t("common.cancel")}</button>
  {/snippet}
</ContentDialog>

<style>
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  /* 键帽：accent 实心块，一键一块，和 PowerToys 的快捷键展示一致 */
  .key {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--r-control);
    background: var(--accent);
    color: var(--on-accent);
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }
  .key.big {
    min-width: 56px;
    height: 56px;
    padding: 0 18px;
    font-size: 17px;
    border-radius: var(--r-card);
  }
  .none {
    color: var(--text-3);
    font-size: 13px;
  }

  .stage {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 96px;
    margin-top: 20px;
    padding: 20px;
    border-radius: var(--r-card);
    background: var(--card-2);
  }
  .waiting {
    color: var(--text-3);
  }
</style>
