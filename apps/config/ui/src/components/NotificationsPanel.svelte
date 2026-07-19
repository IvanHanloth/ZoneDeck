<script>
  import { t } from "../lib/i18n.svelte.js";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import IconBell from "~icons/lucide/bell";
  import { app } from "../lib/state.svelte.js";

  const n = $derived(app.config.notifications);
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
</div>

<style>
  .fcard h3 {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
