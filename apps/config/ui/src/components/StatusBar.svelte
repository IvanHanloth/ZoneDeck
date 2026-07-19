<script>
  import { t } from "../lib/i18n.svelte.js";
  import IconShield from "~icons/lucide/shield";
  import IconCheck from "~icons/lucide/check";
  import IconScrollText from "~icons/lucide/scroll-text";
  import IconRotateCw from "~icons/lucide/rotate-cw";
  import IconPower from "~icons/lucide/power";
  import IconPlay from "~icons/lucide/play";
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

  // 显示核心回报的真实监控状态（core_status.monitoring）。
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
        <IconShield width="10" height="10" class="shield-dot" />
      {:else}
        <i class="dot"></i>
      {/if}
      {statusText}
    </span>

    {#if running === false}
      <button
        class="act icon-only ok"
        onclick={() => startCore(false)}
        title={t("status.startCore")}
        aria-label={t("status.startCore")}
      >
        <IconPlay width="14" height="14" />
      </button>
      <button
        class="act icon-only blue"
        onclick={() => startCore(true)}
        title={t("status.startAdmin")}
        aria-label={t("status.startAdmin")}
      >
        <IconShield width="14" height="14" />
      </button>
    {:else if running}
      {#if !app.status.elevated}
        <button
          class="act icon-only blue"
          onclick={() => restartCore(true)}
          title={t("status.restartAdmin")}
          aria-label={t("status.restartAdmin")}
        >
          <IconShield width="14" height="14" />
        </button>
      {/if}
      <button
        class="act icon-only warn"
        onclick={() => restartCore(app.status.elevated)}
        title={t("status.restartCore")}
        aria-label={t("status.restartCore")}
      >
        <IconRotateCw width="14" height="14" />
      </button>
      <button class="act icon-only danger" onclick={quitCore} title={t("status.quitCore")} aria-label={t("status.quitCore")}>
        <IconPower width="14" height="14" />
      </button>
    {/if}
  </div>

  <div class="right">
    <button class="act icon" onclick={openLog} title={t("status.openLogDir")} aria-label={t("status.openLogDir")}>
      <IconScrollText width="14" height="14" />
    </button>

    <button class="act icon" onclick={cycleTheme} title={themeLabel(themePref)} aria-label={themeLabel(themePref)}>
      {themeIcon(themePref)}
    </button>

    {#if running}
      <span class="monitor" class:paused={!monitoring} title={monitorTitle}>
        <i class="dot"></i>
        {monitorText}
      </span>
    {/if}
    <span class="save" class:saving={app.saving}>
      {#if app.saving}{t("status.saving")}{:else}<IconCheck width="12" height="12" /> {t("status.saved")}{/if}
    </span>
  </div>
</footer>

<style>
  .statusbar {
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 12px;
    background: var(--surface);
    border-top: 1px solid var(--border);
    font-size: 12px;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    white-space: nowrap;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
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

  .act {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: 5px;
    font-size: 12px;
    color: var(--text);
    border: 1px solid var(--border);
    background: var(--surface-2);
    transition: background 0.12s, border-color 0.12s, color 0.12s;
  }
  .act:hover {
    background: var(--hover);
    border-color: var(--accent);
  }

  .act.icon {
    padding: 3px;
    width: 24px;
    height: 22px;
    justify-content: center;
    color: var(--muted);
    position: relative;
  }
  .act.icon:hover {
    color: var(--text);
  }
  .act.icon.active {
    color: var(--accent);
    background: var(--hover);
    border-color: var(--accent);
  }

  .act.icon-only {
    padding: 2px;
    width: 24px;
    height: 22px;
    justify-content: center;
    border: 1px solid transparent;
  }
  .act.icon-only:hover {
    border-color: currentColor;
  }

  .act.ok {
    color: var(--ok);
  }
  .act.ok:hover {
    background: rgba(47, 158, 99, 0.1);
  }

  .act.blue {
    color: #3b82f6;
  }
  .act.blue:hover {
    background: rgba(59, 130, 246, 0.1);
  }

  .act.warn {
    color: var(--warn);
  }
  .act.warn:hover {
    background: rgba(217, 119, 6, 0.1);
  }

  .act.danger {
    color: var(--danger);
  }
  .act.danger:hover {
    background: rgba(229, 72, 77, 0.1);
  }

  .monitor {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
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
    color: var(--muted);
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
