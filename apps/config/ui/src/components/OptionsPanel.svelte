<script>
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import TrayPanel from "./TrayPanel.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import IconPower from "~icons/lucide/power";
  import IconShield from "~icons/lucide/shield";
  import IconKeyRound from "~icons/lucide/key-round";
  import IconLanguages from "~icons/lucide/languages";
  import IconCalendarClock from "~icons/lucide/calendar-clock";
  import IconGauge from "~icons/lucide/gauge";
  import IconLifeBuoy from "~icons/lucide/life-buoy";
  import { app, openRestoreTool, restartCore, startCore, setAutostart } from "../lib/state.svelte.js";
  import { LANGS, LANG_AUTO, LANG_NAMES, t } from "../lib/i18n.svelte.js";

  let elevating = $state(false);

  async function elevate() {
    elevating = true;
    try {
      if (app.status.running) await restartCore(true);
      else await startCore(true);
    } finally {
      elevating = false;
    }
  }

  const s = $derived(app.config.setting);

  // 日志输出等级，由低到高；value 与核心 Setting::log_level 的取值一致。
  // 文案键须为字面量，供 scripts/i18n-check.ps1 静态检查。
  const LOG_LEVELS = [
    { value: "debug", label: "options.logLevel.debug" },
    { value: "info", label: "options.logLevel.info" },
    { value: "warn", label: "options.logLevel.warn" },
    { value: "error", label: "options.logLevel.error" },
  ];
  const logDisabled = $derived(s.log_retention_days === 0);

  const logLevelOptions = $derived(
    LOG_LEVELS.map((l) => ({ value: l.value, label: t(l.label) })),
  );
  const logRetentionOptions = $derived([
    { value: 0, label: t("options.logOff") },
    ...[3, 7, 14, 30].map((n) => ({ value: n, label: t("options.logDays", { n }) })),
  ]);
  const langOptions = $derived([
    { value: LANG_AUTO, label: t("options.languageAuto") },
    ...LANGS.map((tag) => ({ value: tag, label: LANG_NAMES[tag] })),
  ]);

  // 自启注册方式的标注；仅在已开启自启时显示。
  const autostartMethodText = $derived(
    !app.autostart
      ? ""
      : app.autostartMethod === "task"
        ? t("options.methodTask")
        : app.autostartMethod === "registry"
          ? t("options.methodRegistry")
          : "",
  );

  // 权限开关变更后，若自启已开则按新权限重注册。
  async function onAutostartAdminChange(admin) {
    if (app.autostart) await setAutostart(true, admin);
  }

  // 「以管理员身份自启」需先开启开机自启。
  const autostartOff = $derived(!app.autostart);
</script>

<SettingsGroup title={t("options.startupCard")}>
  <SettingsExpander
    icon={IconPower}
    label={t("options.autostart")}
    description={t("options.autostartDesc")}
    autoExpand={app.autostart}
  >
    {#snippet control()}
      {#if autostartMethodText}
        <span class="method">{t("options.autostartMethod", { method: autostartMethodText })}</span>
      {/if}
      <ToggleSwitch
        bind:checked={app.autostart}
        onchange={(e) => setAutostart(e.target.checked, s.autostart_admin)}
      />
    {/snippet}

    <SettingsCard
      variant="sub"
      icon={IconShield}
      label={t("options.autostartAdmin")}
      disabled={autostartOff}
      description={t("options.autostartAdminDesc")}
    >
      {#snippet control()}
        <ToggleSwitch
          bind:checked={s.autostart_admin}
          disabled={autostartOff}
          title={autostartOff ? t("options.needAutostartFirst") : ""}
          onchange={(e) => onAutostartAdminChange(e.target.checked)}
        />
      {/snippet}
    </SettingsCard>
  </SettingsExpander>

  <SettingsCard icon={IconKeyRound} label={t("options.corePrivilege")} description={t("options.corePrivilegeDesc")}>
    {#snippet control()}
      <strong>
        {app.status.running === null
          ? t("status.detecting")
          : !app.status.running
            ? t("status.coreStopped")
            : t(app.status.elevated ? "options.privilegeAdmin" : "options.privilegeUser")}
      </strong>
      <button
        class="btn"
        onclick={elevate}
        disabled={elevating || (app.status.running === true && app.status.elevated)}
      >
        {t(app.status.running ? "options.restartAsAdmin" : "options.startAsAdmin")}
      </button>
    {/snippet}
  </SettingsCard>
</SettingsGroup>

<SettingsGroup title={t("options.languageCard")}>
  <SettingsCard icon={IconLanguages} label={t("options.language")} description={t("options.languageDesc")}>
    {#snippet control()}
      <ComboBox bind:value={s.language} options={langOptions} ariaLabel={t("options.language")} />
    {/snippet}
  </SettingsCard>
</SettingsGroup>

<TrayPanel />

<SettingsGroup title={t("options.logCard")}>
  <SettingsCard icon={IconCalendarClock} label={t("options.logRetention")} description={t("options.logRetentionDesc")}>
    {#snippet control()}
      <ComboBox
        bind:value={s.log_retention_days}
        options={logRetentionOptions}
        ariaLabel={t("options.logRetention")}
      />
    {/snippet}
  </SettingsCard>
  <SettingsCard
    icon={IconGauge}
    label={t("options.logLevel")}
    description={t("options.logLevelDesc")}
    disabled={logDisabled}
  >
    {#snippet control()}
      <ComboBox
        bind:value={s.log_level}
        options={logLevelOptions}
        disabled={logDisabled}
        ariaLabel={t("options.logLevel")}
      />
    {/snippet}
  </SettingsCard>
</SettingsGroup>

<SettingsGroup title={t("options.toolsCard")}>
  <SettingsCard icon={IconLifeBuoy} label={t("options.restoreTool")} description={t("options.restoreToolDesc")}>
    {#snippet control()}
      <button class="btn" onclick={openRestoreTool}>{t("common.open")}</button>
    {/snippet}
  </SettingsCard>
</SettingsGroup>

<style>
  .method {
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }
</style>
