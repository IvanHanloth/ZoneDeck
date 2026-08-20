<script>
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import IconCrosshair from "~icons/lucide/crosshair";
  import IconSnowflake from "~icons/lucide/snowflake";
  import IconZap from "~icons/lucide/zap";
  import IconMemoryStick from "~icons/lucide/memory-stick";
  import { app, refreshPssuspend, toast } from "../lib/state.svelte.js";
  import { openExternal } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const s = $derived(app.config.setting);

  const SCOPES = [
    { value: "self", label: "power.scopeSelf" },
    { value: "tree", label: "power.scopeTree" },
    { value: "image", label: "power.scopeImage" },
  ];

  async function openLink(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("options.openLinkFailed", { err }), true);
    }
  }

  // 未开启「隐藏窗口时冻结进程」时，本页其余选项一律置灰禁用 ——
  // 清空工作集只对停摆的进程有意义，作用范围也就无从谈起。
  const freezeOff = $derived(!s.freeze_after_hide);

  // 增强冻结的前置条件；任一不满足即置灰。
  const enhancedBlocked = $derived.by(() => {
    const reasons = [];
    if (!app.status.running) reasons.push(t("power.blockedCoreStopped"));
    else if (!app.status.elevated) reasons.push(t("power.blockedNeedAdmin"));
    if (!app.pssuspend) reasons.push(t("power.blockedNoPssuspend"));
    return reasons;
  });
  const enhancedDisabled = $derived(enhancedBlocked.length > 0);
</script>

<div class="panel-stack">
  <section class="fcard">
    <h3>{t("power.scopeCard")}</h3>
    <SettingRow
      icon={IconCrosshair}
      label={t("power.scope")}
      disabled={freezeOff}
      description={t("power.scopeDesc")}
    >
      {#snippet control()}
        <select
          class="sel"
          bind:value={s.power_scope}
          disabled={freezeOff}
          title={freezeOff ? t("power.needFreezeFirst") : ""}
        >
          {#each SCOPES as scope (scope.value)}
            <option value={scope.value}>{t(scope.label)}</option>
          {/each}
        </select>
      {/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3>{t("power.freezeCard")}</h3>
    <SettingRow
      icon={IconSnowflake}
      label={t("power.freezeAfterHide")}
      description={t("power.freezeAfterHideDesc")}
    >
      {#snippet control()}<Toggle bind:checked={s.freeze_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow
      icon={IconZap}
      label={t("power.enhancedFreeze")}
      disabled={freezeOff || enhancedDisabled}
      description={!freezeOff && enhancedDisabled
        ? t("power.enhancedFreezeBlocked", { reasons: enhancedBlocked.join("；") })
        : t("power.enhancedFreezeDesc")}
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.enhanced_freeze}
          disabled={freezeOff || enhancedDisabled}
          title={freezeOff
            ? t("power.needFreezeFirst")
            : enhancedDisabled
              ? enhancedBlocked.join("；")
              : ""}
        />
      {/snippet}
    </SettingRow>
    <div class="note" class:disabled={freezeOff}>
      {t("power.freezeNoteBefore")}
      <button class="link" onclick={() => openLink("https://download.sysinternals.com/files/PSTools.zip")} disabled={freezeOff}>PSTools</button>
      {t("power.freezeNoteAfter")}
      <button class="link" onclick={refreshPssuspend} disabled={freezeOff}>{t("power.recheck")}</button>
    </div>
  </section>

  <section class="fcard">
    <h3>{t("power.memoryCard")}</h3>
    <SettingRow
      icon={IconMemoryStick}
      label={t("power.trimMemory")}
      disabled={freezeOff}
      description={t("power.trimMemoryDesc")}
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.trim_memory_after_freeze}
          disabled={freezeOff}
          title={freezeOff ? t("power.needFreezeFirst") : ""}
        />
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
  .sel:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
