<script>
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import InfoBar from "./fluent/InfoBar.svelte";
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
  const scopeOptions = $derived(
    SCOPES.map((sc) => ({ value: sc.value, label: t(sc.label) })),
  );

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

  // 主开关一开就把子项摊开，省得用户再点一次才看见。
  // 展开态由 SettingsExpander 的 autoExpand 负责联动，这里只存状态。
  let expanded = $state(false);
</script>

<SettingsGroup title={t("power.freezeCard")}>
  <SettingsExpander
    bind:open={expanded}
    autoExpand={s.freeze_after_hide}
    icon={IconSnowflake}
    label={t("power.freezeAfterHide")}
    description={t("power.freezeAfterHideDesc")}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.freeze_after_hide} />{/snippet}

    <SettingsCard
      variant="sub"
      icon={IconZap}
      label={t("power.enhancedFreeze")}
      disabled={freezeOff || enhancedDisabled}
      description={!freezeOff && enhancedDisabled
        ? t("power.enhancedFreezeBlocked", { reasons: enhancedBlocked.join("；") })
        : t("power.enhancedFreezeDesc")}
    >
      {#snippet control()}
        <ToggleSwitch
          bind:checked={s.enhanced_freeze}
          disabled={freezeOff || enhancedDisabled}
          title={freezeOff
            ? t("power.needFreezeFirst")
            : enhancedDisabled
              ? enhancedBlocked.join("；")
              : ""}
        />
      {/snippet}
    </SettingsCard>

    <SettingsCard
      variant="sub"
      icon={IconCrosshair}
      label={t("power.scope")}
      disabled={freezeOff}
      description={t("power.scopeDesc")}
    >
      {#snippet control()}
        <ComboBox
          bind:value={s.power_scope}
          options={scopeOptions}
          disabled={freezeOff}
          title={freezeOff ? t("power.needFreezeFirst") : ""}
          ariaLabel={t("power.scope")}
        />
      {/snippet}
    </SettingsCard>

    <div class="sub-note">
      <InfoBar disabled={freezeOff}>
        {t("power.freezeNoteBefore")}
        <button
          class="link"
          onclick={() => openLink("https://download.sysinternals.com/files/PSTools.zip")}
          disabled={freezeOff}>PSTools</button
        >
        {t("power.freezeNoteAfter")}
        <button class="link" onclick={refreshPssuspend} disabled={freezeOff}>
          {t("power.recheck")}
        </button>
      </InfoBar>
    </div>
  </SettingsExpander>
</SettingsGroup>

<SettingsGroup title={t("power.memoryCard")}>
  <SettingsCard
    icon={IconMemoryStick}
    label={t("power.trimMemory")}
    disabled={freezeOff}
    description={t("power.trimMemoryDesc")}
  >
    {#snippet control()}
      <ToggleSwitch
        bind:checked={s.trim_memory_after_freeze}
        disabled={freezeOff}
        title={freezeOff ? t("power.needFreezeFirst") : ""}
      />
    {/snippet}
  </SettingsCard>
</SettingsGroup>

<style>
  .sub-note {
    padding: 12px 16px 16px 20px;
    border-top: 1px solid var(--divider);
  }
  .link {
    color: var(--accent);
    font: inherit;
    padding: 0;
  }
  .link:hover {
    text-decoration: underline;
  }
  .link:disabled {
    color: var(--text-disabled);
    cursor: not-allowed;
    text-decoration: none;
  }
</style>
