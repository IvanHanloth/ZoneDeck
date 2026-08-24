<script>
  import { onMount } from "svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import NavView from "./components/NavView.svelte";
  import ResizeHandles from "./components/ResizeHandles.svelte";
  import BindingPanel from "./components/BindingPanel.svelte";
  import WhitelistPanel from "./components/WhitelistPanel.svelte";
  import HotkeysPanel from "./components/HotkeysPanel.svelte";
  import PowerPanel from "./components/PowerPanel.svelte";
  import HidePanel from "./components/HidePanel.svelte";
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
  import IconAppWindow from "~icons/lucide/app-window";
  import IconShieldCheck from "~icons/lucide/shield-check";
  import IconKeyboard from "~icons/lucide/keyboard";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconBell from "~icons/lucide/bell";
  import IconZap from "~icons/lucide/zap";
  import IconSettings from "~icons/lucide/settings";
  import IconInfo from "~icons/lucide/info";
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
  import { setLangPref, t } from "./lib/i18n.svelte.js";
  import { applyTheme, loadPreference } from "./lib/theme.js";

  const NAV = [
    { id: "binding", labelKey: "tab.binding", icon: IconAppWindow },
    { id: "whitelist", labelKey: "tab.whitelist", icon: IconShieldCheck },
    { id: "hotkeys", labelKey: "tab.hotkeys", icon: IconKeyboard },
    { id: "hide", labelKey: "tab.hide", icon: IconEyeOff },
    { id: "power", labelKey: "tab.power", icon: IconZap },
    { id: "notify", labelKey: "tab.notify", icon: IconBell },
    { id: "about", labelKey: "tab.about", icon: IconInfo, footer: true },
    { id: "options", labelKey: "tab.options", icon: IconSettings, footer: true },
  ];

  // 双栏页要撑满可用高度，其余页按内容自然增高后滚动。
  const FILL_TABS = ["binding", "whitelist"];

  const pageTitle = $derived(
    t(NAV.find((n) => n.id === app.tab)?.labelKey ?? "tab.binding"),
  );
  const PageIcon = $derived(
    NAV.find((n) => n.id === app.tab)?.icon ?? IconAppWindow,
  );
  const fill = $derived(FILL_TABS.includes(app.tab));

  // 导航折叠：窄窗口自动收起，点汉堡后以用户意愿为准。
  let innerWidth = $state(globalThis.innerWidth ?? 1024);
  let navOverride = $state(null);
  const navCollapsed = $derived(navOverride ?? innerWidth < 820);

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

    loadAll().then(() => {
      autoSaveReady = true;
      checkForUpdate();
      loadAnnouncements({ popNew: true });
    });
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

<svelte:window bind:innerWidth />

<div class="window" class:maximized={app.maximized}>
  <TitleBar />

  <div class="shell">
    <NavView
      items={NAV}
      bind:active={app.tab}
      collapsed={navCollapsed}
      ariaLabel={t("app.tabsAria")}
      ontoggle={() => (navOverride = !navCollapsed)}
    />

    <main class="content">
      <!-- key 到 tab 上：换页时这一整块重建，进场动画随之重播，
           滚动容器也一并新建，新页面自然从顶部开始看。 -->
      {#key app.tab}
        <h1 class="type-subtitle page-title" class:wide={fill}>
          <PageIcon width="22" height="22" />
          {pageTitle}
        </h1>
        <div class="page-body" class:fill>
          <div class="page-inner">
            {#if !app.config}
              <p class="hint loading">{t("app.loadingConfig")}</p>
            {:else if app.tab === "binding"}
              <BindingPanel />
            {:else if app.tab === "whitelist"}
              <WhitelistPanel />
            {:else if app.tab === "hotkeys"}
              <HotkeysPanel />
            {:else if app.tab === "hide"}
              <HidePanel />
            {:else if app.tab === "power"}
              <PowerPanel />
            {:else if app.tab === "options"}
              <OptionsPanel />
            {:else if app.tab === "notify"}
              <NotificationsPanel />
            {:else}
              <AboutPanel />
            {/if}
          </div>
        </div>
      {/key}
    </main>
  </div>

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
    border: 1px solid var(--stroke);
    border-radius: var(--r-window);
    overflow: hidden;
  }
  .window.maximized {
    border: none;
    border-radius: 0;
  }

  .shell {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  /* 标题固定，只有下方内容滚动。左右留白算上 page-body 的滚动条槽，与卡片左缘对齐。 */
  .page-title {
    --gutter: calc(var(--content-pad) + var(--scrollbar-w));
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    width: min(100%, calc(var(--content-max) + var(--gutter) * 2));
    margin: 0 auto;
    padding: 2px var(--gutter) 14px;
  }
  .page-title.wide {
    width: min(100%, calc(var(--content-max-wide) + var(--gutter) * 2));
  }
  /* 滚动条贴内容区右缘，不跟着居中的内容走。
     both-edges 让两侧都预留滚动条的位置，居中的内容才真的对称，
     标题也才和卡片左缘对得上（否则一律偏左半个滚动条宽）。 */
  .page-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-gutter: stable both-edges;
    padding: 0 var(--content-pad) var(--content-pad);
  }
  .page-body.fill {
    overflow: hidden;
  }
  .page-inner {
    width: 100%;
    max-width: var(--content-max);
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .page-body.fill .page-inner {
    height: 100%;
    max-width: var(--content-max-wide);
  }

  .loading {
    text-align: center;
    padding: 48px 0;
  }

  /* 换页进场：整块内容从下方浮上来并淡入，同 Win11 设置的页面切换。
     标题与正文同时起步，读起来是一整页换上来，而不是两块各自动。

     位移刻意走 relative + top，不用 translate：任何非 none 的 transform／translate
     都会让元素变成 position:fixed 后代的包含块，页内的下拉、录制弹窗、Issue 弹窗
     就会以这里为基准定位而错位（fill-mode 留下的 translate:0 同样算数）。 */
  .page-title,
  .page-inner {
    position: relative;
    animation: page-enter var(--dur-slow) var(--ease-standard) both;
  }
  @keyframes page-enter {
    from {
      opacity: 0;
      top: 24px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .page-title,
    .page-inner {
      animation: none;
    }
  }
</style>
