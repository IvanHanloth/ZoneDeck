<script>
  import { onMount } from "svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ResizeHandles from "./components/ResizeHandles.svelte";
  import BindingPanel from "./components/BindingPanel.svelte";
  import HotkeysPanel from "./components/HotkeysPanel.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import NotificationsPanel from "./components/NotificationsPanel.svelte";
  import AboutPanel from "./components/AboutPanel.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import RestoreWindowsModal from "./components/RestoreWindowsModal.svelte";
  import UpdateModal from "./components/UpdateModal.svelte";
  import AnnouncementModal from "./components/AnnouncementModal.svelte";
  import ErrorReportModal from "./components/ErrorReportModal.svelte";
  import DataNoticeModal from "./components/DataNoticeModal.svelte";
  import Toast from "./components/Toast.svelte";
  import { invoke, onAppEvent, win } from "./lib/ipc.js";
  import {
    app,
    checkForUpdate,
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
    { id: "hotkeys", labelKey: "tab.hotkeys" },
    { id: "notify", labelKey: "tab.notify" },
    { id: "options", labelKey: "tab.options" },
    { id: "about", labelKey: "tab.about" },
  ];

  // 语言偏好改动后立即换文案；核心侧由 save_config 触发的重载配置跟进。
  $effect(() => {
    const pref = app.config?.setting?.language;
    if (pref) setLangPref(pref);
  });

  // 加载阶段不自动保存；loadAll 完成后才武装。普通 let 不参与响应式追踪。
  let autoSaveReady = false;

  // 自动保存：任何配置或绑定改动，停顿后自动写盘（scheduleSave 内部 debounce）。
  $effect(() => {
    const cfg = app.config;
    if (!cfg) return;
    // 深度读取以建立对所有字段的依赖追踪（含 window_rules / process_rules）。
    JSON.stringify($state.snapshot(cfg));
    if (!autoSaveReady) return;
    scheduleSave();
  });

  onMount(() => {
    // main.js 已在挂载前应用过一次，这里只负责跟随系统变化。
    const media = matchMedia("(prefers-color-scheme: dark)");
    const onSystemTheme = () => applyTheme(loadPreference());
    media.addEventListener("change", onSystemTheme);

    // 配置到手后才淡掉启动屏；失败时同样要淡掉，否则会一直卡住。
    loadAll()
      .then(() => {
        autoSaveReady = true;
        checkForUpdate();
        loadAnnouncements({ popNew: true });
      })
      .finally(hideSplash);
    const stopPolling = startStatusPolling(2000);

    // 核心托盘的「窗口恢复工具」/「关于」会带参数拉起本程序：
    // 冷启动时从启动参数读到，已在运行时则由单实例插件发来对应事件。
    invoke("startup_action").then((a) => {
      if (a === "restore") openRestoreTool();
      else if (a === "about") openAboutTab();
    });
    const stopRestoreEvent = onAppEvent("open-restore", openRestoreTool);
    const stopAboutEvent = onAppEvent("open-about", openAboutTab);

    // 首次启动：若核心未运行，自动拉起（仅本次，不与轮询重复）。
    refreshStatus().then(() => {
      if (app.status.running === false) startCore(false);
    });

    // 跟踪最大化状态（控制圆角 / 缩放热区 / 还原按钮图标）。
    let unlisten = () => {};
    win.isMaximized().then((m) => (app.maximized = m));
    win.onResized(async () => {
      app.maximized = await win.isMaximized();
    }).then((fn) => (unlisten = fn));

    return () => {
      stopPolling();
      stopRestoreEvent();
      stopAboutEvent();
      unlisten();
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
    {:else if app.tab === "hotkeys"}
      <HotkeysPanel />
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
  <ErrorReportModal />
  <!-- 放最后：强制更新的遮罩层级最高，压住其它一切弹窗 -->
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
