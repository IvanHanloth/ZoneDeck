<script>
  import { t } from "../lib/i18n.svelte.js";
  // 只录制修饰键（主键是鼠标按钮本身）；录制期独占键盘，
  // 否则录 Win 时一松手就弹出开始菜单。
  import { onDestroy } from "svelte";
  import { startCapture } from "../lib/capture.js";
  import { resumeMonitoring, suspendMonitoring } from "../lib/state.svelte.js";

  let { value = $bindable(""), compact = false } = $props();

  // 每个录制器一个独立的停用理由。
  const REASON = { recorder: "modifier" };

  let recording = $state(false);
  let live = $state(""); // 此刻按住的组合
  let best = ""; // 录制过程中按住过的最大组合
  let timer = null;
  let stopCapture = null;

  const size = (mods) => (mods ? mods.split("+").length : 0);

  function onState(s) {
    if (s.down && s.key === "Esc" && !s.modifiers) return stop();
    live = s.modifiers;
    if (size(s.modifiers) >= size(best)) best = s.modifiers;
    // 全部松开即定稿。
    if (!s.modifiers && best) {
      value = best;
      stop();
    }
  }

  function start() {
    if (recording) return stop();
    recording = true;
    live = "";
    best = "";
    suspendMonitoring(REASON);
    stopCapture = startCapture({ onState, onLost: stop });
    timer = setTimeout(stop, 10_000);
  }

  function stop() {
    if (!recording) return;
    recording = false;
    live = "";
    best = "";
    clearTimeout(timer);
    stopCapture?.();
    stopCapture = null;
    resumeMonitoring(REASON);
  }

  function clear() {
    stop();
    value = "";
  }

  onDestroy(stop);
</script>

<div class="rec" class:compact>
  <kbd class="combo" class:recording>
    {#if recording}
      {live || t("recorder.holdModifiers")}
    {:else}
      {value || t("recorder.none")}
    {/if}
  </kbd>
  <button class="btn ghost" type="button" onclick={start}>
    {recording ? t("common.cancel") : t("common.record")}
  </button>
  {#if value && !recording}
    <button class="btn ghost" type="button" onclick={clear}>{t("common.clear")}</button>
  {/if}
</div>

<style>
  .rec {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .combo {
    flex: 1;
    min-width: 5.5em;
    font-family: inherit;
    font-size: 12px;
    text-align: center;
    background: var(--control);
    border: 1px solid var(--stroke);
    border-bottom-color: var(--stroke-strong);
    border-radius: var(--r-control);
    padding: 4px 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .combo.recording {
    border-color: var(--accent);
    border-bottom-color: var(--accent);
    color: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.55;
    }
  }
  .rec .btn {
    min-height: 26px;
    padding: 3px 9px;
    font-size: 12px;
  }
</style>
