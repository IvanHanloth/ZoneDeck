<script>
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, openRestoreTool, restartCore, startCore, setAutostart, refreshPssuspend, toast } from "../lib/state.svelte.js";
  import { openExternal } from "../lib/verhub.js";
  import { LANGS, LANG_AUTO, LANG_NAMES, t } from "../lib/i18n.svelte.js";

  let elevating = $state(false);

  async function openLink(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("options.openLinkFailed", { err }), true);
    }
  }

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

  // 权限开关变更：绑定已更新 s.autostart_admin（自动保存）；若自启已开，按新权限重注册。
  async function onAutostartAdminChange(admin) {
    if (app.autostart) await setAutostart(true, admin);
  }

  // 「以管理员身份自启」的前置条件：必须先开启开机自启。
  const autostartOff = $derived(!app.autostart);

  // 未开启「隐藏窗口时冻结进程」时，冻结区下方的子选项一律置灰禁用。
  const freezeOff = $derived(!s.freeze_after_hide);

  // 增强冻结的前置条件；任一不满足即置灰。
  const enhancedBlocked = $derived.by(() => {
    const reasons = [];
    if (!app.status.running) reasons.push(t("options.blockedCoreStopped"));
    else if (!app.status.elevated) reasons.push(t("options.blockedNeedAdmin"));
    if (!app.pssuspend) reasons.push(t("options.blockedNoPssuspend"));
    return reasons;
  });
  const enhancedDisabled = $derived(enhancedBlocked.length > 0);
</script>

<div class="panel-stack">
  <section class="fcard">
    <h3>{t("options.generalCard")}</h3>
    <SettingRow label={t("options.muteAfterHide")} description={t("options.muteAfterHideDesc")}>
      {#snippet control()}<Toggle bind:checked={s.mute_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow label={t("options.hideCurrent")} description={t("options.hideCurrentDesc")}>
      {#snippet control()}<Toggle bind:checked={s.hide_current} />{/snippet}
    </SettingRow>
    <SettingRow label={t("options.clickToHide")} description={t("options.clickToHideDesc")}>
      {#snippet control()}<Toggle bind:checked={s.click_to_hide} />{/snippet}
    </SettingRow>
    <SettingRow label={t("options.hideIcon")} description={t("options.hideIconDesc")}>
      {#snippet control()}<Toggle bind:checked={s.hide_icon_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow label={t("options.sendPause")} description={t("options.sendPauseDesc")}>
      {#snippet control()}<Toggle bind:checked={s.send_before_hide} />{/snippet}
    </SettingRow>
    <!-- <SettingRow label="显示悬浮窗" description="在桌面显示一个可拖动的悬浮小窗，双击可快速切换隐藏（核心侧功能开发中）。">
      {#snippet control()}<Toggle bind:checked={s.show_float_window} />{/snippet}
    </SettingRow> -->
  </section>

  <section class="fcard">
    <h3>{t("options.freezeCard")}</h3>
    <SettingRow label={t("options.freezeAfterHide")} description={t("options.freezeAfterHideDesc")}>
      {#snippet control()}<Toggle bind:checked={s.freeze_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow
      label={t("options.enhancedFreeze")}
      disabled={freezeOff || enhancedDisabled}
      description={!freezeOff && enhancedDisabled
        ? t("options.enhancedFreezeBlocked", { reasons: enhancedBlocked.join("；") })
        : t("options.enhancedFreezeDesc")}
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.enhanced_freeze}
          disabled={freezeOff || enhancedDisabled}
          title={freezeOff
            ? t("options.needFreezeFirst")
            : enhancedDisabled
              ? enhancedBlocked.join("；")
              : ""}
        />
      {/snippet}
    </SettingRow>
    <SettingRow
      label={t("options.freezeWholeTree")}
      disabled={freezeOff}
      description={t("options.freezeWholeTreeDesc")}
    >
      {#snippet control()}<Toggle bind:checked={s.freeze_whole_tree} disabled={freezeOff} />{/snippet}
    </SettingRow>
    <div class="note" class:disabled={freezeOff}>
      {t("options.freezeNoteBefore")}
      <button class="link" onclick={() => openLink("https://download.sysinternals.com/files/PSTools.zip")} disabled={freezeOff}>PSTools</button>
      {t("options.freezeNoteAfter")}
      <button class="link" onclick={refreshPssuspend} disabled={freezeOff}>{t("options.recheck")}</button>
    </div>
  </section>

  <section class="fcard">
    <h3>{t("options.startupCard")}</h3>
    <SettingRow label={t("options.autostart")} description={t("options.autostartDesc")}>
      {#snippet control()}
        <div class="autostart-ctl">
          {#if autostartMethodText}
            <span class="method">{t("options.autostartMethod", { method: autostartMethodText })}</span>
          {/if}
          <Toggle bind:checked={app.autostart} onchange={(e) => setAutostart(e.target.checked, s.autostart_admin)} />
        </div>
      {/snippet}
    </SettingRow>
    <SettingRow
      label={t("options.autostartAdmin")}
      disabled={autostartOff}
      description={t("options.autostartAdminDesc")}
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.autostart_admin}
          disabled={autostartOff}
          title={autostartOff ? t("options.needAutostartFirst") : ""}
          onchange={(e) => onAutostartAdminChange(e.target.checked)}
        />
      {/snippet}
    </SettingRow>
    <SettingRow
      label={t("options.corePrivilege")}
      description={t("options.corePrivilegeDesc")}
    >
      {#snippet control()}
        <div class="perm-ctl">
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
        </div>
      {/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3>{t("options.languageCard")}</h3>
    <SettingRow label={t("options.language")} description={t("options.languageDesc")}>
      {#snippet control()}
        <select class="sel" bind:value={s.language}>
          <option value={LANG_AUTO}>{t("options.languageAuto")}</option>
          {#each LANGS as tag (tag)}
            <option value={tag}>{LANG_NAMES[tag]}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3>{t("options.logCard")}</h3>
    <SettingRow
      label={t("options.logRetention")}
      description={t("options.logRetentionDesc")}
    >
      {#snippet control()}
        <select class="sel" bind:value={s.log_retention_days}>
          <option value={0}>{t("options.logOff")}</option>
          {#each [3, 7, 14, 30] as days (days)}
            <option value={days}>{t("options.logDays", { n: days })}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>
    <SettingRow
      label={t("options.logLevel")}
      description={t("options.logLevelDesc")}
      disabled={logDisabled}
    >
      {#snippet control()}
        <select class="sel" bind:value={s.log_level} disabled={logDisabled}>
          {#each LOG_LEVELS as level (level.value)}
            <option value={level.value}>{t(level.label)}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3>{t("options.toolsCard")}</h3>
    <SettingRow
      label={t("options.restoreTool")}
      description={t("options.restoreToolDesc")}
    >
      {#snippet control()}
        <button class="btn" onclick={openRestoreTool}>{t("common.open")}</button>
      {/snippet}
    </SettingRow>
  </section>
</div>

<style>
  .note {
    padding: 10px 14px;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.6;
    border-top: 1px solid var(--border);
  }
  .note.disabled {
    opacity: 0.45;
  }
  .note .link {
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
  }
  .note .link:hover {
    text-decoration: underline;
  }
  .note .link:disabled {
    color: var(--muted);
    cursor: not-allowed;
    text-decoration: none;
  }
  .perm-ctl {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .autostart-ctl {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .autostart-ctl .method {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
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
</style>
