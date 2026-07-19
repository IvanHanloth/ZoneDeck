<script>
  // 鼠标按键选择器；图形几何取自根目录 mouse_icon.svg。
  import ModifierRecorder from "./ModifierRecorder.svelte";
  import { MOUSE_PARTS, describeTrigger } from "../lib/pointer.js";
  import { t } from "../lib/i18n.svelte.js";

  let { mouse } = $props();

  const CLICK_KEYS = ["mouse.singleClick", "mouse.doubleClick", "mouse.tripleClick"];

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
    <rect class="shell" x="16" y="16" width="575" height="892" rx="287.5" ry="287.5" />

    <g
      class="part"
      class:on={mouse.left.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.left.enabled}
      aria-label={t("mouse.left")}
      onclick={() => toggle("left")}
      onkeydown={(e) => onkey(e, "left")}
    >
      <path d="M 16 418 L 16 303.5 A 287.5 287.5 0 0 1 303.5 16 L 303.5 418 Z" />
    </g>

    <g
      class="part"
      class:on={mouse.right.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.right.enabled}
      aria-label={t("mouse.right")}
      onclick={() => toggle("right")}
      onkeydown={(e) => onkey(e, "right")}
    >
      <path d="M 591 418 L 591 303.5 A 287.5 287.5 0 0 0 303.5 16 L 303.5 418 Z" />
    </g>

    <!-- 透明矩形扩大侧键点击区 -->
    <g
      class="part"
      class:on={mouse.side1.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.side1.enabled}
      aria-label={t("mouse.side1")}
      onclick={() => toggle("side1")}
      onkeydown={(e) => onkey(e, "side1")}
    >
      <rect class="hit" x="-46" y="345" width="126" height="165" />
      <path d="M 16 355 C 64 355, 64 355, 64 400 L 64 510 L 16 510 Z" />
    </g>

    <g
      class="part"
      class:on={mouse.side2.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.side2.enabled}
      aria-label={t("mouse.side2")}
      onclick={() => toggle("side2")}
      onkeydown={(e) => onkey(e, "side2")}
    >
      <rect class="hit" x="-46" y="510" width="126" height="165" />
      <path d="M 16 510 L 64 510 L 64 615 C 64 665, 64 665, 16 665 Z" />
    </g>

    <!-- 中键，画在左右键之上 -->
    <g
      class="part"
      class:on={mouse.middle.enabled}
      role="checkbox"
      tabindex="0"
      aria-checked={mouse.middle.enabled}
      aria-label={t("mouse.middle")}
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
          <span class="text">{t(part.labelKey)}</span>
          <span class="state">{describeTrigger(btn)}</span>
        </button>

        {#if btn.enabled}
          <div class="cfg">
            <div class="clicks" role="radiogroup" aria-label={t("mouse.clicksAria", { part: t(part.labelKey) })}>
              {#each CLICK_KEYS as key, i (key)}
                <button
                  type="button"
                  class="seg"
                  class:sel={btn.clicks === i + 1}
                  role="radio"
                  aria-checked={btn.clicks === i + 1}
                  onclick={() => (btn.clicks = i + 1)}
                >
                  {t(key)}
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

<style>
  .picker {
    display: flex;
    align-items: flex-start;
    gap: 18px;

    --line: color-mix(in srgb, var(--text) 58%, var(--surface-2));
    --face: var(--surface);
    --face-hover: color-mix(in srgb, var(--text) 9%, var(--surface));
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
