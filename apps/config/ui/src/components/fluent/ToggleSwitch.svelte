<script>
  // Win11 ToggleSwitch：轨道 40×20，thumb 12px；hover 涨到 14px，按下拉成胶囊。
  import { t } from "../../lib/i18n.svelte.js";

  let {
    checked = $bindable(false),
    label = "",
    title = "",
    disabled = false,
    onchange = undefined,
  } = $props();

  // 带自定义 label 时不再重复显示开/关状态字。
  const showState = $derived(!label);
</script>

<label class="toggle" {title} class:disabled>
  <input type="checkbox" role="switch" bind:checked {disabled} {onchange} />
  {#if showState}
    <span class="state">{t(checked ? "common.on" : "common.off")}</span>
  {/if}
  <span class="track"><span class="thumb"></span></span>
  {#if label}<span class="text">{label}</span>{/if}
</label>

<style>
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    line-height: 20px;
  }
  .toggle.disabled {
    cursor: not-allowed;
  }

  input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .state {
    color: var(--text-2);
    font-size: 12px;
    min-width: 1.5em;
    text-align: right;
  }

  .track {
    flex: none;
    position: relative;
    width: 40px;
    height: 20px;
    border-radius: 10px;
    background: var(--control-alt);
    border: 1px solid var(--stroke-control-strong);
    transition:
      background var(--dur-fast) var(--ease-standard),
      border-color var(--dur-fast) var(--ease-standard);
  }
  .thumb {
    position: absolute;
    top: 50%;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 7px;
    background: var(--text-2);
    translate: 0 -50%;
    transition:
      translate var(--dur-normal) var(--ease-out),
      scale var(--dur-fast) var(--ease-standard),
      background var(--dur-fast) var(--ease-standard);
  }

  .toggle:not(.disabled):hover .track {
    background: var(--control-hover);
  }
  .toggle:not(.disabled):hover .thumb {
    scale: 1.17;
  }
  /* 按下时横向拉成胶囊，与 WinUI 的 pressed 态一致 */
  .toggle:not(.disabled):active .thumb {
    scale: 1.42 1;
  }

  input:checked + .state {
    color: var(--text-2);
  }
  .toggle:has(input:checked) .track {
    background: var(--accent);
    border-color: transparent;
  }
  .toggle:has(input:checked) .thumb {
    background: var(--on-accent);
    translate: 20px -50%;
  }
  .toggle:not(.disabled):has(input:checked):hover .track {
    background: var(--accent-hover);
  }

  .toggle.disabled .track {
    border-color: var(--text-disabled);
  }
  .toggle.disabled .thumb {
    background: var(--text-disabled);
  }
  .toggle.disabled:has(input:checked) .track {
    background: var(--accent-disabled);
    border-color: transparent;
  }
  .toggle.disabled .state,
  .toggle.disabled .text {
    color: var(--text-disabled);
  }

  input:focus-visible + .state + .track,
  input:focus-visible + .track {
    outline: 2px solid var(--focus-outer);
    outline-offset: 3px;
    box-shadow: 0 0 0 1px var(--focus-inner);
  }
</style>
