<script>
  // 鼠标按键选择器；各按键区域为独立可点击的 path。
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

    /* 示意图走「浅色线稿」：轮廓是淡灰线，选中态是主色淡染 + 主色描边，
       避免整块深绿压在面板上。状态在右侧列表里还有圆点与文字重复表达。 */
    --line: var(--illustration-line);
    --face: var(--illustration);
    --face-hover: color-mix(in srgb, var(--illustration-line) 14%, var(--illustration));
    --face-on: color-mix(in srgb, var(--accent) 22%, var(--illustration));
    --face-on-hover: color-mix(in srgb, var(--accent) 34%, var(--illustration));
    --line-on: color-mix(in srgb, var(--accent) 60%, var(--illustration));
  }

  .mouse {
    flex: none;
    width: 132px;
    height: auto;
    overflow: visible;
  }

  .shell {
    fill: color-mix(in srgb, var(--illustration-edge) 45%, var(--illustration));
    stroke: var(--line);
    stroke-width: 18;
  }

  .part {
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
      fill var(--dur-fast) var(--ease-standard),
      stroke var(--dur-fast) var(--ease-standard);
  }
  .part:hover :is(rect, path):not(.hit) {
    fill: var(--face-hover);
  }
  .part.on :is(rect, path):not(.hit) {
    fill: var(--face-on);
    stroke: var(--line-on);
  }
  .part.on:hover :is(rect, path):not(.hit) {
    fill: var(--face-on-hover);
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
    border-radius: var(--r-card);
  }
  .row.on {
    border-color: var(--stroke);
    background: var(--card-2);
  }

  .name {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--r-control);
    text-align: left;
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .name:hover {
    background: var(--subtle-hover);
  }
  .name:active {
    background: var(--subtle-pressed);
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--text-3);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .row.on .dot {
    background: var(--accent);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .state {
    flex: none;
    font-size: 12px;
    color: var(--text-2);
  }
  .row.on .state {
    color: var(--accent);
  }

  .cfg {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 0 10px 10px 29px;
  }

  /* Win11 分段控件：整体一个圆角壳，选中段填 accent */
  .clicks {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--stroke);
    border-bottom-color: var(--stroke-strong);
    border-radius: var(--r-control);
    background: var(--control);
  }
  .seg {
    padding: 3px 10px;
    border-radius: 2px;
    font-size: 12px;
    color: var(--text-2);
    transition:
      background var(--dur-fast) var(--ease-standard),
      color var(--dur-fast) var(--ease-standard);
  }
  .seg:hover {
    background: var(--subtle-hover);
    color: var(--text);
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
