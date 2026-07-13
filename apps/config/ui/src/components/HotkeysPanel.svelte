<script>
  import Card from "./Card.svelte";
  import HotkeyRecorder from "./HotkeyRecorder.svelte";
  import Toggle from "./Toggle.svelte";
  import { app } from "../lib/state.svelte.js";
</script>

<div class="panel-stack">
  <Card title="键盘热键">
    <HotkeyRecorder label="隐藏 / 显示窗口" bind:value={app.config.hotkey.hide_hotkey} />
    <HotkeyRecorder label="一键关闭程序" bind:value={app.config.hotkey.close_hotkey} />
  </Card>

  <Card title="鼠标按键隐藏">
    <div class="opt-grid">
      <Toggle label="鼠标中键切换隐藏" bind:checked={app.config.setting.middle_button_hide} />
      <Toggle label="侧键 1（前进键）切换" bind:checked={app.config.setting.side_button1_hide} />
      <Toggle label="侧键 2（后退键）切换" bind:checked={app.config.setting.side_button2_hide} />
    </div>
  </Card>

  <Card title="移动到屏幕四角隐藏">
    <div class="opt-grid corners">
      <Toggle label="左上角" bind:checked={app.config.setting.top_left_hide} />
      <Toggle label="右上角" bind:checked={app.config.setting.top_right_hide} />
      <Toggle label="左下角" bind:checked={app.config.setting.bottom_left_hide} />
      <Toggle label="右下角" bind:checked={app.config.setting.bottom_right_hide} />
    </div>
    <Toggle
      label="允许移动到同一角落恢复窗口"
      bind:checked={app.config.setting.allow_move_restore}
    />
  </Card>

  <Card title="空闲自动隐藏">
    <Toggle label="启用自动隐藏" bind:checked={app.config.setting.auto_hide_enabled} />
    <div class="idle-row" class:dim={!app.config.setting.auto_hide_enabled}>
      <span>无操作</span>
      <input
        type="number"
        min="1"
        max="120"
        bind:value={app.config.setting.auto_hide_time}
        disabled={!app.config.setting.auto_hide_enabled}
      />
      <span>分钟后自动隐藏</span>
    </div>
  </Card>
</div>

<style>
  .idle-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .idle-row.dim {
    opacity: 0.55;
  }
  .idle-row input {
    width: 72px;
  }
</style>
