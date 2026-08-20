<script>
  import { t } from "../lib/i18n.svelte.js";
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import InfoBar from "./fluent/InfoBar.svelte";
  import IconPlay from "~icons/lucide/play";
  import IconCircleStop from "~icons/lucide/circle-stop";
  import IconPower from "~icons/lucide/power";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconEye from "~icons/lucide/eye";
  import IconCircle from "~icons/lucide/circle";
  import IconTag from "~icons/lucide/tag";
  import { app } from "../lib/state.svelte.js";

  const n = $derived(app.config.notifications);
  const s = $derived(app.config.setting);

  // 四种角标颜色，顺序即优先级；与核心 tray_badge.rs 一致。
  const BADGE_COLORS = [
    { key: "red", labelKey: "notify.badgeRed", css: "#f44336" },
    { key: "green", labelKey: "notify.badgeGreen", css: "#4caf50" },
    { key: "yellow", labelKey: "notify.badgeYellow", css: "#ffc107" },
    { key: "blue", labelKey: "notify.badgeBlue", css: "#2196f3" },
  ];

  // 可绑定的状态源；"" 表示不显示该颜色，取值与 TRAY_STATUSES 一致。
  const BADGE_STATUSES = [
    { labelKey: "notify.statusNone", value: "" },
    { value: "hidden", labelKey: "notify.statusHidden" },
    { value: "auto_hide", labelKey: "notify.statusAutoHide" },
    { value: "hide_current", labelKey: "notify.statusHideCurrent" },
    { value: "freeze", labelKey: "notify.statusFreeze" },
    { value: "elevated", labelKey: "notify.statusElevated" },
    { value: "monitor_paused", labelKey: "notify.statusMonitorPaused" },
  ];

  const badgeOptions = $derived(
    BADGE_STATUSES.map((st) => ({ value: st.value, label: t(st.labelKey) })),
  );

  // 行首的圆环即色板，未绑定状态时淡化。
  function badgeColor(color) {
    return s.tray_badges[color.key]
      ? color.css
      : `color-mix(in srgb, ${color.css} 30%, var(--card))`;
  }
</script>

<SettingsGroup title={t("notify.card")}>
  <SettingsCard icon={IconPlay} label={t("notify.onStart")} description={t("notify.onStartDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={n.on_start} />{/snippet}
  </SettingsCard>
  <SettingsCard icon={IconCircleStop} label={t("notify.onQuit")} description={t("notify.onQuitDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={n.on_quit} />{/snippet}
  </SettingsCard>
  <SettingsCard icon={IconPower} label={t("notify.onAutostart")} description={t("notify.onAutostartDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={n.on_autostart} />{/snippet}
  </SettingsCard>
  <SettingsCard icon={IconEyeOff} label={t("notify.onHide")} description={t("notify.onHideDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={n.on_hide} />{/snippet}
  </SettingsCard>
  <SettingsCard icon={IconEye} label={t("notify.onShow")} description={t("notify.onShowDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={n.on_show} />{/snippet}
  </SettingsCard>
</SettingsGroup>

<SettingsGroup title={t("notify.trayCard")}>
  {#each BADGE_COLORS as color (color.key)}
    <SettingsCard icon={IconCircle} iconColor={badgeColor(color)} label={t(color.labelKey)}>
      {#snippet control()}
        <ComboBox
          bind:value={s.tray_badges[color.key]}
          options={badgeOptions}
          ariaLabel={t(color.labelKey)}
        />
      {/snippet}
    </SettingsCard>
  {/each}

  <InfoBar>{t("notify.trayPriorityNote")}</InfoBar>

  <SettingsCard icon={IconTag} label={t("notify.trayTooltip")} description={t("notify.trayTooltipDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={s.tray_show_tooltip} />{/snippet}
  </SettingsCard>
</SettingsGroup>
