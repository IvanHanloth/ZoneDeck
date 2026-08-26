<script>
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import IconVolumeOff from "~icons/lucide/volume-off";
  import IconAppWindow from "~icons/lucide/app-window";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconPause from "~icons/lucide/pause";
  import IconMinimize from "~icons/lucide/minimize-2";
  import { app } from "../lib/state.svelte.js";
  import { t } from "../lib/i18n.svelte.js";

  const s = $derived(app.config.setting);
  // 没有托盘图标就无所谓「一起隐藏」。
  const trayOff = $derived(!s.tray_enabled);
</script>

<SettingsGroup title={t("hide.generalCard")}>
  <SettingsCard icon={IconVolumeOff} label={t("hide.muteAfterHide")} description={t("hide.muteAfterHideDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={s.mute_after_hide} />{/snippet}
  </SettingsCard>
  <SettingsCard icon={IconAppWindow} label={t("hide.hideCurrent")} description={t("hide.hideCurrentDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={s.hide_current} />{/snippet}
  </SettingsCard>
  <SettingsCard
    icon={IconEyeOff}
    label={t("hide.hideIcon")}
    description={t("hide.hideIconDesc")}
    disabled={trayOff}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.hide_icon_after_hide} disabled={trayOff} />{/snippet}
  </SettingsCard>
  <SettingsCard icon={IconPause} label={t("hide.sendPause")} description={t("hide.sendPauseDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={s.send_before_hide} />{/snippet}
  </SettingsCard>
  <SettingsCard
    icon={IconMinimize}
    label={t("hide.minimizeBeforeHide")}
    description={t("hide.minimizeBeforeHideDesc")}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.minimize_before_hide} />{/snippet}
  </SettingsCard>
  <!-- <SettingsCard label="显示悬浮窗" description="在桌面显示一个可拖动的悬浮小窗，双击可快速切换隐藏（核心侧功能开发中）。">
    {#snippet control()}<ToggleSwitch bind:checked={s.show_float_window} />{/snippet}
  </SettingsCard> -->
</SettingsGroup>
