<script>
  import { onMount } from "svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ResizeHandles from "./components/ResizeHandles.svelte";
  import BindingPanel from "./components/BindingPanel.svelte";
  import HotkeysPanel from "./components/HotkeysPanel.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import AboutPanel from "./components/AboutPanel.svelte";
  import Toggle from "./components/Toggle.svelte";
  import Toast from "./components/Toast.svelte";
  import { invoke, win } from "./lib/ipc.js";
  import {
    app,
    loadAll,
    saveConfig,
    startStatusPolling,
    toast,
  } from "./lib/state.svelte.js";
  import { applyTheme, loadPreference } from "./lib/theme.js";

  const TABS = [
    { id: "binding", label: "窗口绑定" },
    { id: "hotkeys", label: "热键与鼠标" },
    { id: "options", label: "其他选项" },
    { id: "about", label: "关于" },
  ];
  let active = $state("binding");

  async function onAutostartChange(e) {
    const enabled = e.target.checked;
    try {
      await invoke("set_autostart", { enabled });
      toast(enabled ? "已开启开机自启" : "已关闭开机自启");
    } catch (err) {
      app.autostart = !enabled; // 失败回滚
      toast("设置开机自启失败：" + err, true);
    }
  }

  onMount(() => {
    // 主题：立即应用并跟随系统变化（auto 模式）。
    applyTheme(loadPreference());
    const media = matchMedia("(prefers-color-scheme: dark)");
    const onSystemTheme = () => applyTheme(loadPreference());
    media.addEventListener("change", onSystemTheme);

    loadAll();
    const stopPolling = startStatusPolling(2000);

    // 跟踪最大化状态（控制圆角/缩放热区/还原按钮图标）。
    let unlisten = () => {};
    win.isMaximized().then((m) => (app.maximized = m));
    win
      .onResized(async () => {
        app.maximized = await win.isMaximized();
      })
      .then((fn) => (unlisten = fn));

    return () => {
      stopPolling();
      unlisten();
      media.removeEventListener("change", onSystemTheme);
    };
  });
</script>

<div class="window" class:maximized={app.maximized}>
  <TitleBar />

  <nav class="tabs" role="tablist" aria-label="设置分类">
    {#each TABS as tab (tab.id)}
      <button
        class="tab"
        class:active={active === tab.id}
        role="tab"
        aria-selected={active === tab.id}
        onclick={() => (active = tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </nav>

  <main class="content">
    {#if !app.config}
      <p class="hint loading">正在加载配置…</p>
    {:else if active === "binding"}
      <BindingPanel />
    {:else if active === "hotkeys"}
      <HotkeysPanel />
    {:else if active === "options"}
      <OptionsPanel />
    {:else}
      <AboutPanel />
    {/if}
  </main>

  <footer class="footer">
    <Toggle
      label="开机自启"
      bind:checked={app.autostart}
      onchange={onAutostartChange}
    />
    <button class="btn primary save" onclick={saveConfig} disabled={!app.config || app.saving}>
      {app.saving ? "保存中…" : "保存设置"}
    </button>
  </footer>

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

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    background: var(--surface);
    border-top: 1px solid var(--border);
    flex: none;
  }
  .save {
    min-width: 110px;
  }
</style>
