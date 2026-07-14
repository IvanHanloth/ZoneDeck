<script>
  import { onMount } from "svelte";
  import IconShield from "~icons/lucide/shield";
  import IconCheck from "~icons/lucide/check";
  import IconScrollText from "~icons/lucide/scroll-text";
  import IconRotateCw from "~icons/lucide/rotate-cw";
  import IconPower from "~icons/lucide/power";
  import IconPlay from "~icons/lucide/play";
  import IconChevronDown from "~icons/lucide/chevron-down";
  import { invoke } from "../lib/ipc.js";
  import { app, startCore, restartCore, quitCore, toast } from "../lib/state.svelte.js";

  const running = $derived(app.status.running);
  let menuOpen = $state(false);
  let menuContainer;

  onMount(() => {
    function handleClickOutside(e) {
      if (menuOpen && menuContainer && !menuContainer.contains(e.target)) {
        menuOpen = false;
      }
    }
    document.addEventListener("click", handleClickOutside);
    return () => document.removeEventListener("click", handleClickOutside);
  });

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
        <IconShield width="7" height="7" class="shield-dot" />
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
      <button
        class="act icon-only blue"
        onclick={() => restartCore(app.status.elevated)}
        title="重启核心"
        aria-label="重启核心"
      >
        <IconRotateCw width="14" height="14" />
      </button>
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
      <button class="act icon-only warn" onclick={() => {}} title="暂停" aria-label="暂停">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
          <rect x="6" y="3" width="4" height="18" rx="1" />
          <rect x="14" y="3" width="4" height="18" rx="1" />
        </svg>
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

    <div class="menu-container" bind:this={menuContainer}>
      <button
        class="act icon"
        class:active={menuOpen}
        onclick={() => menuOpen = !menuOpen}
        title="更多选项"
        aria-label="更多选项"
      >
        <IconChevronDown width="14" height="14" />
      </button>
      {#if menuOpen}
        <div class="dropdown-menu">
          <label class="menu-item">
            <input type="checkbox" bind:checked={app.showBackground} />
            <span>后台进程</span>
          </label>
          <label class="menu-item">
            <input type="checkbox" bind:checked={app.showUntitled} />
            <span>无标题窗口</span>
          </label>
        </div>
      {/if}
    </div>

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

  .menu-container {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    bottom: 100%;
    right: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    padding: 4px 0;
    margin-bottom: 6px;
    min-width: 140px;
    z-index: 100;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    cursor: pointer;
    user-select: none;
    color: var(--text);
    font-size: 12px;
    transition: background 0.12s;
  }
  .menu-item:hover {
    background: var(--hover);
  }
  .menu-item input[type="checkbox"] {
    cursor: pointer;
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    flex: none;
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
