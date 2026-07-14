<script>
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
      toast("打开日志失败：" + err, true);
    }
  }

  function toggleMenu() {
    menuOpen = !menuOpen;
  }

  function closeMenu() {
    menuOpen = false;
  }

  const statusText = $derived(
    running === null ? "检测中…" : running ? "核心运行中" : "核心未运行",
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
        title="启动核心"
        aria-label="启动核心"
      >
        <IconPlay width="14" height="14" />
      </button>
      <button
        class="act icon-only blue"
        onclick={() => startCore(true)}
        title="管理员启动"
        aria-label="管理员启动"
      >
        <IconShield width="14" height="14" />
      </button>
    {:else if running}
      {#if !app.status.elevated}
        <button
          class="act icon-only blue"
          onclick={() => restartCore(true)}
          title="管理员身份重启"
          aria-label="管理员身份重启"
        >
          <IconShield width="14" height="14" />
        </button>
      {/if}
      <button
        class="act icon-only warn"
        onclick={() => restartCore(app.status.elevated)}
        title="重启核心"
        aria-label="重启核心"
      >
        <IconRotateCw width="14" height="14" />
      </button>
      <button class="act icon-only danger" onclick={quitCore} title="退出核心" aria-label="退出核心">
        <IconPower width="14" height="14" />
      </button>
    {/if}
  </div>

  <div class="right">
    <button class="act icon" onclick={openLog} title="打开日志目录" aria-label="打开日志目录">
      <IconScrollText width="14" height="14" />
    </button>

    <button class="act icon" onclick={cycleTheme} title={themeLabel(themePref)} aria-label={themeLabel(themePref)}>
      {themeIcon(themePref)}
    </button>

    <span class="save" class:saving={app.saving}>
      {#if app.saving}保存中…{:else}<IconCheck width="12" height="12" /> 已保存{/if}
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

  /* 纯图标按钮：正方形点击区 */
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
    .save {
      display: none;
    }
  }
</style>
