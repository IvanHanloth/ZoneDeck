<script>
  import { t } from "../lib/i18n.svelte.js";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import IconBell from "~icons/lucide/bell";
  import IconCircleDot from "~icons/lucide/circle-dot";
  import { app } from "../lib/state.svelte.js";

  const n = $derived(app.config.notifications);
  const s = $derived(app.config.setting);

  // 四种角标颜色，顺序即优先级（红最高），与核心 tray_badge.rs 保持一致。
  const BADGE_COLORS = [
    { key: "red", labelKey: "notify.badgeRed", css: "#f44336" },
    { key: "green", labelKey: "notify.badgeGreen", css: "#4caf50" },
    { key: "yellow", labelKey: "notify.badgeYellow", css: "#ffc107" },
    { key: "blue", labelKey: "notify.badgeBlue", css: "#2196f3" },
  ];

  // 可绑定的状态源；"" 表示不显示该颜色。取值与 crates/common 的 TRAY_STATUSES 一致。
  const BADGE_STATUSES = [
    { labelKey: "notify.statusNone", value: "" },
    { value: "hidden", labelKey: "notify.statusHidden" },
    { value: "auto_hide", labelKey: "notify.statusAutoHide" },
    { value: "hide_current", labelKey: "notify.statusHideCurrent" },
    { value: "freeze", labelKey: "notify.statusFreeze" },
    { value: "elevated", labelKey: "notify.statusElevated" },
    { value: "monitor_paused", labelKey: "notify.statusMonitorPaused" },
  ];
</script>

<div class="panel-stack">
  <section class="fcard">
    <h3><IconBell width="16" height="16" /> {t("notify.card")}</h3>
    <SettingRow label={t("notify.onStart")} description={t("notify.onStartDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_start} />{/snippet}
    </SettingRow>
    <SettingRow label={t("notify.onQuit")} description={t("notify.onQuitDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_quit} />{/snippet}
    </SettingRow>
    <SettingRow label={t("notify.onAutostart")} description={t("notify.onAutostartDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_autostart} />{/snippet}
    </SettingRow>
    <SettingRow label={t("notify.onHide")} description={t("notify.onHideDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_hide} />{/snippet}
    </SettingRow>
    <SettingRow label={t("notify.onShow")} description={t("notify.onShowDesc")}>
      {#snippet control()}<Toggle bind:checked={n.on_show} />{/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3><IconCircleDot width="16" height="16" /> {t("notify.trayCard")}</h3>
    {#each BADGE_COLORS as color (color.key)}
      <SettingRow label={t(color.labelKey)}>
        {#snippet control()}
          <div class="badge-ctl">
            <span class="dot" style:background={color.css} class:off={!s.tray_badges[color.key]}></span>
            <select class="sel" bind:value={s.tray_badges[color.key]}>
              {#each BADGE_STATUSES as st (st.value)}
                <option value={st.value}>{t(st.labelKey)}</option>
              {/each}
            </select>
          </div>
        {/snippet}
      </SettingRow>
    {/each}
    <div class="note">{t("notify.trayPriorityNote")}</div>
    <SettingRow label={t("notify.trayTooltip")} description={t("notify.trayTooltipDesc")}>
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
  .badge-ctl {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--surface-2);
    box-shadow: 0 0 0 1px var(--border);
    flex-shrink: 0;
  }
  .dot.off {
    opacity: 0.25;
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
