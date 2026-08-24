<script module>
  // 每个实例一个 id 前缀，供 aria-activedescendant 指向高亮项。
  let seq = 0;
</script>

<script>
  // Fluent ComboBox：替代原生 <select>，展开层走 Flyout。
  import IconChevronDown from "~icons/lucide/chevron-down";
  import Flyout from "./Flyout.svelte";

  let {
    value = $bindable(),
    options = [],
    disabled = false,
    compact = false,
    title = "",
    ariaLabel = "",
  } = $props();

  const uid = `cbx-${++seq}`;

  let trigger = $state(null);
  let list = $state(null);
  let open = $state(false);
  let active = $state(-1);

  const selectedIndex = $derived(options.findIndex((o) => o.value === value));
  const selectedLabel = $derived(options[selectedIndex]?.label ?? "");

  function toggle() {
    if (disabled) return;
    open = !open;
    if (open) active = selectedIndex >= 0 ? selectedIndex : 0;
  }

  function pick(i) {
    const opt = options[i];
    if (!opt) return;
    value = opt.value;
    open = false;
    trigger?.focus();
  }

  function onKey(e) {
    if (disabled) return;
    const last = options.length - 1;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) return toggle();
      active = Math.min(last, Math.max(0, active + (e.key === "ArrowDown" ? 1 : -1)));
    } else if (e.key === "Home" && open) {
      e.preventDefault();
      active = 0;
    } else if (e.key === "End" && open) {
      e.preventDefault();
      active = last;
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (open) pick(active);
      else toggle();
    } else if (e.key === "Escape" && open) {
      e.preventDefault();
      open = false;
    } else if (e.key === "Tab") {
      open = false;
    }
  }

  // 键盘移动高亮时把选项滚进视野。
  $effect(() => {
    if (!open || active < 0 || !list) return;
    list.children[active]?.scrollIntoView({ block: "nearest" });
  });
</script>

<button
  bind:this={trigger}
  class="trigger elev"
  class:open
  class:compact
  {title}
  {disabled}
  role="combobox"
  aria-label={ariaLabel || null}
  aria-controls="{uid}-list"
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-activedescendant={open && active >= 0 ? `${uid}-${active}` : null}
  onclick={toggle}
  onkeydown={onKey}
>
  <span class="val">{selectedLabel}</span>
  <IconChevronDown width="12" height="12" />
</button>

<Flyout bind:open anchor={trigger} matchWidth minWidth={120} role="presentation">
  <div
    bind:this={list}
    id="{uid}-list"
    class="list"
    role="listbox"
    aria-label={ariaLabel || title || null}
  >
    {#each options as opt, i (opt.value)}
      <!-- 键盘选择统一在触发器上处理（↑↓ / Enter / Esc），选项本身只接鼠标 -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        id="{uid}-{i}"
        class="opt"
        class:sel={i === selectedIndex}
        class:active={i === active}
        role="option"
        tabindex="-1"
        aria-selected={i === selectedIndex}
        onclick={() => pick(i)}
        onpointerenter={() => (active = i)}
      >
        <span class="ind" aria-hidden="true"></span>
        <span class="opt-label">{opt.label}</span>
      </div>
    {/each}
  </div>
</Flyout>

<style>
  /* 下拉在一页里齐宽，不跟着选项文字长短伸缩；超过 200px 才撑开 */
  .trigger {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-width: 200px;
    min-height: 32px;
    padding: 5px 11px;
    border-radius: var(--r-control);
    background: var(--control);
    color: var(--text);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .trigger:hover:not(:disabled) {
    background: var(--control-hover);
  }
  .trigger.open,
  .trigger:active:not(:disabled) {
    background: var(--control-pressed);
    color: var(--text-2);
  }
  .trigger:disabled {
    background: var(--control-pressed);
    color: var(--text-disabled);
    border-bottom-color: var(--stroke);
    cursor: not-allowed;
  }
  .val {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* 行内紧凑档：塞进 34px 高的列表行里 */
  .trigger.compact {
    min-width: 0;
    min-height: 26px;
    padding: 2px 8px;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-2);
  }
  .trigger.compact:hover:not(:disabled) {
    color: var(--text);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    padding: 6px 11px 6px 8px;
    border-radius: var(--r-control);
    color: var(--text);
  }
  .opt.active {
    background: var(--subtle-hover);
  }
  .opt:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
  }
  /* 选中项左侧的 accent 竖条，Win11 列表的通用选中标记 */
  .ind {
    flex: none;
    width: 3px;
    height: 16px;
    border-radius: 2px;
    background: transparent;
  }
  .opt.sel .ind {
    background: var(--accent);
  }
  .opt.sel {
    background: var(--subtle-hover);
  }
  /* 同 NavView：选中项被指到时要看得出来 */
  .opt.sel.active {
    background: var(--subtle-selected-hover);
  }
  .opt.sel:active {
    background: var(--subtle-pressed);
    color: var(--text-2);
  }
  .opt-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
