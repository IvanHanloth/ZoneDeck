<script>
  // Fluent 弹出层基元：fixed 定位 + 溢出翻转，不受滚动容器的 overflow 裁剪。
  // 供 ComboBox、筛选菜单等复用。
  import { tick } from "svelte";

  let {
    open = $bindable(false),
    anchor = null,
    align = "start",
    matchWidth = false,
    minWidth = 0,
    ariaLabel = "",
    role = "group",
    children,
  } = $props();

  let el = $state(null);
  let top = $state(0);
  let left = $state(0);
  let width = $state(0);
  let flipped = $state(false);

  const GAP = 4;
  const EDGE = 8;

  async function place() {
    if (!anchor) return;
    // 等内容挂上 DOM 才量得到尺寸。
    await tick();
    if (!el) return;
    const a = anchor.getBoundingClientRect();
    const w = matchWidth ? a.width : Math.max(el.offsetWidth, minWidth);
    const h = el.offsetHeight;

    // 下方放不下就翻到上方；上方也放不下则贴住视口底部。
    let t = a.bottom + GAP;
    let f = false;
    if (t + h > innerHeight - EDGE) {
      const above = a.top - GAP - h;
      if (above >= EDGE) {
        t = above;
        f = true;
      } else {
        t = Math.max(EDGE, innerHeight - EDGE - h);
      }
    }

    let l = align === "end" ? a.right - w : a.left;
    l = Math.min(Math.max(EDGE, l), Math.max(EDGE, innerWidth - w - EDGE));

    top = t;
    left = l;
    width = w;
    flipped = f;
  }

  function close() {
    open = false;
  }

  // 点在触发器上时不关：交给触发器自己 toggle，否则会关了又立刻开。
  function onPointerDown(e) {
    if (!open) return;
    if (el?.contains(e.target) || anchor?.contains(e.target)) return;
    close();
  }

  $effect(() => {
    if (open) place();
  });

  // 任何祖先容器滚动都会让 fixed 定位失准，直接收起。
  $effect(() => {
    if (!open) return;
    document.addEventListener("scroll", close, true);
    return () => document.removeEventListener("scroll", close, true);
  });
</script>

<svelte:window
  onpointerdown={onPointerDown}
  onkeydown={(e) => open && e.key === "Escape" && close()}
  onresize={close}
/>

{#if open}
  <div
    bind:this={el}
    class="flyout"
    class:flipped
    class:sized={matchWidth}
    style:top="{top}px"
    style:left="{left}px"
    style:width={matchWidth ? `${width}px` : null}
    style:min-width={minWidth ? `${minWidth}px` : null}
    {role}
    aria-label={ariaLabel || null}
  >
    {@render children?.()}
  </div>
{/if}

<style>
  .flyout {
    position: fixed;
    z-index: 500;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: 4px;
    border-radius: var(--r-overlay);
    border: 1px solid var(--stroke);
    /* 实心底：弹层内容后面不该透出正文 */
    background: var(--flyout-solid);
    box-shadow: var(--shadow-flyout);
    animation: flyout-in var(--dur-normal) var(--ease-standard);
    transform-origin: top center;
  }
  .flyout.flipped {
    transform-origin: bottom center;
  }

  @keyframes flyout-in {
    from {
      opacity: 0;
      scale: 1 0.9;
    }
  }
</style>
