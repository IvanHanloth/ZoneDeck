<script>
  // 热键录制：点击后捕获下一个组合键，写回双向绑定的 value。
  import { onDestroy } from "svelte";
  import { comboFromEvent } from "../lib/hotkey.js";

  let { label, value = $bindable("") } = $props();

  let recording = $state(false);
  let timer = null;

  function onKeydown(e) {
    e.preventDefault();
    e.stopPropagation();
    const combo = comboFromEvent(e);
    if (!combo) return; // 只按了修饰键或不支持的键，继续等
    value = combo;
    stop();
  }

  function start() {
    if (recording) return stop();
    recording = true;
    window.addEventListener("keydown", onKeydown, true);
    timer = setTimeout(stop, 10_000); // 超时自动取消
  }

  function stop() {
    recording = false;
    clearTimeout(timer);
    window.removeEventListener("keydown", onKeydown, true);
  }

  onDestroy(stop);
</script>

<div class="row">
  <span class="label">{label}</span>
  <kbd class="combo" class:recording>{recording ? "请按下组合键…" : value || "未设置"}</kbd>
  <button class="btn ghost" onclick={start}>{recording ? "取消" : "录制"}</button>
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
