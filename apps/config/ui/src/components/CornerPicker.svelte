<script>
  import { CORNERS, buildTimeline, enabledParts } from "../lib/pointer.js";

  let { setting } = $props();

  const picked = $derived(enabledParts(CORNERS, setting));
  const frames = $derived(
    buildTimeline(picked, setting.allow_move_restore, setting.corner_fast_only),
  );

  let step = $state(0);

  // 选中的角 / 恢复方式一变，演示从头开始。
  $effect(() => {
    frames.length;
    step = 0;
  });

  $effect(() => {
    if (frames.length === 0) return;
    const timer = setTimeout(
      () => (step = (step + 1) % frames.length),
      frames[step % frames.length].ms,
    );
    return () => clearTimeout(timer);
  });

  const IDLE = { cursor: [160, 92], visible: true, corner: null, caption: "" };
  const frame = $derived(frames.length ? frames[step % frames.length] : IDLE);

  function toggle(key) {
    setting[key] = !setting[key];
  }

  function onkey(e, key) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      toggle(key);
    }
  }

  const R = 90;
  const RADIUS_OFFSET = 0.707; // R / (R√2)
  const ZONES = {
    top_left_hide: {
      hit: { x: 8, y: 8, w: 152, h: 84 },
      wedge: { x: 8, y: 8 },
      c: { cx: 8 + R, cy: 8 + R },
    },
    top_right_hide: {
      hit: { x: 160, y: 8, w: 152, h: 84 },
      wedge: { x: 312 - R, y: 8 },
      c: { cx: 312 - R, cy: 8 + R },
    },
    bottom_left_hide: {
      hit: { x: 8, y: 92, w: 152, h: 84 },
      wedge: { x: 8, y: 176 - R },
      c: { cx: 8 + R, cy: 176 - R },
    },
    bottom_right_hide: {
      hit: { x: 160, y: 92, w: 152, h: 84 },
      wedge: { x: 312 - R, y: 176 - R },
      c: { cx: 312 - R, cy: 176 - R },
    },
  };
</script>

<div class="picker">
  <svg class="screen" viewBox="0 0 320 210" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <defs>
      <clipPath id="screen-clip">
        <rect x="8" y="8" width="304" height="168" rx="12" ry="12" />
      </clipPath>
      <!--
        每个角一对渐变（选中蓝 / 未选中灰），球心是该角圆弧的圆心、半径 R√2 直抵方块的尖角。
        圆角矩形之内（offset ≤ 0.707）一片纯透明，出了圆弧才从透明慢慢浓起来。
      -->
      {#each CORNERS as corner (corner.key)}
        {@const c = ZONES[corner.key].c}
        {#each ["on", "off"] as state (state)}
          <radialGradient
            id="corner-{corner.key}-{state}"
            gradientUnits="userSpaceOnUse"
            cx={c.cx}
            cy={c.cy}
            r={R * Math.SQRT2}
          >
            <stop class="s0 {state}" offset="0" />
            <stop class="s1 {state}" offset={RADIUS_OFFSET} />
            <stop class="s2 {state}" offset="0.88" />
            <stop class="s3 {state}" offset="1" />
          </radialGradient>
        {/each}
      {/each}
    </defs>

    <rect class="panel" x="8" y="8" width="304" height="168" rx="12" ry="12" />

    <g clip-path="url(#screen-clip)">
      {#each CORNERS as corner (corner.key)}
        {@const z = ZONES[corner.key]}
        <g
          class="zone"
          class:on={setting[corner.key]}
          class:active={frame.corner === corner.key}
          role="checkbox"
          tabindex="0"
          aria-checked={setting[corner.key]}
          aria-label={corner.label}
          onclick={() => toggle(corner.key)}
          onkeydown={(e) => onkey(e, corner.key)}
        >
          <rect class="hit" x={z.hit.x} y={z.hit.y} width={z.hit.w} height={z.hit.h} />
          <rect
            class="wedge"
            x={z.wedge.x}
            y={z.wedge.y}
            width={R}
            height={R}
            fill="url(#corner-{corner.key}-{setting[corner.key] ? 'on' : 'off'})"
          />
        </g>
      {/each}

      <g class="win" class:gone={!frame.visible}>
        <rect class="win-body" x="120" y="54" width="80" height="76" rx="6" ry="6" />
        <rect class="win-bar" x="120" y="54" width="80" height="14" rx="6" ry="6" />
        <circle class="win-dot" cx="129" cy="61" r="2" />
        <circle class="win-dot" cx="137" cy="61" r="2" />
        <circle class="win-dot" cx="145" cy="61" r="2" />
        <rect class="win-line" x="128" y="78" width="64" height="5" rx="2.5" />
        <rect class="win-line" x="128" y="90" width="50" height="5" rx="2.5" />
        <rect class="win-line" x="128" y="102" width="58" height="5" rx="2.5" />
        <rect class="win-line" x="128" y="114" width="36" height="5" rx="2.5" />
      </g>
    </g>

    <rect class="stand" x="140" y="176" width="40" height="16" />
    <rect class="stand" x="112" y="192" width="96" height="10" rx="5" ry="5" />

    <!-- fast 帧是「甩」，位移动画又快又冲 -->
    <g
      class="cursor"
      class:idle={picked.length === 0}
      class:fast={frame.fast}
      style="--x: {frame.cursor[0]}px; --y: {frame.cursor[1]}px"
    >
      <path
        d="M0 0 L0 20 L5 15.5 L8.5 23 L12 21.3 L8.6 14 L15 13.4 Z"
        transform="translate(-7,-9)"
      />
    </g>
  </svg>

  <p class="caption" class:dim={picked.length === 0}>
    {#if picked.length === 0}
      点击屏幕的任意一角即可启用
    {:else}
      {frame.caption}
    {/if}
  </p>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .screen {
    width: 100%;
    max-width: 320px;
    height: auto;
    align-self: center;
  }

  .panel {
    fill: var(--surface);
    stroke: var(--border);
    stroke-width: 2;
  }
  .stand {
    fill: var(--border);
  }

  /* 圆角矩形之内整片全透明（s0 == s1）→ 出了圆弧也从透明起步，到角上的尖角才最浓（s3） */
  .s0.off,
  .s1.off {
    stop-color: var(--muted);
    stop-opacity: 0;
  }
  .s2.off {
    stop-color: var(--muted);
    stop-opacity: 0.12;
  }
  .s3.off {
    stop-color: var(--muted);
    stop-opacity: 0.35;
  }
  .s0.on,
  .s1.on {
    stop-color: var(--accent);
    stop-opacity: 0;
  }
  .s2.on {
    stop-color: var(--accent);
    stop-opacity: 0.3;
  }
  .s3.on {
    stop-color: var(--accent);
    stop-opacity: 0.75;
  }

  .zone {
    cursor: pointer;
    outline: none;
    transition: opacity 0.18s;
  }
  /* 整个象限都能点，但只有角上那块方块画渐变 */
  .hit {
    fill: transparent;
    pointer-events: all;
  }
  .zone:hover {
    opacity: 0.75;
  }
  /* 正在演示的角落加亮一档，和配文对得上 */
  .zone.on.active {
    opacity: 1;
    filter: brightness(1.15);
  }
  .zone:focus-visible .hit {
    stroke: var(--accent);
    stroke-width: 2;
    stroke-dasharray: 6 4;
  }

  .win {
    pointer-events: none; /* 别挡住底下的角落热区 */
    transform-box: view-box;
    transform-origin: 160px 130px; /* 窗口底边，收起时往下缩 */
    transition:
      transform 0.5s ease-in,
      opacity 0.45s ease-in;
  }
  .win.gone {
    transform: translateY(14px) scale(0.88);
    opacity: 0;
  }
  .win-body {
    fill: var(--surface);
    stroke: var(--border);
    stroke-width: 2;
  }
  .win-bar {
    fill: var(--border);
  }
  .win-dot {
    fill: var(--muted);
    opacity: 0.7;
  }
  .win-line {
    fill: var(--border);
  }

  .cursor {
    pointer-events: none;
    translate: var(--x) var(--y);
    transition: translate 0.75s cubic-bezier(0.4, 0, 0.2, 1);
  }
  /* 「快速移动」：一记加速冲进角落，和核心那边的速度门槛对得上 */
  .cursor.fast {
    transition:
      translate 0.22s cubic-bezier(0.7, 0, 0.9, 0.3),
      filter 0.22s;
    filter: blur(0.6px);
  }
  .cursor path {
    fill: var(--text);
    stroke: var(--surface);
    stroke-width: 1.5;
    stroke-linejoin: round;
  }
  .cursor.idle {
    opacity: 0.35;
  }

  .caption {
    min-height: 20px;
    text-align: center;
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.6;
  }
  .caption.dim {
    color: var(--muted);
  }
</style>
