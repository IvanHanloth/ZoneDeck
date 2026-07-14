<script>
  import Modal from "./Modal.svelte";
  import WindowList from "./WindowList.svelte";
  import { invoke } from "../lib/ipc.js";
  import { toast } from "../lib/state.svelte.js";

  let { open = $bindable(false) } = $props();

  let windows = $state([]);
  let selected = $state([]);

  async function load() {
    windows = await invoke("list_windows");
  }

  $effect(() => {
    if (open) {
      selected = [];
      load();
    }
  });

  async function restore() {
    if (selected.length === 0) return toast("请先勾选要恢复的窗口", true);
    await invoke("show_windows", { hwnds: selected });
    toast("已尝试恢复显示选中的窗口");
    await load();
  }
</script>

<Modal title="窗口恢复工具" bind:open>
  <p class="hint">
    列出当前所有窗口（含后台 / 已隐藏）。勾选后点击「恢复显示」，尝试将其重新显示出来——
    用于找回被误隐藏、无法通过热键恢复的窗口。
  </p>
  <div class="list-wrap">
    <WindowList title="所有窗口" {windows} bind:selected />
  </div>

  {#snippet footer()}
    <button class="btn" onclick={() => (open = false)}>关闭</button>
    <button class="btn primary" onclick={restore}>恢复显示</button>
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
</style>
