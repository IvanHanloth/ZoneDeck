<script>
  // 无边框窗口的八向缩放热区：命中后交给系统原生缩放（startResizeDragging）。
  import { win } from "../lib/ipc.js";
  import { app } from "../lib/state.svelte.js";

  const EDGE = 5; // 热区厚度（px）
  const handles = [
    { dir: "North", style: `top:0;left:${EDGE}px;right:${EDGE}px;height:${EDGE}px;cursor:n-resize` },
    { dir: "South", style: `bottom:0;left:${EDGE}px;right:${EDGE}px;height:${EDGE}px;cursor:s-resize` },
    { dir: "West", style: `left:0;top:${EDGE}px;bottom:${EDGE}px;width:${EDGE}px;cursor:w-resize` },
    { dir: "East", style: `right:0;top:${EDGE}px;bottom:${EDGE}px;width:${EDGE}px;cursor:e-resize` },
    { dir: "NorthWest", style: `top:0;left:0;width:${EDGE * 2}px;height:${EDGE * 2}px;cursor:nw-resize` },
    { dir: "NorthEast", style: `top:0;right:0;width:${EDGE * 2}px;height:${EDGE * 2}px;cursor:ne-resize` },
    { dir: "SouthWest", style: `bottom:0;left:0;width:${EDGE * 2}px;height:${EDGE * 2}px;cursor:sw-resize` },
    { dir: "SouthEast", style: `bottom:0;right:0;width:${EDGE * 2}px;height:${EDGE * 2}px;cursor:se-resize` },
  ];
</script>

{#if !app.maximized}
  {#each handles as h (h.dir)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="handle"
      style={h.style}
      aria-hidden="true"
      onpointerdown={(e) => {
        if (e.button === 0) win.startResize(h.dir);
      }}
    ></div>
  {/each}
{/if}

<style>
  .handle {
    position: fixed;
    z-index: 1000;
  }
</style>
