<script>
  import { onMount } from "svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ResizeHandles from "./components/ResizeHandles.svelte";
  import BindingPanel from "./components/BindingPanel.svelte";
  import WhitelistPanel from "./components/WhitelistPanel.svelte";
  import HotkeysPanel from "./components/HotkeysPanel.svelte";
  import PowerPanel from "./components/PowerPanel.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import NotificationsPanel from "./components/NotificationsPanel.svelte";
  import AboutPanel from "./components/AboutPanel.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import RestoreWindowsModal from "./components/RestoreWindowsModal.svelte";
  import UpdateModal from "./components/UpdateModal.svelte";
  import AnnouncementModal from "./components/AnnouncementModal.svelte";
  import ErrorReportModal from "./components/ErrorReportModal.svelte";
  import DataNoticeModal from "./components/DataNoticeModal.svelte";
  import BroadRegexModal from "./components/BroadRegexModal.svelte";
  import Toast from "./components/Toast.svelte";
  import { invoke, onAppEvent, win } from "./lib/ipc.js";
  import {
    app,
    checkForUpdate,
    flushSave,
    hasUnsavedChanges,
    loadAll,
    loadAnnouncements,
    openAboutTab,
    openRestoreTool,
    refreshStatus,
    scheduleSave,
    startCore,
    startStatusPolling,
  } from "./lib/state.svelte.js";
  import { hideSplash } from "./lib/splash.js";
  import { setLangPref, t } from "./lib/i18n.svelte.js";
  import { applyTheme, loadPreference } from "./lib/theme.js";

  const TABS = [
    { id: "binding", labelKey: "tab.binding" },
    { id: "whitelist", labelKey: "tab.whitelist" },
    { id: "hotkeys", labelKey: "tab.hotkeys" },
    { id: "notify", labelKey: "tab.notify" },
    { id: "power", labelKey: "tab.power" },
    { id: "options", labelKey: "tab.options" },
    { id: "about", labelKey: "tab.about" },
  ];

  // 语言偏好改动后立即换文案；核心侧由 save_config 触发的重载配置跟进。
  $effect(() => {
    const pref = app.config?.setting?.language;
    if (pref) setLangPref(pref);
  });

  // 加载阶段不自动保存；loadAll 完成后才武装。
  let autoSaveReady = false;

  // 任何配置或绑定改动，停顿后自动写盘。
  $effect(() => {
    const cfg = app.config;
    if (!cfg) return;
    // 深度读取以建立对所有字段的依赖追踪。
    JSON.stringify($state.snapshot(cfg));
    if (!autoSaveReady) return;
    scheduleSave();
  });

  onMount(() => {
    // main.js 已在挂载前应用过一次，此处只跟随系统变化。
    const media = matchMedia("(prefers-color-scheme: dark)");
    const onSystemTheme = () => applyTheme(loadPreference());
    media.addEventListener("change", onSystemTheme);

    // 配置到手后才淡掉启动屏；失败时同样要淡掉。
    loadAll()
      .then(() => {
        autoSaveReady = true;
        checkForUpdate();
        loadAnnouncements({ popNew: true });
      })
      .finally(hideSplash);
    const stopPolling = startStatusPolling(2000);

    // 托盘直达：冷启动时从启动参数读，已在运行时由单实例插件发来事件。
    invoke("startup_action").then((a) => {
      if (a === "restore") openRestoreTool();
      else if (a === "about") openAboutTab();
    });
    const stopRestoreEvent = onAppEvent("open-restore", openRestoreTool);
    const stopAboutEvent = onAppEvent("open-about", openAboutTab);

    // 首次启动时若核心未运行则自动拉起。
    refreshStatus().then(() => {
      if (app.status.running === false) startCore(false);
    });

    // 跟踪最大化状态，控制圆角 / 缩放热区 / 还原按钮图标。
    win.isMaximized().then((m) => (app.maximized = m));
    const resizedReg = win.onResized(async () => {
      app.maximized = await win.isMaximized();
    });

    // 关窗前把未落盘的改动写完；写盘失败时留在窗口，再次关闭不再阻拦。
    const closeReg = win.onCloseRequested(async (e) => {
      if (!hasUnsavedChanges()) return;
      e.preventDefault();
      if (await flushSave()) win.close();
    });

    return () => {
      stopPolling();
      stopRestoreEvent();
      stopAboutEvent();
      // 清理挂在注册的 promise 上，卸载先于注册完成时监听器也能解除。
      resizedReg.then((fn) => fn());
      closeReg.then((fn) => fn());
      media.removeEventListener("change", onSystemTheme);
    };
  });
</script>

<div class="window" class:maximized={app.maximized}>
  <TitleBar />

  <div class="tabs" role="tablist" aria-label={t("app.tabsAria")}>
    {#each TABS as tab (tab.id)}
      <button
        class="tab"
        class:active={app.tab === tab.id}
        role="tab"
        aria-selected={app.tab === tab.id}
        onclick={() => (app.tab = tab.id)}
      >
        {t(tab.labelKey)}
      </button>
    {/each}
  </div>

  <main class="content">
    {#if !app.config}
      <p class="hint loading">{t("app.loadingConfig")}</p>
    {:else if app.tab === "binding"}
      <BindingPanel />
    {:else if app.tab === "whitelist"}
      <WhitelistPanel />
    {:else if app.tab === "hotkeys"}
      <HotkeysPanel />
    {:else if app.tab === "power"}
      <PowerPanel />
    {:else if app.tab === "options"}
      <OptionsPanel />
    {:else if app.tab === "notify"}
      <NotificationsPanel />
    {:else}
      <AboutPanel />
    {/if}
  </main>

  <StatusBar />

  <RestoreWindowsModal bind:open={app.restoreOpen} />
  <AnnouncementModal />
  <DataNoticeModal />
  <BroadRegexModal />
  <ErrorReportModal />
  <!-- 放最后，强制更新的遮罩层级最高 -->
  <UpdateModal />
  <Toast />
  <ResizeHandles />
</div>

<style>
  .window {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .window.maximized {
    border: none;
    border-radius: 0;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 8px 14px 0;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex: none;
    overflow-x: auto;
  }
  .tab {
    padding: 8px 16px 9px;
    border-radius: 8px 8px 0 0;
    color: var(--muted);
    font-weight: 500;
    white-space: nowrap;
    position: relative;
  }
  .tab:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tab.active {
    color: var(--accent);
    font-weight: 600;
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 0;
    height: 2.5px;
    border-radius: 2px 2px 0 0;
    background: var(--accent);
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 14px 16px;
  }
  .content :global(.panel-stack) {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 860px;
    margin: 0 auto;
  }
  .content :global(.opt-grid) {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 2px 18px;
  }
  .content :global(.opt-grid.corners) {
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
  }

  .loading {
    text-align: center;
    padding: 48px 0;
  }

</style>
