<script>
  import { t } from "../lib/i18n.svelte.js";
  // 录制期独占键盘，并暂停核心的全局热键监控，结束后一并恢复。
  import { onDestroy } from "svelte";
  import IconPencil from "~icons/lucide/pencil";
  import IconBan from "~icons/lucide/ban";
  import { startCapture } from "../lib/capture.js";
  import { joinCombo } from "../lib/hotkey.js";
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
  // 此刻按住的键。
  let live = $state({ modifiers: "", key: null });
  // 本轮已录到完整组合；松手途中不再跟着 live 掉键，避免画面闪回半截组合。
  let committed = $state(false);
  // 上一次按了热键表里没有的键；按到能用的键或重开对话框才消掉。
  let unsupported = $state(false);
  // 没能独占键盘，按键仍会漏给其他程序。
  let degraded = $state(false);

  let stop = null;

  const keys = $derived(value ? value.split("+") : []);
  const held = $derived(joinCombo(live.modifiers, live.key));
  // 手按着就跟着手走，录完与全松开后停在已录到的组合上。
  const stage = $derived(committed ? draft : held || draft);
  const stageKeys = $derived(stage ? stage.split("+") : []);

  function reset() {
    live = { modifiers: "", key: null };
    committed = false;
    unsupported = false;
  }

  function onState(s) {
    live = { modifiers: s.modifiers, key: s.key };
    // 手全松开，下一次按下重新开始录。
    if (!s.modifiers && !s.key) committed = false;
    if (!s.down) return;
    // 键盘被独占时裸 Esc 是唯一的键盘退路；Win+Esc 等带修饰键的组合照常录。
    if (s.key === "Esc" && !s.modifiers) return (open = false);
    if (s.unsupported) return (unsupported = true);
    if (!s.key) return;
    draft = joinCombo(s.modifiers, s.key);
    committed = true;
    unsupported = false;
  }

  function edit() {
    draft = value;
    reset();
    degraded = false;
    open = true;
    suspendMonitoring(REASON);
    stop = startCapture({
      onState,
      onLost: () => (open = false),
      onDegraded: () => (degraded = true),
    });
  }

  function teardown() {
    stop?.();
    stop = null;
    reset();
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
  <div class="stage" class:live={!!held}>
    {#if stageKeys.length}
      {#each stageKeys as k, i (i)}<kbd class="key big">{k}</kbd>{/each}
    {:else}
      <span class="waiting">{t("recorder.waiting")}</span>
    {/if}
  </div>
  {#if unsupported}
    <p class="note">{t("recorder.unsupportedKey")}</p>
  {/if}
  {#if degraded}
    <p class="note">{t("recorder.captureFailed")}</p>
  {/if}

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
    border: 1px solid transparent;
    transition: border-color var(--dur-fast) var(--ease-standard);
  }
  /* 手按着键时描边点亮，让「程序收到了」这件事看得见 */
  .stage.live {
    border-color: var(--accent);
  }
  .waiting {
    color: var(--text-3);
  }
  .note {
    margin-top: 10px;
    font-size: 12px;
    color: var(--text-2);
    text-align: center;
  }
</style>
