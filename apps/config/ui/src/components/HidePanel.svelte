<script>
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import IconVolumeOff from "~icons/lucide/volume-off";
  import IconAppWindow from "~icons/lucide/app-window";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconPause from "~icons/lucide/pause";
  import IconPlay from "~icons/lucide/play";
  import IconMinimize from "~icons/lucide/minimize-2";
  import IconSlidersHorizontal from "~icons/lucide/sliders-horizontal";
  import IconCircleHelp from "~icons/lucide/circle-help";
  import IconPanelBottom from "~icons/lucide/panel-bottom";
  import { app, toast } from "../lib/state.svelte.js";
  import { openExternal } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  // 系统设置的「任务栏」页，其他程序的托盘图标在那里逐个开关。
  const TASKBAR_SETTINGS_URI = "ms-settings:taskbar";

  const s = $derived(app.config.setting);
  // 没有托盘图标就无所谓「一起隐藏」。
  const trayOff = $derived(!s.tray_enabled);
  // 没暂停过就没有可续播的。
  const pauseOff = $derived(!s.send_before_hide);
  // 主开关一开就把子项摊开。
  // 展开态由 SettingsExpander 的 autoExpand 负责联动，这里只存状态。
  let pauseExpanded = $state(false);

  async function openLink(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("options.openLinkFailed", { err }), true);
    }
  }
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
  <SettingsCard
    icon={IconSlidersHorizontal}
    label={t("hide.hideConfig")}
    description={t("hide.hideConfigDesc")}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.hide_config_after_hide} />{/snippet}
  </SettingsCard>
  <SettingsExpander
    bind:open={pauseExpanded}
    autoExpand={s.send_before_hide}
    icon={IconPause}
    label={t("hide.sendPause")}
    description={t("hide.sendPauseDesc")}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.send_before_hide} />{/snippet}

    <SettingsCard
      variant="sub"
      icon={IconPlay}
      label={t("hide.resumeMedia")}
      description={t("hide.resumeMediaDesc")}
      disabled={pauseOff}
    >
      {#snippet control()}
        <ToggleSwitch
          bind:checked={s.resume_media_after_show}
          disabled={pauseOff}
          title={pauseOff ? t("hide.needPauseFirst") : ""}
        />
      {/snippet}
    </SettingsCard>
  </SettingsExpander>
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

<SettingsGroup title={t("hide.supportCard")}>
  <div class="links">
    <button class="link" type="button" onclick={() => openLink(t("hide.helpOtherIconsUrl"))}>
      <IconCircleHelp width="16" height="16" />
      <span class="label">{t("hide.helpOtherIcons")}</span>
    </button>
    <button class="link" type="button" onclick={() => openLink(TASKBAR_SETTINGS_URI)}>
      <IconPanelBottom width="16" height="16" />
      <span class="label">{t("hide.taskbarSettings")}</span>
    </button>
  </div>
</SettingsGroup>

<style>
  /* 左缘对齐分组标题 */
  .links {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 2px 1px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0;
    color: var(--accent);
  }
  .link:hover .label {
    text-decoration: underline;
  }
  .link:active {
    color: var(--accent-pressed);
  }
</style>
