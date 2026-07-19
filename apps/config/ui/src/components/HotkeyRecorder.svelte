<script>
  import { t } from "../lib/i18n.svelte.js";
  // 录制热键期间暂停核心的全局热键监控，结束后恢复。
  import { onDestroy } from "svelte";
  import { comboFromEvent } from "../lib/hotkey.js";
  import { resumeMonitoring, suspendMonitoring } from "../lib/state.svelte.js";

  let { label, value = $bindable("") } = $props();

  // 独立理由，避免多个录制器互相撤销停用。
  const REASON = { recorder: "hotkey" };

  let recording = $state(false);
  let timer = null;

  function onKeydown(e) {
    e.preventDefault();
    e.stopPropagation();
    const combo = comboFromEvent(e);
    if (!combo) return; // 修饰键或不支持的键，继续等待
    value = combo;
    stop();
  }

  function start() {
    if (recording) return stop();
    recording = true;
    suspendMonitoring(REASON);
    window.addEventListener("keydown", onKeydown, true);
    timer = setTimeout(stop, 10_000);
  }

  function stop() {
    if (!recording) return;
    recording = false;
    clearTimeout(timer);
    window.removeEventListener("keydown", onKeydown, true);
    resumeMonitoring(REASON);
  }

  function clear() {
    stop();
    value = "";
  }

  onDestroy(stop);
</script>

<div class="row">
  <span class="label">{label}</span>
  <kbd class="combo" class:recording class:off={!recording && !value}>
    {recording ? t("recorder.pressCombo") : value || t("recorder.disabled")}
  </kbd>
  <button class="btn ghost" type="button" onclick={start}>{recording ? t("common.cancel") : t("common.record")}</button>
  {#if value && !recording}
    <button class="btn ghost" type="button" onclick={clear}>{t("common.clear")}</button>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .label {
    width: 8.5em;
    flex: none;
  }
  .combo {
    flex: 1;
    min-width: 0;
    font-family: inherit;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 12px;
    letter-spacing: 0.03em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .combo.recording {
    border-color: var(--accent);
    color: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .combo.off {
    color: var(--muted);
  }
  @keyframes pulse {
    50% {
      opacity: 0.55;
    }
  }

  @media (max-width: 560px) {
    .row {
      flex-wrap: wrap;
    }
    .label {
      width: 100%;
    }
  }
</style>
