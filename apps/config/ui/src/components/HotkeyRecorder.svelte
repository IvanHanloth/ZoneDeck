<script>
  // 录制期间必须让核心先停用全局热键——否则你按下的组合键会直接触发现有热键
  // （比如按到 Ctrl+Q 就把窗口藏了），根本录不进来。取消 / 完成 / 组件销毁后恢复。
  import { onDestroy } from "svelte";
  import { comboFromEvent } from "../lib/hotkey.js";
  import { resumeMonitoring, suspendMonitoring } from "../lib/state.svelte.js";

  let { label, value = $bindable("") } = $props();

  // 每个录制器一个独立的理由（对象身份即标识）——两个录制器同时开着时，
  // 先结束的那个不能把另一个还需要的停用给撤了。
  const REASON = { recorder: "hotkey" };

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
