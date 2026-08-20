<script>
  // Fluent TextBox：聚焦时底边长出 2px accent 下划线，而非整圈描边。
  // suffix 供数字框挂单位（"毫秒""分钟"）。
  let {
    value = $bindable(""),
    type = "text",
    placeholder = "",
    disabled = false,
    invalid = false,
    mono = false,
    suffix = "",
    title = "",
    ariaLabel = "",
    min = undefined,
    max = undefined,
    step = undefined,
    maxlength = undefined,
    spellcheck = false,
    width = "",
    onblur = undefined,
    oninput = undefined,
  } = $props();
</script>

<div class="tb elev" class:disabled class:invalid class:mono {title} style:width={width || null}>
  {#if type === "number"}
    <input
      type="number"
      bind:value
      {placeholder}
      {disabled}
      {min}
      {max}
      {step}
      {onblur}
      {oninput}
      aria-label={ariaLabel || null}
    />
  {:else}
    <input
      type="text"
      bind:value
      {placeholder}
      {disabled}
      {maxlength}
      {spellcheck}
      {onblur}
      {oninput}
      aria-label={ariaLabel || null}
    />
  {/if}
  {#if suffix}<span class="suffix">{suffix}</span>{/if}
</div>

<style>
  .tb {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
    padding: 0 10px;
    border-radius: var(--r-control);
    background: var(--control);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .tb:hover:not(.disabled) {
    background: var(--control-hover);
  }
  .tb:focus-within {
    background: var(--control-focus);
  }
  .tb.disabled {
    background: var(--control-pressed);
    border-bottom-color: var(--stroke);
  }
  .tb.invalid {
    border-color: var(--danger);
  }

  /* 覆盖在底边框上，聚焦时从底部展开 */
  .tb::after {
    content: "";
    position: absolute;
    left: -1px;
    right: -1px;
    bottom: -1px;
    height: 2px;
    border-radius: 0 0 var(--r-control) var(--r-control);
    background: var(--accent);
    scale: 1 0;
    transform-origin: bottom;
    transition: scale var(--dur-normal) var(--ease-standard);
  }
  .tb:focus-within::after {
    scale: 1 1;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 5px 0;
    font: inherit;
    color: var(--text);
    background: none;
    border: none;
    user-select: text;
    cursor: text;
  }
  input:focus {
    outline: none;
  }
  input::placeholder {
    color: var(--text-3);
  }
  input:disabled {
    color: var(--text-disabled);
    cursor: not-allowed;
  }
  .mono input {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .suffix {
    flex: none;
    color: var(--text-2);
    font-size: 12px;
  }
  .tb.disabled .suffix {
    color: var(--text-disabled);
  }
</style>
