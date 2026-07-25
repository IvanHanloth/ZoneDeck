<script>
  import { t } from "../lib/i18n.svelte.js";
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
  <Card title={t("hotkeys.keyboardCard")}>
    <HotkeyRecorder
      label={t("hotkeys.hideShow")}
      bind:value={app.config.hotkey.hide_hotkey}
      bind:intercept={app.config.hotkey.hide_intercept}
      interceptLabel={t("hotkeys.interceptShort")}
      interceptTitle={t("hotkeys.interceptDesc")}
    />
    <HotkeyRecorder
      label={t("hotkeys.hideOnly")}
      bind:value={app.config.hotkey.hide_only_hotkey}
      bind:intercept={app.config.hotkey.hide_only_intercept}
      interceptLabel={t("hotkeys.interceptShort")}
      interceptTitle={t("hotkeys.interceptDesc")}
    />
    <HotkeyRecorder
      label={t("hotkeys.showOnly")}
      bind:value={app.config.hotkey.show_only_hotkey}
      bind:intercept={app.config.hotkey.show_only_intercept}
      interceptLabel={t("hotkeys.interceptShort")}
      interceptTitle={t("hotkeys.interceptDesc")}
    />
    <HotkeyRecorder
      label={t("hotkeys.hideForeground")}
      bind:value={app.config.hotkey.hide_foreground_hotkey}
      bind:intercept={app.config.hotkey.hide_foreground_intercept}
      interceptLabel={t("hotkeys.interceptShort")}
      interceptTitle={t("hotkeys.interceptDesc")}
    />
    <HotkeyRecorder
      label={t("hotkeys.closeApp")}
      bind:value={app.config.hotkey.close_hotkey}
      bind:intercept={app.config.hotkey.close_intercept}
      interceptLabel={t("hotkeys.interceptShort")}
      interceptTitle={t("hotkeys.interceptDesc")}
    />
    <p class="card-hint">{t("hotkeys.interceptDesc")}</p>
  </Card>

  <Card title={t("hotkeys.mouseCard")}>
    <MousePicker mouse={s.mouse} />
    <SettingRow label={t("hotkeys.clickRestore")} description={t("hotkeys.clickRestoreDesc")}>
      {#snippet control()}<Toggle bind:checked={s.mouse.allow_click_restore} />{/snippet}
    </SettingRow>
    <SettingRow
      label={t("hotkeys.multiClickWindow")}
      description={t("hotkeys.multiClickWindowDesc")}
    >
      {#snippet control()}
        <div class="num-ctl">
          <input
            type="number"
            min={MIN_MULTI_CLICK_MS}
            max={MAX_MULTI_CLICK_MS}
            step="50"
            aria-label={t("hotkeys.multiClickWindowAria")}
            bind:value={s.mouse.multi_click_ms}
          />
          <span>{t("hotkeys.milliseconds")}</span>
        </div>
      {/snippet}
    </SettingRow>
  </Card>

  <Card title={t("hotkeys.cornerCard")}>
    <CornerPicker setting={s} />
    <SettingRow
      label={t("hotkeys.fastOnly")}
      description={t("hotkeys.fastOnlyDesc")}
    >
      {#snippet control()}<Toggle bind:checked={s.corner_fast_only} />{/snippet}
    </SettingRow>
    <SettingRow label={t("hotkeys.cornerRestore")} description={t("hotkeys.cornerRestoreDesc")}>
      {#snippet control()}<Toggle bind:checked={s.allow_move_restore} />{/snippet}
    </SettingRow>
  </Card>

  <section class="fcard">
    <h3>{t("hotkeys.idleCard")}</h3>
    <SettingRow label={t("hotkeys.autoHide")} description={t("hotkeys.autoHideDesc")}>
      {#snippet control()}<Toggle bind:checked={s.auto_hide_enabled} />{/snippet}
    </SettingRow>
    <SettingRow
      label={t("hotkeys.idleTime")}
      description={t("hotkeys.idleTimeDesc")}
      disabled={!s.auto_hide_enabled}
    >
      {#snippet control()}
        <div class="num-ctl">
          <input
            type="number"
            min="1"
            max="120"
            bind:value={s.auto_hide_time}
            disabled={!s.auto_hide_enabled}
          />
          <span>{t("hotkeys.minutes")}</span>
        </div>
      {/snippet}
    </SettingRow>
  </section>
</div>

<style>
  .card-hint {
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .num-ctl {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .num-ctl input {
    width: 76px;
  }
</style>
