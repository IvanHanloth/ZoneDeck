<script>
  import Card from "./Card.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, refreshStatus, toast } from "../lib/state.svelte.js";
  import { invoke } from "../lib/ipc.js";

  let elevating = $state(false);

  async function elevate() {
    elevating = true;
    try {
      const accepted = await invoke("restart_core_elevated");
      if (accepted) {
        toast("核心正在以管理员身份重启…");
        setTimeout(refreshStatus, 1500);
      } else {
        toast("已取消提权", true);
      }
    } catch (err) {
      toast("提权重启失败：" + err, true);
    } finally {
      elevating = false;
    }
  }

  const elevationText = $derived(
    app.status.running === null
      ? "检测中…"
      : !app.status.running
        ? "核心未运行"
        : app.status.elevated
          ? "管理员"
          : "普通用户",
  );
</script>

<div class="panel-stack">
  <Card title="常规">
    <div class="opt-grid">
      <Toggle label="隐藏窗口后静音" bind:checked={app.config.setting.mute_after_hide} />
      <Toggle label="同时隐藏当前活动窗口" bind:checked={app.config.setting.hide_current} />
      <Toggle label="单击托盘图标切换隐藏" bind:checked={app.config.setting.click_to_hide} />
      <Toggle
        label="隐藏后同时隐藏托盘图标"
        bind:checked={app.config.setting.hide_icon_after_hide}
      />
      <Toggle
        label="文件路径匹配"
        title="启用后隐藏由同一程序启动的所有窗口"
        bind:checked={app.config.setting.path_match}
      />
      <Toggle
        label="隐藏前发送暂停键（Beta）"
        title="隐藏前发送媒体暂停键，会带来约 0.2 秒延迟"
        bind:checked={app.config.setting.send_before_hide}
      />
      <Toggle label="显示悬浮窗" bind:checked={app.config.setting.show_float_window} />
    </div>
  </Card>

  <Card title="进程冻结">
    <Toggle
      label="隐藏窗口时冻结进程（Beta）"
      title="隐藏窗口时冻结进程以降低 CPU / 内存占用"
      bind:checked={app.config.setting.freeze_after_hide}
    />
    <Toggle
      label="使用增强冻结（pssuspend64 + 管理员）"
      title="需要程序目录下放置 pssuspend64.exe 且核心以管理员运行"
      bind:checked={app.config.setting.enhanced_freeze}
    />
    <p class="hint">
      增强冻结需下载
      <a href="https://download.sysinternals.com/files/PSTools.zip" target="_blank" rel="noreferrer">PSTools</a>
      并将 pssuspend64.exe 放入程序目录。
    </p>
  </Card>

  <Card title="管理员权限">
    <p class="hint">增强冻结与「计划任务开机自启（最高权限）」需要核心以管理员身份运行。</p>
    <div class="elev-row">
      <span>核心权限：</span>
      <strong>{elevationText}</strong>
      <button
        class="btn"
        onclick={elevate}
        disabled={elevating || (app.status.running === true && app.status.elevated)}
      >
        以管理员身份重启核心
      </button>
    </div>
  </Card>
</div>

<style>
  .elev-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
</style>
