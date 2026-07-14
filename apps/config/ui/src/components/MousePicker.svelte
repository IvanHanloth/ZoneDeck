<script>
  // 鼠标示意图：点亮哪颗键，哪颗键就能触发隐藏。五颗键都可用（左 / 中 / 右 / 两颗侧键），
  // 每颗键各自配「连击几次」和「要不要按住修饰键」，连击判定窗口全局共用一个。
  // 图形几何取自根目录 mouse_icon.svg。
  import ModifierRecorder from "./ModifierRecorder.svelte";
  import {
    MAX_MULTI_CLICK_MS,
    MIN_MULTI_CLICK_MS,
    MOUSE_PARTS,
    describeTrigger,
  } from "../lib/pointer.js";

  let { mouse } = $props();

  const CLICK_LABELS = ["单击", "双击", "三击"];

  function toggle(key) {
    mouse[key].enabled = !mouse[key].enabled;
  }

  function onkey(e, key) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      toggle(key);
    }
  }
</script>

<div class="picker">
  <svg class="mouse" viewBox="0 0 607 924" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <!-- 外壳 -->
    <rect class="shell" x="16" y="16" width="575" height="892" rx="287.5" ry="287.5" />

    <!-- 左键 -->
    <g
      class="part"
      class:on={mouse.left.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.left.enabled}
      aria-label="左键"
      onclick={() => toggle("left")}
      onkeydown={(e) => onkey(e, "left")}
    >
      <path d="M 16 418 L 16 303.5 A 287.5 287.5 0 0 1 303.5 16 L 303.5 418 Z" />
    </g>

    <!-- 右键 -->
    <g
      class="part"
      class:on={mouse.right.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.right.enabled}
      aria-label="右键"
      onclick={() => toggle("right")}
      onkeydown={(e) => onkey(e, "right")}
    >
      <path d="M 591 418 L 591 303.5 A 287.5 287.5 0 0 0 303.5 16 L 303.5 418 Z" />
    </g>

    <!-- 侧键 1（前进键）：靠前的那半。侧键画得很窄，用透明矩形把点击区扩到壳体外侧 -->
    <g
      class="part"
      class:on={mouse.side1.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.side1.enabled}
      aria-label="侧键 1（前进键）"
      onclick={() => toggle("side1")}
      onkeydown={(e) => onkey(e, "side1")}
    >
      <rect class="hit" x="-46" y="345" width="126" height="165" />
      <path d="M 16 355 C 64 355, 64 355, 64 400 L 64 510 L 16 510 Z" />
    </g>

    <!-- 侧键 2（后退键）：靠后的那半 -->
    <g
      class="part"
      class:on={mouse.side2.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.side2.enabled}
      aria-label="侧键 2（后退键）"
      onclick={() => toggle("side2")}
      onkeydown={(e) => onkey(e, "side2")}
    >
      <rect class="hit" x="-46" y="510" width="126" height="165" />
      <path d="M 16 510 L 64 510 L 64 615 C 64 665, 64 665, 16 665 Z" />
    </g>

    <!-- 中键（滚轮）：画在左右键之上，压住它们的分界线 -->
    <g
      class="part"
      class:on={mouse.middle.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.middle.enabled}
      aria-label="中键（滚轮）"
      onclick={() => toggle("middle")}
      onkeydown={(e) => onkey(e, "middle")}
    >
      <rect x="256" y="128" width="95" height="200" rx="47.5" ry="47.5" />
    </g>
  </svg>

  <ul class="list">
    {#each MOUSE_PARTS as part (part.key)}
      {@const btn = mouse[part.key]}
      <li class="row" class:on={btn.enabled}>
        <button
          type="button"
          class="name"
          aria-pressed={btn.enabled}
          onclick={() => toggle(part.key)}
        >
          <span class="dot"></span>
          <span class="text">{part.label}</span>
          <span class="state">{describeTrigger(btn)}</span>
        </button>

        {#if btn.enabled}
          <div class="cfg">
            <div class="clicks" role="radiogroup" aria-label="{part.label}连击次数">
              {#each CLICK_LABELS as label, i (label)}
                <button
                  type="button"
                  class="seg"
                  class:sel={btn.clicks === i + 1}
                  role="radio"
                  aria-checked={btn.clicks === i + 1}
                  onclick={() => (btn.clicks = i + 1)}
                >
                  {label}
                </button>
              {/each}
            </div>
            <ModifierRecorder bind:value={btn.modifiers} />
          </div>
        {/if}
      </li>
    {/each}
  </ul>
</div>

<div class="window">
  <label class="win-label" for="multi-click-ms">连击判定窗口</label>
  <div class="win-ctl">
    <input
      id="multi-click-ms"
      type="number"
      min={MIN_MULTI_CLICK_MS}
      max={MAX_MULTI_CLICK_MS}
      step="50"
      bind:value={mouse.multi_click_ms}
    />
    <span class="muted">毫秒</span>
  </div>
</div>


<style>
  .picker {
    display: flex;
    align-items: flex-start;
    gap: 18px;

    /* 线条和填充一律用不透明色：左右键的描边和外壳、滚轮和左右键都有重叠，
       只要有一点透明度，重叠处就会叠出更深的假描边，悬浮时还会把底下压着的线透出来。 */
    --line: color-mix(in srgb, var(--text) 58%, var(--surface-2));
    --face: var(--surface);
    --face-hover: color-mix(in srgb, var(--text) 9%, var(--surface));
    /* 混 --text 而不是混黑：亮色主题下描边偏深、暗色主题下自动偏淡，两边都压得住蓝色填充 */
    --face-on-line: color-mix(in srgb, var(--accent) 50%, var(--text));
  }

  .mouse {
    flex: none;
    width: 132px;
    height: auto;
    overflow: visible;
  }

  .shell {
    fill: var(--surface-2);
    stroke: var(--line);
    stroke-width: 18;
  }

  .part {
    cursor: pointer;
    outline: none;
  }
  .hit {
    fill: transparent;
    stroke: none;
    pointer-events: all;
  }
  .part :is(rect, path):not(.hit) {
    fill: var(--face);
    stroke: var(--line);
    stroke-width: 18;
    stroke-linejoin: round;
    transition:
      fill 0.15s,
      stroke 0.15s;
  }
  .part:hover :is(rect, path):not(.hit) {
    fill: var(--face-hover);
  }
  .part.on :is(rect, path):not(.hit) {
    fill: var(--accent);
    stroke: var(--face-on-line);
  }
  .part.on:hover :is(rect, path):not(.hit) {
    fill: var(--accent-strong);
  }
  .part:focus-visible :is(rect, path):not(.hit) {
    stroke: var(--accent);
    stroke-dasharray: 24 16;
  }

  .list {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }

  .row {
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 2px;
  }
  .row.on {
    border-color: var(--border);
    background: var(--surface-2);
  }

  .name {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px;
    border-radius: 6px;
    text-align: left;
  }
  .name:hover {
    background: var(--hover);
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--muted);
    transition: background 0.15s;
  }
  .row.on .dot {
    background: var(--accent);
  }
  .text {
    flex: 1;
    min-width: 0;
    font-size: 13px;
  }
  .state {
    flex: none;
    font-size: 12px;
    color: var(--muted);
  }
  .row.on .state {
    color: var(--accent);
  }

  .cfg {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 0 8px 8px 26px;
  }

  .clicks {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    background: var(--surface);
  }
  .seg {
    padding: 4px 10px;
    font-size: 12.5px;
    color: var(--muted);
    border-right: 1px solid var(--border);
  }
  .seg:last-child {
    border-right: none;
  }
  .seg:hover {
    background: var(--hover);
  }
  .seg.sel {
    background: var(--accent);
    color: var(--on-accent);
  }

  .window {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    padding-top: 4px;
    border-top: 1px solid var(--border);
    margin-top: 4px;
  }
  .win-label {
    font-size: 13px;
  }
  .win-ctl {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .win-ctl input {
    width: 84px;
  }
  .win-ctl .muted {
    font-size: 12px;
  }

  @media (max-width: 560px) {
    .picker {
      flex-direction: column;
      align-items: stretch;
    }
    .mouse {
      align-self: center;
    }
  }
</style>
