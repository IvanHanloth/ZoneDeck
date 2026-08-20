<script>
  import { t } from "../lib/i18n.svelte.js";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import IconBell from "~icons/lucide/bell";
  import IconCircleDot from "~icons/lucide/circle-dot";
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

  // 行首的圆环即色板，未绑定状态时淡化。
  function badgeColor(color) {
    return s.tray_badges[color.key]
      ? color.css
      : `color-mix(in srgb, ${color.css} 30%, var(--surface))`;
  }
</script>

<div class="panel-stack">
  <section class="fcard">
    <h3><IconBell width="16" height="16" /> {t("notify.card")}</h3>
    <SettingRow icon={IconPlay} label={t("notify.onStart")} description={t("notify.onStartDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_start} />{/snippet}
    </SettingRow>
    <SettingRow icon={IconCircleStop} label={t("notify.onQuit")} description={t("notify.onQuitDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_quit} />{/snippet}
    </SettingRow>
    <SettingRow icon={IconPower} label={t("notify.onAutostart")} description={t("notify.onAutostartDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_autostart} />{/snippet}
    </SettingRow>
    <SettingRow icon={IconEyeOff} label={t("notify.onHide")} description={t("notify.onHideDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_hide} />{/snippet}
    </SettingRow>
    <SettingRow icon={IconEye} label={t("notify.onShow")} description={t("notify.onShowDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_show} />{/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3><IconCircleDot width="16" height="16" /> {t("notify.trayCard")}</h3>
    {#each BADGE_COLORS as color (color.key)}
      <SettingRow icon={IconCircle} iconColor={badgeColor(color)} label={t(color.labelKey)}>
        {#snippet control()}
          <select class="sel" bind:value={s.tray_badges[color.key]}>
            {#each BADGE_STATUSES as st (st.value)}
              <option value={st.value}>{t(st.labelKey)}</option>
            {/each}
          </select>
        {/snippet}
      </SettingRow>
    {/each}
    <div class="note">{t("notify.trayPriorityNote")}</div>
    <SettingRow icon={IconTag} label={t("notify.trayTooltip")} description={t("notify.trayTooltipDesc")}>
      {#snippet control()}<Toggle bind:checked={s.tray_show_tooltip} />{/snippet}
    </SettingRow>
  </section>
</div>

<style>
  .fcard h3 {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sel {
    padding: 5px 10px;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 13px;
  }
  .sel:focus {
    outline: none;
    border-color: var(--accent);
  }
  .note {
    padding: 10px 14px;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.6;
    border-top: 1px solid var(--border);
  }
</style>
