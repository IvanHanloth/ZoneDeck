<script>
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import InfoBar from "./fluent/InfoBar.svelte";
  import IconPanelBottom from "~icons/lucide/panel-bottom";
  import IconMousePointerClick from "~icons/lucide/mouse-pointer-click";
  import IconMousePointer2 from "~icons/lucide/mouse-pointer-2";
  import IconMenu from "~icons/lucide/menu";
  import IconTag from "~icons/lucide/tag";
  import IconCircle from "~icons/lucide/circle";
  import IconBell from "~icons/lucide/bell";
  import IconPlay from "~icons/lucide/play";
  import IconCircleStop from "~icons/lucide/circle-stop";
  import IconPower from "~icons/lucide/power";
  import IconEyeOff from "~icons/lucide/eye-off";
  import IconEye from "~icons/lucide/eye";
  import { app } from "../lib/state.svelte.js";
  import { t } from "../lib/i18n.svelte.js";

  const s = $derived(app.config.setting);
  const n = $derived(app.config.notifications);

  // 可绑定的点击动作；取值与核心 TRAY_ACTIONS 一致。
  const ACTIONS = [
    { value: "none", labelKey: "tray.actionNone" },
    { value: "toggle", labelKey: "tray.actionToggle" },
    { value: "menu", labelKey: "tray.actionMenu" },
    { value: "settings", labelKey: "tray.actionSettings" },
  ];

  // 三种点击各一行，顺序即界面顺序。
  const CLICKS = [
    { key: "left", labelKey: "tray.left", descKey: "tray.leftDesc", icon: IconMousePointerClick },
    { key: "double", labelKey: "tray.double", descKey: "tray.doubleDesc", icon: IconMousePointer2 },
    { key: "right", labelKey: "tray.right", descKey: "tray.rightDesc", icon: IconMenu },
  ];

  // 四种角标颜色，顺序即优先级；与核心 tray_badge.rs 一致。
  const BADGE_COLORS = [
    { key: "red", labelKey: "notify.badgeRed", css: "#f44336" },
    { key: "green", labelKey: "notify.badgeGreen", css: "#4caf50" },
    { key: "yellow", labelKey: "notify.badgeYellow", css: "#ffc107" },
    { key: "blue", labelKey: "notify.badgeBlue", css: "#2196f3" },
  ];

  // 可绑定的状态源；"" 表示不显示该颜色，取值与 TRAY_STATUSES 一致。
  // 空串一项须把 labelKey 写在前面：i18n-check 扫字符串字面量时，
  // 前置的 "" 会连带吃掉后一个引号，把它后面的文案键遮成「无人引用」。
  const BADGE_STATUSES = [
    { labelKey: "notify.statusNone", value: "" },
    { value: "hidden", labelKey: "notify.statusHidden" },
    { value: "auto_hide", labelKey: "notify.statusAutoHide" },
    { value: "hide_current", labelKey: "notify.statusHideCurrent" },
    { value: "freeze", labelKey: "notify.statusFreeze" },
    { value: "elevated", labelKey: "notify.statusElevated" },
    { value: "monitor_paused", labelKey: "notify.statusMonitorPaused" },
  ];

  // 五个可逐项开关的通知事件。
  const EVENTS = [
    { key: "on_start", labelKey: "notify.onStart", descKey: "notify.onStartDesc", icon: IconPlay },
    { key: "on_quit", labelKey: "notify.onQuit", descKey: "notify.onQuitDesc", icon: IconCircleStop },
    { key: "on_autostart", labelKey: "notify.onAutostart", descKey: "notify.onAutostartDesc", icon: IconPower },
    { key: "on_hide", labelKey: "notify.onHide", descKey: "notify.onHideDesc", icon: IconEyeOff },
    { key: "on_show", labelKey: "notify.onShow", descKey: "notify.onShowDesc", icon: IconEye },
  ];

  const actionOptions = $derived(ACTIONS.map((a) => ({ value: a.value, label: t(a.labelKey) })));
  const badgeOptions = $derived(
    BADGE_STATUSES.map((st) => ({ value: st.value, label: t(st.labelKey) })),
  );

  // 行首的圆环即色板，未绑定状态时淡化。
  function badgeColor(color) {
    return s.tray_badges[color.key]
      ? color.css
      : `color-mix(in srgb, ${color.css} 30%, var(--card))`;
  }

  // 角标画在图标上，图标撤掉它就无处可依；通知走 Toast，与图标无关。
  const trayOff = $derived(!s.tray_enabled);

  // 点击行为是本组的主体，进来就摊开；关掉图标再打开时由 autoExpand 重新展开。
  let clicksOpen = $state(true);
</script>

<SettingsGroup title={t("tray.card")}>
  <SettingsExpander
    icon={IconPanelBottom}
    label={t("tray.enabled")}
    description={t("tray.enabledDesc")}
    bind:open={clicksOpen}
    autoExpand={s.tray_enabled}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.tray_enabled} />{/snippet}

    {#each CLICKS as click (click.key)}
      <SettingsCard
        variant="sub"
        icon={click.icon}
        label={t(click.labelKey)}
        description={t(click.descKey)}
        disabled={trayOff}
      >
        {#snippet control()}
          <ComboBox
            bind:value={s.tray_clicks[click.key]}
            options={actionOptions}
            disabled={trayOff}
            ariaLabel={t(click.labelKey)}
          />
        {/snippet}
      </SettingsCard>
    {/each}

    <SettingsCard
      variant="sub"
      icon={IconTag}
      label={t("tray.tooltip")}
      description={t("tray.tooltipDesc")}
      disabled={trayOff}
    >
      {#snippet control()}
        <ToggleSwitch bind:checked={s.tray_show_tooltip} disabled={trayOff} />
      {/snippet}
    </SettingsCard>
  </SettingsExpander>

  {#if trayOff}
    <InfoBar severity="warning">{t("tray.disabledNote")}</InfoBar>
  {/if}

  <SettingsExpander
    icon={IconCircle}
    label={t("tray.badges")}
    description={t("tray.badgesDesc")}
    disabled={trayOff}
  >
    {#each BADGE_COLORS as color (color.key)}
      <SettingsCard
        variant="sub"
        icon={IconCircle}
        iconColor={badgeColor(color)}
        label={t(color.labelKey)}
        disabled={trayOff}
      >
        {#snippet control()}
          <ComboBox
            bind:value={s.tray_badges[color.key]}
            options={badgeOptions}
            disabled={trayOff}
            ariaLabel={t(color.labelKey)}
          />
        {/snippet}
      </SettingsCard>
    {/each}

    <InfoBar disabled={trayOff}>{t("notify.trayPriorityNote")}</InfoBar>
  </SettingsExpander>

  <SettingsExpander icon={IconBell} label={t("tray.notify")} description={t("tray.notifyDesc")}>
    {#each EVENTS as ev (ev.key)}
      <SettingsCard
        variant="sub"
        icon={ev.icon}
        label={t(ev.labelKey)}
        description={t(ev.descKey)}
      >
        {#snippet control()}<ToggleSwitch bind:checked={n[ev.key]} />{/snippet}
      </SettingsCard>
    {/each}
  </SettingsExpander>
</SettingsGroup>
