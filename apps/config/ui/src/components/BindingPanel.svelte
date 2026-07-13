<script>
  import WindowList from "./WindowList.svelte";
  import { moveWindows } from "../lib/grouping.js";
  import { app, refreshWindows, toast } from "../lib/state.svelte.js";

  let selectedAvail = $state([]);
  let selectedBound = $state([]);

  function pick(list, hwnds) {
    return list.filter((w) => hwnds.includes(w.hwnd));
  }

  function addBinding() {
    const picked = pick(app.available, selectedAvail);
    if (picked.length === 0) return toast("请先在左侧勾选要绑定的窗口", true);
    const { from, to } = moveWindows(app.available, app.bound, picked);
    app.available = from;
    app.bound = to;
    selectedAvail = [];
  }

  function removeBinding() {
    const picked = pick(app.bound, selectedBound);
    if (picked.length === 0) return toast("请先在右侧勾选要解绑的窗口", true);
    const { from, to } = moveWindows(app.bound, app.available, picked);
    app.bound = from;
    app.available = to;
    selectedBound = [];
  }

  async function refresh() {
    await refreshWindows();
    selectedAvail = [];
    toast("窗口列表已刷新");
  }
</script>

<div class="binding">
  <div class="grid">
    <WindowList title="现有窗口" windows={app.available} bind:selected={selectedAvail} />
    <div class="actions">
      <button class="btn primary" onclick={addBinding}>添加 →</button>
      <button class="btn" onclick={removeBinding}>← 移除</button>
      <button class="btn ghost" onclick={refresh}>⟳ 刷新</button>
    </div>
    <WindowList title="已绑定窗口" windows={app.bound} bind:selected={selectedBound} />
  </div>
  <p class="hint">
    勾选左侧窗口后点击「添加」。按下隐藏热键将一键隐藏所有已绑定窗口；修改后记得保存设置。
  </p>
</div>

<style>
  .binding {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 0;
  }

  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    gap: 12px;
    align-items: stretch;
  }

  .actions {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
  }

  /* 窄窗口：上下堆叠，操作按钮横排 */
  @media (max-width: 680px) {
    .grid {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr auto 1fr;
    }
    .actions {
      flex-direction: row;
      justify-content: center;
    }
  }
</style>
