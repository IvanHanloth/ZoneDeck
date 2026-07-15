<script>
  import Modal from "./Modal.svelte";
  import WindowList from "./WindowList.svelte";
  import { invoke } from "../lib/ipc.js";
  import { app, toast } from "../lib/state.svelte.js";
  import { applyListFilters } from "../lib/grouping.js";

  let { open = $bindable(false) } = $props();

  let windows = $state([]);
  let selected = $state([]);
  let search = $state("");
  let showBackground = $state(true); // 恢复工具默认显示后台/已隐藏窗口，便于找回。
  let showUntitled = $state(false);
  let busy = $state(false);

  // 过滤在父层完成后再交给 WindowList（与 BindingPanel 一致），使搜索/后台过滤真正生效。
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

  /** 选中的句柄映射到去重后的 PID（冻结/解冻按进程粒度）。 */
  function selectedPids() {
    const pids = windows
      .filter((w) => selected.includes(w.hwnd))
      .map((w) => w.PID)
      .filter((pid) => pid);
    return [...new Set(pids)];
  }

  // 冻结/解冻跟随全局设置：增强冻结可用即用、遵循「冻结完整进程」。
  const freezeArgs = $derived({
    enhanced: !!(app.config?.setting?.enhanced_freeze && app.pssuspend),
    whole_tree: !!app.config?.setting?.freeze_whole_tree,
  });

  async function run(label, fn) {
    if (busy) return;
    busy = true;
    try {
      await fn();
      await load();
    } catch (err) {
      toast(label + "失败：" + err, true);
    } finally {
      busy = false;
    }
  }

  function showSel() {
    if (selected.length === 0) return toast("请先勾选窗口", true);
    run("显示窗口", async () => {
      await invoke("show_windows", { hwnds: selected });
      toast("已尝试显示选中窗口");
    });
  }

  function hideSel() {
    if (selected.length === 0) return toast("请先勾选窗口", true);
    run("隐藏窗口", async () => {
      await invoke("hide_windows", { hwnds: selected });
      toast("已隐藏选中窗口");
    });
  }

  function freezeSel() {
    const pids = selectedPids();
    if (pids.length === 0) return toast("请先勾选窗口", true);
    run("冻结进程", async () => {
      await invoke("freeze_pids", { pids, ...freezeArgs });
      toast(`已冻结 ${pids.length} 个进程`);
    });
  }

  function resumeSel() {
    const pids = selectedPids();
    if (pids.length === 0) return toast("请先勾选窗口", true);
    run("解冻进程", async () => {
      await invoke("resume_pids", { pids, ...freezeArgs });
      toast(`已解冻 ${pids.length} 个进程`);
    });
  }
</script>

<Modal title="窗口恢复工具" bind:open>
  <p class="hint">
   勾选后可显示 / 隐藏窗口，或冻结 / 解冻其所属进程<br>
   冻结跟随「进程冻结」里的增强冻结与「冻结完整进程」设置。
  </p>
  <div class="list-wrap">
    <WindowList
      title="所有窗口"
      windows={shown}
      bind:selected
      bind:search
      bind:showBackground
      bind:showUntitled
      onrefresh={load}
    />
  </div>

  {#snippet footer()}
    <button class="btn" onclick={() => (open = false)}>关闭</button>
    <span class="spacer"></span>
    <button class="btn" disabled={busy} onclick={hideSel}>隐藏窗口</button>
    <button class="btn" disabled={busy} onclick={freezeSel}>冻结进程</button>
    <button class="btn" disabled={busy} onclick={resumeSel}>解冻进程</button>
    <button class="btn primary" disabled={busy} onclick={showSel}>显示窗口</button>
  {/snippet}
</Modal>

<style>
  .hint {
    color: var(--muted);
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
