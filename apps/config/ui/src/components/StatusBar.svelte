<script>
  import { t } from "../lib/i18n.svelte.js";
  import IconShield from "~icons/lucide/shield";
  import IconCheck from "~icons/lucide/check";
  import IconScrollText from "~icons/lucide/scroll-text";
  import IconRotateCw from "~icons/lucide/rotate-cw";
  import IconPower from "~icons/lucide/power";
  import IconPlay from "~icons/lucide/play";
  import IconContrast from "~icons/lucide/contrast";
  import IconSun from "~icons/lucide/sun";
  import IconMoon from "~icons/lucide/moon";
  import { invoke } from "../lib/ipc.js";
  import { app, startCore, restartCore, quitCore, toast } from "../lib/state.svelte.js";
  import {
    applyTheme,
    loadPreference,
    nextTheme,
    savePreference,
    themeIcon,
    themeLabel,
  } from "../lib/theme.js";

  const running = $derived(app.status.running);
  let themePref = $state(loadPreference());

  const THEME_ICONS = { contrast: IconContrast, sun: IconSun, moon: IconMoon };
  const ThemeIcon = $derived(THEME_ICONS[themeIcon(themePref)] ?? IconContrast);

  function cycleTheme() {
    themePref = nextTheme(themePref);
    savePreference(themePref);
    applyTheme(themePref);
  }

  async function openLog() {
    try {
      await invoke("open_log_dir");
    } catch (err) {
      toast(t("status.openLogFailed", { err }), true);
    }
  }

  // 显示核心回报的真实监控状态。
  const monitoring = $derived(app.status.monitoring);
  const monitorText = $derived(
    app.monitorPending
      ? t("status.monitorSwitching")
      : t(monitoring ? "status.monitorOn" : "status.monitorOff"),
  );
  const monitorTitle = $derived(
    t(monitoring ? "status.monitorOnTitle" : "status.monitorOffTitle"),
  );

  const statusText = $derived(
    running === null
      ? t("status.detecting")
      : t(running ? "status.coreRunning" : "status.coreStopped"),
  );
  const statusClass = $derived(
    running === null ? "pending" : running ? "online" : "offline",
  );
</script>

<footer class="statusbar">
  <div class="left">
    <span class="status {statusClass}">
      {#if running && app.status.elevated}
        <IconShield width="11" height="11" class="shield-dot" />
      {:else}
        <i class="dot"></i>
      {/if}
      {statusText}
    </span>

    {#if running === false}
      <button
        class="act ok"
        onclick={() => startCore(false)}
        title={t("status.startCore")}
        aria-label={t("status.startCore")}
      >
        <IconPlay width="14" height="14" />
      </button>
      <button
        class="act blue"
        onclick={() => startCore(true)}
        title={t("status.startAdmin")}
        aria-label={t("status.startAdmin")}
      >
        <IconShield width="14" height="14" />
      </button>
    {:else if running}
      {#if !app.status.elevated}
        <button
          class="act blue"
          onclick={() => restartCore(true)}
          title={t("status.restartAdmin")}
          aria-label={t("status.restartAdmin")}
        >
          <IconShield width="14" height="14" />
        </button>
      {/if}
      <button
        class="act warn"
        onclick={() => restartCore(app.status.elevated)}
        title={t("status.restartCore")}
        aria-label={t("status.restartCore")}
      >
        <IconRotateCw width="14" height="14" />
      </button>
      <button
        class="act danger"
        onclick={quitCore}
        title={t("status.quitCore")}
        aria-label={t("status.quitCore")}
      >
        <IconPower width="14" height="14" />
      </button>
    {/if}
  </div>

  <div class="right">
    <button
      class="act"
      onclick={openLog}
      title={t("status.openLogDir")}
      aria-label={t("status.openLogDir")}
    >
      <IconScrollText width="14" height="14" />
    </button>

    <button
      class="act"
      onclick={cycleTheme}
      title={themeLabel(themePref)}
      aria-label={themeLabel(themePref)}
    >
      <ThemeIcon width="14" height="14" />
    </button>

    {#if running}
      <span class="monitor" class:paused={!monitoring} title={monitorTitle}>
        <i class="dot"></i>
        {monitorText}
      </span>
    {/if}
    <span class="save" class:saving={app.saving}>
      {#if app.saving}
        {t("status.saving")}
      {:else}
        <IconCheck width="12" height="12" /> {t("status.saved")}
      {/if}
    </span>
  </div>
</footer>

<style>
  .statusbar {
    height: var(--statusbar-h);
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 8px 0 16px;
    background: var(--card);
    border-top: 1px solid var(--divider);
    font-size: 12px;
    line-height: 16px;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .left {
    gap: 8px;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .shield-dot {
    color: var(--ok);
  }
  .status.online {
    color: var(--ok);
  }
  .status.online .dot {
    background: var(--ok);
  }
  .status.offline {
    color: var(--danger);
  }
  .status.offline .dot {
    background: var(--danger);
  }

  /* 状态栏按钮一律无底色，hover 才浮出，避免把细条塞满边框 */
  .act {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 24px;
    border-radius: var(--r-control);
    color: var(--text-2);
    transition:
      background var(--dur-fast) var(--ease-standard),
      color var(--dur-fast) var(--ease-standard);
  }
  .act:hover {
    background: var(--subtle-hover);
    color: var(--text);
  }
  .act:active {
    background: var(--subtle-pressed);
  }

  /* hover 底色由 currentColor 派生：手写 rgba 换主题时会与令牌脱钩 */
  .act.ok {
    color: var(--ok);
  }
  .act.blue {
    color: #3b82f6;
  }
  .act.warn {
    color: var(--warn);
  }
  .act.danger {
    color: var(--danger);
  }
  .act.ok:hover,
  .act.blue:hover,
  .act.warn:hover,
  .act.danger:hover {
    color: currentColor;
    background: color-mix(in srgb, currentColor 12%, transparent);
  }

  .monitor {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: 4px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .monitor .dot {
    background: var(--ok);
  }
  .monitor.paused {
    color: var(--warn);
  }
  .monitor.paused .dot {
    background: var(--warn);
    animation: blink 1.2s ease-in-out infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0.3;
    }
  }

  .save {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: 4px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .save.saving {
    color: var(--accent);
  }

  @media (max-width: 560px) {
    .save,
    .monitor {
      display: none;
    }
  }
</style>
