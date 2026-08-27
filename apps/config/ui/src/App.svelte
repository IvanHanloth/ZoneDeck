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
  import AboutPanel from "./components/AboutPanel.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import RestoreWindowsModal from "./components/RestoreWindowsModal.svelte";
  import UpdateModal from "./components/UpdateModal.svelte";
  import AnnouncementModal from "./components/AnnouncementModal.svelte";
  import ErrorReportModal from "./components/ErrorReportModal.svelte";
  import DataNoticeModal from "./components/DataNoticeModal.svelte";
  import AnalyticsConsentModal from "./components/AnalyticsConsentModal.svelte";
  import BroadRegexModal from "./components/BroadRegexModal.svelte";
  import Toast from "./components/Toast.svelte";
  import { NAV, navItem } from "./lib/nav.js";
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
    refreshLocalizedContent,
    refreshStatus,
    scheduleSave,
    startCore,
    startStatusPolling,
    trackFeatures,
  } from "./lib/state.svelte.js";
  import { flush as flushAnalytics } from "./lib/analytics.js";
  import { resolve, setLangPref, t } from "./lib/i18n.svelte.js";
  import { applyTheme, loadPreference } from "./lib/theme.js";

  // 双栏页要撑满可用高度，其余页按内容自然增高后滚动。
  const FILL_TABS = ["binding", "whitelist"];

  const pageTitle = $derived(t(navItem(app.tab).labelKey));
  const PageIcon = $derived(navItem(app.tab).icon);
  const fill = $derived(FILL_TABS.includes(app.tab));

  // 导航折叠：窄窗口自动收起，点汉堡后以用户意愿为准。
  let innerWidth = $state(globalThis.innerWidth ?? 1024);
  let navOverride = $state(null);
  const navCollapsed = $derived(navOverride ?? innerWidth < 820);

  // 语言偏好改动后立即换文案；核心侧由 save_config 触发的重载配置跟进。
  // 首次由 loadAll 拉取，之后每次真的换了语言才按新语言重取服务端内容。
  let appliedLang = null;
  $effect(() => {
    const pref = app.config?.setting?.language;
    if (!pref) return;
    setLangPref(pref);
    const applied = resolve(pref, globalThis.navigator?.language);
    if (appliedLang === applied) return;
    if (appliedLang !== null) refreshLocalizedContent();
    appliedLang = applied;
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

    // 托盘直达：冷启动时从启动参数读，已在运行时由单实例插件发来事件。
    invoke("startup_action").then((a) => {
      if (a === "restore") openRestoreTool();
      else if (a === "about") openAboutTab();
    });

    loadAll().then(() => {
      autoSaveReady = true;
      // 配置读完才知道用户授权没有，埋点一律排在它后面。
      trackFeatures();
      checkForUpdate();
      loadAnnouncements({ popNew: true });
    });
    const stopPolling = startStatusPolling(2000);
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
    // 收尾走完之后自己调 close，那一次不能再拦，否则关不掉。
    let closing = false;
    const closeReg = win.onCloseRequested(async (e) => {
      if (closing) return;
      e.preventDefault();
      // 攒着的埋点要赶在进程消失前送出去。
      await flushAnalytics();
      // 写盘失败就留在窗口里，再次关闭时重来一遍。
      if (hasUnsavedChanges() && !(await flushSave())) return;
      closing = true;
      win.close();
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
      <!-- 标题不进 key 块：同 Win11 设置，换页时标题直接切换，不跟正文动画。 -->
      <h1 class="type-subtitle page-title" class:wide={fill}>
        <PageIcon width="22" height="22" />
        {pageTitle}
      </h1>
      <!-- key 到 tab 上：换页时正文整块重建，进场动画随之重播，
           滚动容器也一并新建，新页面自然从顶部开始看。 -->
      {#key app.tab}
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
  <AnalyticsConsentModal />
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

  /* 换页进场：正文整块从下往上滑出来，全程不透明，没有渐显；标题不动。

     位移刻意走 relative + top，不用 translate：任何非 none 的 transform／translate
     都会让元素变成 position:fixed 后代的包含块，页内的下拉、录制弹窗、Issue 弹窗
     就会以这里为基准定位而错位（fill-mode 留下的 translate:0 同样算数）。

     page-appear 只用来在开场那段空档里藏住内容，step-start 让它在延迟结束的一瞬
     整块显形。这里换成任何有时长的 opacity 过渡都会变回渐显。

     top: 0 不能省：关键帧只给了 from，隐式的 to 取元素自身的值，不写就是 auto，
     而长度与 auto 之间没法插值，浏览器会退化成在中点硬翻一次——那样根本没有
     滑动。起始距离同理必须写成字面量，关键帧里的 var() 也插值不了。 */
  .page-inner {
    position: relative;
    top: 0;
    animation:
      page-slide var(--dur-page) var(--ease-out) var(--delay-page) both,
      page-appear var(--dur-page) step-start var(--delay-page) both;
  }
  @keyframes page-slide {
    from {
      top: 36px;
    }
  }
  @keyframes page-appear {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .page-inner {
      animation: none;
    }
  }
</style>
