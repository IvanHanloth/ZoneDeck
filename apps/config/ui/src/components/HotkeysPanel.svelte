<script>
  import { t } from "../lib/i18n.svelte.js";
  import { onDestroy, onMount } from "svelte";
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import TextBox from "./fluent/TextBox.svelte";
  import CornerPicker from "./CornerPicker.svelte";
  import HotkeyRecorder from "./HotkeyRecorder.svelte";
  import MousePicker from "./MousePicker.svelte";
  import IconRepeat from "~icons/lucide/repeat";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconEye from "~icons/lucide/eye";
  import IconAppWindow from "~icons/lucide/app-window";
  import IconPower from "~icons/lucide/power";
  import IconUndo from "~icons/lucide/undo-2";
  import IconTimer from "~icons/lucide/timer";
  import IconZap from "~icons/lucide/zap";
  import IconCornerUpLeft from "~icons/lucide/corner-up-left";
  import IconHourglass from "~icons/lucide/hourglass";
  import IconClock from "~icons/lucide/clock";
  import { MAX_MULTI_CLICK_MS, MIN_MULTI_CLICK_MS } from "../lib/pointer.js";
  import {
    clampInt,
    DEFAULT_AUTO_HIDE_TIME,
    DEFAULT_MULTI_CLICK_MS,
    MAX_AUTO_HIDE_TIME,
    MIN_AUTO_HIDE_TIME,
  } from "../lib/sanitize.js";
  import { app, resumeMonitoring, suspendMonitoring } from "../lib/state.svelte.js";

  const s = $derived(app.config.setting);

  // 数字输入框清空或越界时失焦归位。
  function fixMultiClickMs() {
    s.mouse.multi_click_ms = clampInt(
      s.mouse.multi_click_ms,
      MIN_MULTI_CLICK_MS,
      MAX_MULTI_CLICK_MS,
      DEFAULT_MULTI_CLICK_MS,
    );
  }
  function fixAutoHideTime() {
    s.auto_hide_time = clampInt(
      s.auto_hide_time,
      MIN_AUTO_HIDE_TIME,
      MAX_AUTO_HIDE_TIME,
      DEFAULT_AUTO_HIDE_TIME,
    );
  }

  // 鼠标进入本页设置区时暂停核心监控，离开时恢复。
  const REASON = { area: "hotkeys-panel" };

  // 窗口失焦时恢复监控，此时不会触发 pointerleave。
  onMount(() => {
    const onBlur = () => resumeMonitoring(REASON);
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  });
  onDestroy(() => resumeMonitoring(REASON));

  // 展开态由 SettingsExpander 的 autoExpand 负责联动，这里只存状态。
  let idleOpen = $state(false);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="stack"
  onpointerenter={() => suspendMonitoring(REASON)}
  onpointerleave={() => resumeMonitoring(REASON)}
>
  <SettingsGroup title={t("hotkeys.keyboardCard")}>
    <HotkeyRecorder
      icon={IconRepeat}
      label={t("hotkeys.hideShow")}
      bind:value={app.config.hotkey.hide_hotkey}
      bind:hook={app.config.hotkey.hide_hook}
      bind:intercept={app.config.hotkey.hide_intercept}
    />
    <HotkeyRecorder
      icon={IconEyeOff}
      label={t("hotkeys.hideOnly")}
      bind:value={app.config.hotkey.hide_only_hotkey}
      bind:hook={app.config.hotkey.hide_only_hook}
      bind:intercept={app.config.hotkey.hide_only_intercept}
    />
    <HotkeyRecorder
      icon={IconEye}
      label={t("hotkeys.showOnly")}
      bind:value={app.config.hotkey.show_only_hotkey}
      bind:hook={app.config.hotkey.show_only_hook}
      bind:intercept={app.config.hotkey.show_only_intercept}
    />
    <HotkeyRecorder
      icon={IconAppWindow}
      label={t("hotkeys.hideForeground")}
      bind:value={app.config.hotkey.hide_foreground_hotkey}
      bind:hook={app.config.hotkey.hide_foreground_hook}
      bind:intercept={app.config.hotkey.hide_foreground_intercept}
    />
    <HotkeyRecorder
      icon={IconPower}
      label={t("hotkeys.closeApp")}
      bind:value={app.config.hotkey.close_hotkey}
      bind:hook={app.config.hotkey.close_hook}
      bind:intercept={app.config.hotkey.close_intercept}
    />
  </SettingsGroup>

  <SettingsGroup title={t("hotkeys.mouseCard")}>
    <div class="surface pad"><MousePicker mouse={s.mouse} /></div>
    <SettingsCard icon={IconUndo} label={t("hotkeys.clickRestore")} description={t("hotkeys.clickRestoreDesc")}>
      {#snippet control()}<ToggleSwitch bind:checked={s.mouse.allow_click_restore} />{/snippet}
    </SettingsCard>
    <SettingsCard
      icon={IconTimer}
      label={t("hotkeys.multiClickWindow")}
      description={t("hotkeys.multiClickWindowDesc")}
    >
      {#snippet control()}
        <TextBox
          type="number"
          width="130px"
          min={MIN_MULTI_CLICK_MS}
          max={MAX_MULTI_CLICK_MS}
          step={50}
          suffix={t("hotkeys.milliseconds")}
          ariaLabel={t("hotkeys.multiClickWindowAria")}
          bind:value={s.mouse.multi_click_ms}
          onblur={fixMultiClickMs}
        />
      {/snippet}
    </SettingsCard>
  </SettingsGroup>

  <SettingsGroup title={t("hotkeys.cornerCard")}>
    <div class="surface pad"><CornerPicker setting={s} /></div>
    <SettingsCard icon={IconZap} label={t("hotkeys.fastOnly")} description={t("hotkeys.fastOnlyDesc")}>
      {#snippet control()}<ToggleSwitch bind:checked={s.corner_fast_only} />{/snippet}
    </SettingsCard>
    <SettingsCard
      icon={IconCornerUpLeft}
      label={t("hotkeys.cornerRestore")}
      description={t("hotkeys.cornerRestoreDesc")}
    >
      {#snippet control()}<ToggleSwitch bind:checked={s.allow_move_restore} />{/snippet}
    </SettingsCard>
  </SettingsGroup>

  <SettingsGroup title={t("hotkeys.idleCard")}>
    <SettingsExpander
      bind:open={idleOpen}
      autoExpand={s.auto_hide_enabled}
      icon={IconHourglass}
      label={t("hotkeys.autoHide")}
      description={t("hotkeys.autoHideDesc")}
    >
      {#snippet control()}<ToggleSwitch bind:checked={s.auto_hide_enabled} />{/snippet}

      <SettingsCard
        variant="sub"
        icon={IconClock}
        label={t("hotkeys.idleTime")}
        description={t("hotkeys.idleTimeDesc")}
        disabled={!s.auto_hide_enabled}
      >
        {#snippet control()}
          <TextBox
            type="number"
            width="120px"
            min={MIN_AUTO_HIDE_TIME}
            max={MAX_AUTO_HIDE_TIME}
            suffix={t("hotkeys.minutes")}
            ariaLabel={t("hotkeys.idleTime")}
            bind:value={s.auto_hide_time}
            onblur={fixAutoHideTime}
            disabled={!s.auto_hide_enabled}
          />
        {/snippet}
      </SettingsCard>
    </SettingsExpander>
  </SettingsGroup>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
</style>
