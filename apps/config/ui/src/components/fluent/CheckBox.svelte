<script>
  // Win11 CheckBox：方框 + accent 底白勾。
  // 传 group（数组）+ value 走多选，否则走单个 checked。
  let {
    checked = $bindable(false),
    group = $bindable(undefined),
    value = undefined,
    indeterminate = false,
    label = "",
    title = "",
    ariaLabel = "",
    disabled = false,
    block = false,
    small = false,
    onchange = undefined,
    children,
  } = $props();

  // 一个实例的模式不会中途改变，初始化时定一次即可。
  const multi = group !== undefined;

  // 多选不能用 bind:group —— 它按组件实例分组，每个 CheckBox 只看得见自己那一个
  // input，勾第二个时会把数组整个覆盖成只剩它，表现成单选。这里自己增删，
  // 数组始终由父组件持有，勾选态回读 group 即可。
  function onMultiChange(e) {
    group = e.currentTarget.checked
      ? [...group, value]
      : group.filter((v) => v !== value);
    onchange?.(e);
  }
</script>

<label class="cb" {title} class:disabled class:block class:small>
  {#if multi}
    <input
      type="checkbox"
      checked={group.includes(value)}
      {disabled}
      onchange={onMultiChange}
      aria-label={ariaLabel || null}
    />
  {:else}
    <input
      type="checkbox"
      bind:checked
      {disabled}
      {indeterminate}
      {onchange}
      aria-label={ariaLabel || null}
    />
  {/if}
  <span class="box" aria-hidden="true">
    {#if indeterminate}
      <span class="dash"></span>
    {:else}
      <svg viewBox="0 0 12 12" fill="none">
        <path
          d="M2 6.2 4.7 8.9 10 3"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {/if}
  </span>
  {#if label}<span class="text">{label}</span>{/if}
  {#if children}<span class="text">{@render children()}</span>{/if}
</label>

<style>
  .cb {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    line-height: 20px;
  }
  /* 撑满父容器，让整行都是点击区（列表多选用） */
  .cb.block {
    display: flex;
    width: 100%;
    min-width: 0;
  }
  .cb.block .text {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cb.disabled {
    cursor: not-allowed;
  }

  input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .box {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: var(--r-control);
    background: var(--control-alt);
    border: 1px solid var(--stroke-control-strong);
    color: transparent;
    transition:
      background var(--dur-fast) var(--ease-standard),
      border-color var(--dur-fast) var(--ease-standard);
  }
  /* color: inherit 锁住勾的颜色，外部的 `.xx svg {}` 才染不到它 ——
     勾靠 .box 的 color 在 transparent / --on-accent 之间切换 */
  .box svg {
    width: 12px;
    height: 12px;
    color: inherit;
  }
  /* 列表行里一行一个，20px 压得太满，收一档 */
  .cb.small .box {
    width: 17px;
    height: 17px;
  }
  .cb.small .box svg {
    width: 11px;
    height: 11px;
  }
  .dash {
    width: 8px;
    height: 1.6px;
    border-radius: 1px;
    background: currentColor;
  }

  .cb:not(.disabled):hover .box {
    background: var(--control-hover);
  }
  .cb:not(.disabled):active .box {
    background: var(--control-pressed);
    border-color: var(--text-3);
  }

  .cb:has(input:checked) .box,
  .cb:has(input:indeterminate) .box {
    background: var(--accent);
    border-color: transparent;
    color: var(--on-accent);
  }
  .cb:not(.disabled):has(input:checked):hover .box,
  .cb:not(.disabled):has(input:indeterminate):hover .box {
    background: var(--accent-hover);
  }

  .cb.disabled .box {
    border-color: var(--text-disabled);
  }
  .cb.disabled:has(input:checked) .box {
    background: var(--accent-disabled);
    border-color: transparent;
    color: var(--text-disabled);
  }
  .cb.disabled .text {
    color: var(--text-disabled);
  }

  input:focus-visible + .box {
    outline: 2px solid var(--focus-outer);
    outline-offset: 3px;
    box-shadow: 0 0 0 1px var(--focus-inner);
  }
</style>
