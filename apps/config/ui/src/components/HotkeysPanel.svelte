<script>
  import Card from "./Card.svelte";
  import CornerPicker from "./CornerPicker.svelte";
  import HotkeyRecorder from "./HotkeyRecorder.svelte";
  import MousePicker from "./MousePicker.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import { app } from "../lib/state.svelte.js";

  const s = $derived(app.config.setting);
</script>

<div class="panel-stack">
  <Card title="键盘热键">
    <HotkeyRecorder label="隐藏 / 显示窗口" bind:value={app.config.hotkey.hide_hotkey} />
    <HotkeyRecorder label="一键关闭程序" bind:value={app.config.hotkey.close_hotkey} />
  </Card>

  <Card title="鼠标按键隐藏">
    <p class="card-hint">点亮鼠标上的按键即可启用；每颗键可单独设连击次数和修饰键。</p>
    <MousePicker mouse={s.mouse} />
  </Card>

  <Card title="移动到屏幕四角隐藏">
    <CornerPicker setting={s} />
    <SettingRow label="角落恢复" description="开启后，允许通过鼠标移动至角落来恢复已隐藏的窗口。">
      {#snippet control()}<Toggle bind:checked={app.config.setting.allow_move_restore} />{/snippet}
    </SettingRow>
  </Card>

  <section class="fcard">
    <h3>空闲自动隐藏</h3>
    <SettingRow label="启用自动隐藏" description="长时间无键鼠操作后，自动隐藏已绑定窗口。">
      {#snippet control()}<Toggle bind:checked={s.auto_hide_enabled} />{/snippet}
    </SettingRow>
    <SettingRow label="空闲时长" description="无操作达到该时长后触发自动隐藏。">
      {#snippet control()}
        <div class="idle-ctl" class:dim={!s.auto_hide_enabled}>
          <input
            type="number"
            min="1"
            max="120"
            bind:value={s.auto_hide_time}
            disabled={!s.auto_hide_enabled}
          />
          <span>分钟</span>
        </div>
      {/snippet}
    </SettingRow>
  </section>
</div>

<style>
  .card-hint {
    font-size: 12.5px;
    color: var(--muted);
  }
  .idle-ctl {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .idle-ctl.dim {
    opacity: 0.55;
  }
  .idle-ctl input {
    width: 72px;
  }
</style>
