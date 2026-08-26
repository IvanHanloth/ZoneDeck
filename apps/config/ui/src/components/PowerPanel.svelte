<script>
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import SettingsExpander from "./fluent/SettingsExpander.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import ComboBox from "./fluent/ComboBox.svelte";
  import PowerStatsCard from "./PowerStatsCard.svelte";
  import IconCrosshair from "~icons/lucide/crosshair";
  import IconSnowflake from "~icons/lucide/snowflake";
  import IconZap from "~icons/lucide/zap";
  import IconLeaf from "~icons/lucide/leaf";
  import IconFileSearch from "~icons/lucide/file-search";
  import IconMemoryStick from "~icons/lucide/memory-stick";
  import IconDownload from "~icons/lucide/download";
  import { app, openProgramDir, refreshPssuspend, toast } from "../lib/state.svelte.js";
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

  const PSTOOLS_URL = "https://download.sysinternals.com/files/PSTools.zip";

  async function openLink(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("options.openLinkFailed", { err }), true);
    }
  }

  // 未开启「隐藏窗口时冻结进程」时，冻结相关选项一律置灰。
  const freezeOff = $derived(!s.freeze_after_hide);
  const efficiencyOff = $derived(!s.efficiency_after_hide);
  // 两套能效手段都没开，作用范围整组没有可调的东西。
  const scopeOff = $derived(freezeOff && efficiencyOff);

  // 增强冻结的前置条件；任一不满足即置灰。
  const enhancedBlocked = $derived.by(() => {
    const reasons = [];
    if (!app.status.running) reasons.push(t("power.blockedCoreStopped"));
    else if (!app.status.elevated) reasons.push(t("power.blockedNeedAdmin"));
    if (!app.pssuspend) reasons.push(t("power.blockedNoPssuspend"));
    return reasons;
  });
  const enhancedDisabled = $derived(enhancedBlocked.length > 0);

  // 主开关一开就把子项摊开。
  // 展开态由 SettingsExpander 的 autoExpand 负责联动，这里只存状态。
  let expanded = $state(false);
  let scopeExpanded = $state(false);
</script>

<SettingsGroup title={t("power.efficiencyCard")}>
  <SettingsCard
    icon={IconLeaf}
    label={t("power.efficiencyAfterHide")}
    description={t("power.efficiencyAfterHideDesc")}
  >
    {#snippet control()}<ToggleSwitch bind:checked={s.efficiency_after_hide} />{/snippet}
  </SettingsCard>
</SettingsGroup>

<SettingsGroup title={t("power.scopeCard")}>
  <SettingsExpander
    bind:open={scopeExpanded}
    icon={IconCrosshair}
    label={t("power.scope")}
    disabled={scopeOff}
    description={t("power.scopeDesc")}
  >
    <SettingsCard
      variant="sub"
      icon={IconSnowflake}
      label={t("power.scopeFreeze")}
      disabled={freezeOff}
      description={t("power.scopeFreezeDesc")}
    >
      {#snippet control()}
        <ComboBox
          bind:value={s.power_scope}
          options={scopeOptions}
          disabled={freezeOff}
          title={freezeOff ? t("power.needFreezeFirst") : ""}
          ariaLabel={t("power.scopeFreeze")}
        />
      {/snippet}
    </SettingsCard>

    <SettingsCard
      variant="sub"
      icon={IconLeaf}
      label={t("power.scopeEfficiency")}
      disabled={efficiencyOff}
      description={t("power.scopeEfficiencyDesc")}
    >
      {#snippet control()}
        <ComboBox
          bind:value={s.efficiency_scope}
          options={scopeOptions}
          disabled={efficiencyOff}
          title={efficiencyOff ? t("power.needEfficiencyFirst") : ""}
          ariaLabel={t("power.scopeEfficiency")}
        />
      {/snippet}
    </SettingsCard>
  </SettingsExpander>
</SettingsGroup>

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
      icon={IconFileSearch}
      label={t("power.pssuspend")}
      disabled={freezeOff}
      description={t("power.pssuspendDesc")}
    >
      {#snippet control()}
        <strong class:missing={!app.pssuspend}>
          {t(app.pssuspend ? "power.pssuspendFound" : "power.pssuspendMissing")}
        </strong>
        <!-- 已就位就没什么可下载、可重测的了，只留一个「去看看」的入口 -->
        {#if !app.pssuspend}
          <button class="btn" onclick={() => openLink(PSTOOLS_URL)} disabled={freezeOff}>
            <IconDownload width="14" height="14" />
            {t("power.downloadPstools")}
          </button>
        {/if}
        <button class="btn" onclick={openProgramDir} disabled={freezeOff}>
          {t("power.openProgramDir")}
        </button>
        {#if !app.pssuspend}
          <button class="btn" onclick={refreshPssuspend} disabled={freezeOff}>
            {t("power.recheck")}
          </button>
        {/if}
      {/snippet}
    </SettingsCard>
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

<PowerStatsCard />

<style>
  /* 未检测到时用警告色，一眼能看出增强冻结还差这一步 */
  .missing {
    color: var(--warn);
  }
</style>
