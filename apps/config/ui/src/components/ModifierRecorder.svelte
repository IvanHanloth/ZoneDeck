<script>
  // 修饰键录制：点击后按住想要的修饰键（Ctrl / Alt / Shift / Win），松手即写入。
  // 和 HotkeyRecorder 的区别是这里没有主键——主键是鼠标按钮本身。
  import { onDestroy } from "svelte";
  import { modifiersFromEvent } from "../lib/hotkey.js";

  let { value = $bindable(""), compact = false } = $props();

  let recording = $state(false);
  let held = $state(""); // 录制过程中按住过的最大组合
  let timer = null;

  function onKeydown(e) {
    if (e.key === "Escape") return stop();
    e.preventDefault();
    const mods = modifiersFromEvent(e);
    // 取按下过的最长组合：按 Ctrl 再加 Shift 时不该被后一次事件覆盖成更短的
    if (mods.split("+").filter(Boolean).length >= held.split("+").filter(Boolean).length) {
      held = mods;
    }
  }

  function onKeyup(e) {
    e.preventDefault();
    // 修饰键全部松开 → 录制完成
    if (!e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey && held) {
      value = held;
      stop();
    }
  }

  function start() {
    if (recording) return stop();
    recording = true;
    held = "";
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("keyup", onKeyup, true);
    timer = setTimeout(stop, 10_000);
  }

  function stop() {
    if (!recording) return;
    recording = false;
    held = "";
    clearTimeout(timer);
    window.removeEventListener("keydown", onKeydown, true);
    window.removeEventListener("keyup", onKeyup, true);
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
      {held || "按住修饰键…"}
    {:else}
      {value || "无"}
    {/if}
  </kbd>
  <button class="btn ghost" type="button" onclick={start}>
    {recording ? "取消" : "录制"}
  </button>
  {#if value && !recording}
    <button class="btn ghost" type="button" onclick={clear}>清除</button>
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
    font-size: 12.5px;
    text-align: center;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .combo.recording {
    border-color: var(--accent);
    color: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.55;
    }
  }
  .rec .btn {
    padding: 4px 9px;
    font-size: 12.5px;
  }
</style>
