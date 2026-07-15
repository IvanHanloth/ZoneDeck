<script>
  let {
    checked = $bindable(false),
    label = "",
    title = "",
    disabled = false,
    onchange = undefined,
  } = $props();
</script>

<label class="toggle" {title} class:disabled>
  <input type="checkbox" bind:checked {disabled} {onchange} />
  <span class="track"><span class="thumb"></span></span>
  {#if label}<span class="text">{label}</span>{/if}
</label>

<style>
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    padding: 5px 2px;
    cursor: pointer;
    line-height: 1.4;
  }
  .toggle.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .track {
    flex: none;
    width: 38px;
    height: 21px;
    border-radius: 99px;
    background: var(--border);
    position: relative;
    transition: background 0.18s;
  }
  .thumb {
    position: absolute;
    top: 2.5px;
    left: 2.5px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--surface);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: transform 0.18s;
  }

  input:checked + .track {
    background: var(--accent);
  }
  input:checked + .track .thumb {
    transform: translateX(17px);
  }
  input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
