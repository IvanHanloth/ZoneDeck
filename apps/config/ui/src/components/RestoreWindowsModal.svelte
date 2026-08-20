<script>
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import WindowList from "./WindowList.svelte";
  import { invoke } from "../lib/ipc.js";
  import { app, toast } from "../lib/state.svelte.js";
  import { applyListFilters } from "../lib/grouping.js";
  import { t } from "../lib/i18n.svelte.js";

  let { open = $bindable(false) } = $props();

  let windows = $state([]);
  let selected = $state([]);
  let search = $state("");
  let showBackground = $state(true); // 恢复工具默认显示后台/已隐藏窗口，便于找回。
  let showUntitled = $state(false);
  let busy = $state(false);

  // 过滤在父层完成后再交给 WindowList。
  const shown = $derived(
    applyListFilters(windows, { showBackground, showUntitled, search }),
  );

  async function load() {
    windows = await invoke("list_windows");
  }

  $effect(() => {
    if (open) {
      selected = [];
      load();
    }
  });

  /** 选中的句柄映射到去重后的 PID。 */
  function selectedPids() {
    const pids = windows
      .filter((w) => selected.includes(w.hwnd))
      .map((w) => w.PID)
      .filter((pid) => pid);
    return [...new Set(pids)];
  }

  // 冻结 / 解冻跟随「能效控制」里的设置。
  const freezeArgs = $derived({
    enhanced: !!(app.config?.setting?.enhanced_freeze && app.pssuspend),
    scope: app.config?.setting?.power_scope ?? "self",
  });

  async function run(label, fn) {
    if (busy) return;
    busy = true;
    try {
      await fn();
      await load();
    } catch (err) {
      toast(t("restore.actionFailed", { action: label, err }), true);
    } finally {
      busy = false;
    }
  }

  function showSel() {
    if (selected.length === 0) return toast(t("restore.pickFirst"), true);
    run(t("restore.showWindows"), async () => {
      await invoke("show_windows", { hwnds: selected });
      toast(t("restore.shown"));
    });
  }

  function hideSel() {
    if (selected.length === 0) return toast(t("restore.pickFirst"), true);
    run(t("restore.hideWindows"), async () => {
      await invoke("hide_windows", { hwnds: selected });
      toast(t("restore.hidden"));
    });
  }

  function freezeSel() {
    const pids = selectedPids();
    if (pids.length === 0) return toast(t("restore.pickFirst"), true);
    run(t("restore.freezeProcesses"), async () => {
      await invoke("freeze_pids", { pids, ...freezeArgs });
      toast(t("restore.frozen", { n: pids.length }));
    });
  }

  function resumeSel() {
    const pids = selectedPids();
    if (pids.length === 0) return toast(t("restore.pickFirst"), true);
    run(t("restore.resumeProcesses"), async () => {
      await invoke("resume_pids", { pids, ...freezeArgs });
      toast(t("restore.resumed", { n: pids.length }));
    });
  }
</script>

<ContentDialog title={t("restore.title")} bind:open>
  <p class="hint">
    {t("restore.hintLine1")}<br />
    {t("restore.hintLine2")}
  </p>
  <div class="list-wrap">
    <WindowList
      title={t("restore.allWindows")}
      windows={shown}
      bind:selected
      bind:search
      bind:showBackground
      bind:showUntitled
      onrefresh={load}
    />
  </div>

  {#snippet footer()}
    <button class="btn" onclick={() => (open = false)}>{t("common.close")}</button>
    <span class="spacer"></span>
    <button class="btn" disabled={busy} onclick={hideSel}>{t("restore.hideWindows")}</button>
    <button class="btn" disabled={busy} onclick={freezeSel}>{t("restore.freezeProcesses")}</button>
    <button class="btn" disabled={busy} onclick={resumeSel}>{t("restore.resumeProcesses")}</button>
    <button class="btn primary" disabled={busy} onclick={showSel}>{t("restore.showWindows")}</button>
  {/snippet}
</ContentDialog>

<style>
  .hint {
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.6;
    margin-bottom: 10px;
  }
  .list-wrap {
    height: 340px;
    display: flex;
  }
  .spacer {
    flex: 1;
  }
</style>
