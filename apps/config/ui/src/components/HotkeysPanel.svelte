<script>
  import { onDestroy, onMount } from "svelte";
  import Card from "./Card.svelte";
  import CornerPicker from "./CornerPicker.svelte";
  import HotkeyRecorder from "./HotkeyRecorder.svelte";
  import MousePicker from "./MousePicker.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import { MAX_MULTI_CLICK_MS, MIN_MULTI_CLICK_MS } from "../lib/pointer.js";
  import { app, resumeMonitoring, suspendMonitoring } from "../lib/state.svelte.js";

  const s = $derived(app.config.setting);

  // 鼠标进入本页设置区时暂停核心监控，离开时恢复。
  const REASON = { area: "hotkeys-panel" };

  // 窗口失焦时恢复监控（此时不会触发 pointerleave）。
  onMount(() => {
    const onBlur = () => resumeMonitoring(REASON);
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  });
  onDestroy(() => resumeMonitoring(REASON));
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="panel-stack"
  onpointerenter={() => suspendMonitoring(REASON)}
  onpointerleave={() => resumeMonitoring(REASON)}
>
  <Card title="键盘热键">
    <HotkeyRecorder label="隐藏 / 显示窗口" bind:value={app.config.hotkey.hide_hotkey} />
    <HotkeyRecorder label="一键关闭程序" bind:value={app.config.hotkey.close_hotkey} />
  </Card>

  <Card title="鼠标按键隐藏">
    <MousePicker mouse={s.mouse} />
    <SettingRow label="按键恢复" description="开启后，再按一次同样的键即可恢复已隐藏的窗口。">
      {#snippet control()}<Toggle bind:checked={s.mouse.allow_click_restore} />{/snippet}
    </SettingRow>
    <SettingRow
      label="连击判定窗口"
      description="两次点击间隔不超过该时长才算连击；五颗按键共用这一个值。"
    >
      {#snippet control()}
        <div class="num-ctl">
          <input
            type="number"
            min={MIN_MULTI_CLICK_MS}
            max={MAX_MULTI_CLICK_MS}
            step="50"
            aria-label="连击判定窗口（毫秒）"
            bind:value={s.mouse.multi_click_ms}
          />
          <span>毫秒</span>
        </div>
      {/snippet}
    </SettingRow>
  </Card>

  <Card title="移动到屏幕四角隐藏">
    <CornerPicker setting={s} />
    <SettingRow
      label="仅快速移动触发"
      description="开启后仅快速移动鼠标至角落时触发隐藏/恢复。"
    >
      {#snippet control()}<Toggle bind:checked={s.corner_fast_only} />{/snippet}
    </SettingRow>
    <SettingRow label="角落恢复" description="开启后，允许通过鼠标移动至角落来恢复已隐藏的窗口。">
      {#snippet control()}<Toggle bind:checked={s.allow_move_restore} />{/snippet}
    </SettingRow>
  </Card>

  <section class="fcard">
    <h3>空闲自动隐藏</h3>
    <SettingRow label="启用自动隐藏" description="长时间无键鼠操作后，自动隐藏已绑定窗口。">
      {#snippet control()}<Toggle bind:checked={s.auto_hide_enabled} />{/snippet}
    </SettingRow>
    <SettingRow label="空闲时长" description="无操作达到该时长后触发自动隐藏。">
      {#snippet control()}
        <div class="num-ctl" class:dim={!s.auto_hide_enabled}>
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
  .num-ctl {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .num-ctl.dim {
    opacity: 0.55;
  }
  .num-ctl input {
    width: 76px;
  }
</style>
