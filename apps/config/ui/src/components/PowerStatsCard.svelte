<script>
  // 能效统计：把累计冻结 / 效率模式 / 内存释放折成节能与减碳成绩。
  // 数据由核心独占写盘，这里只读、只展示；重置交回核心执行。
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import IconSprout from "~icons/lucide/sprout";
  import IconSnowflake from "~icons/lucide/snowflake";
  import IconLeaf from "~icons/lucide/leaf";
  import IconClock from "~icons/lucide/clock";
  import IconMemoryStick from "~icons/lucide/memory-stick";
  import IconZap from "~icons/lucide/zap";
  import IconRotateCcw from "~icons/lucide/rotate-ccw";
  import { invoke } from "../lib/ipc.js";
  import { toast } from "../lib/state.svelte.js";
  import { lang, t } from "../lib/i18n.svelte.js";
  import {
    EMPTY_STATS,
    derive,
    formatBytes,
    formatCo2,
    formatCount,
    formatDuration,
    formatEnergy,
    formatSince,
    formatTreeDays,
  } from "../lib/powerstats.js";

  /** 刷新间隔；核心的落盘节流是 2 秒，比它宽松即可。 */
  const REFRESH_MS = 5000;

  let stats = $state({ ...EMPTY_STATS });
  let confirmOpen = $state(false);
  let resetting = $state(false);

  const eco = $derived(derive(stats));
  const since = $derived(formatSince(stats.since, lang()));
  /** 一笔都还没记过：主砖改说引导语，免得摆一串零还染成绿的。 */
  const blank = $derived(
    !stats.freeze_count && !stats.efficiency_count && !stats.memory_freed_bytes,
  );

  const co2 = $derived(formatCo2(eco.co2Kg));
  const energy = $derived(formatEnergy(eco.kwh));
  const memory = $derived(formatBytes(stats.memory_freed_bytes));
  const frozen = $derived(formatDuration(stats.freeze_seconds, lang()));
  const times = $derived(t("power.statsUnitTimes"));

  async function refresh() {
    try {
      stats = (await invoke("power_stats")) ?? { ...EMPTY_STATS };
    } catch {
      /* 统计读不到不值得打扰用户，下次轮询再试 */
    }
  }

  // 页面不可见时不轮询，与核心状态轮询同一套做法。
  $effect(() => {
    refresh();
    const timer = setInterval(() => {
      if (document.visibilityState === "visible") refresh();
    }, REFRESH_MS);
    return () => clearInterval(timer);
  });

  async function reset() {
    resetting = true;
    try {
      await invoke("reset_power_stats");
      stats = { ...EMPTY_STATS };
      confirmOpen = false;
      toast(t("power.statsResetDone"));
      // 核心刚清完盘，回读一次拿到它盖上的新起算时刻。
      refresh();
    } catch (err) {
      toast(t("power.statsResetFailed", { err }), true);
    } finally {
      resetting = false;
    }
  }
</script>

<!-- 一块砖：图标压在角上，数值撑场面，标签收底 -->
{#snippet tile(Icon, value, unit, label, hint)}
  <span class="ico" aria-hidden="true"><Icon width="18" height="18" /></span>
  <div class="num">{value}<span class="unit">{unit}</span></div>
  <div class="label">{label}</div>
  {#if hint}<div class="hint">{hint}</div>{/if}
{/snippet}

<SettingsGroup title={t("power.statsCard")}>
  <div class="stats">
    <div class="bento">
      <div class="tile lead" class:blank title={t("power.statsCo2Desc")}>
        {#if blank}
          <span class="ico" aria-hidden="true"><IconSprout width="18" height="18" /></span>
          <div class="empty">{t("power.statsEmpty")}</div>
          <div class="hint">{t("power.statsEmptyHint")}</div>
        {:else}
          {@render tile(
            IconSprout,
            co2.value,
            co2.unit,
            t("power.statsCo2Label"),
            t("power.statsCo2Hint", { trees: formatTreeDays(eco.treeDays) }),
          )}
        {/if}
      </div>

      <div class="tile" title={t("power.statsFreezeDesc")}>
        {@render tile(
          IconSnowflake,
          formatCount(stats.freeze_count, lang()),
          times,
          t("power.statsFreeze"),
        )}
      </div>

      <div class="tile" title={t("power.statsEfficiencyDesc")}>
        {@render tile(
          IconLeaf,
          formatCount(stats.efficiency_count, lang()),
          times,
          t("power.statsEfficiency"),
        )}
      </div>

      <div class="tile wide" title={t("power.statsDurationDesc")}>
        {@render tile(IconClock, frozen.value, t(frozen.unitKey), t("power.statsDuration"))}
      </div>

      <div class="tile wide" title={t("power.statsEnergyDesc")}>
        {@render tile(IconZap, energy.value, energy.unit, t("power.statsEnergy"))}
      </div>

      <div class="tile wide" title={t("power.statsMemoryDesc")}>
        {@render tile(IconMemoryStick, memory.value, memory.unit, t("power.statsMemory"))}
      </div>
    </div>

    <div class="foot">
      <span class="note">
        {since ? t("power.statsSince", { date: since }) : t("power.statsSinceNever")}
        · {t("power.statsEstimate")}
      </span>
      <button class="btn" onclick={() => (confirmOpen = true)} disabled={blank}>
        <IconRotateCcw width="14" height="14" />
        {t("power.statsReset")}
      </button>
    </div>
  </div>
</SettingsGroup>

<ContentDialog bind:open={confirmOpen} title={t("power.statsResetTitle")}>
  <p class="confirm">{t("power.statsResetBody")}</p>

  {#snippet footer()}
    <button class="btn" onclick={() => (confirmOpen = false)} disabled={resetting}>
      {t("common.cancel")}
    </button>
    <button class="btn primary" onclick={reset} disabled={resetting}>
      {t("power.statsReset")}
    </button>
  {/snippet}
</ContentDialog>

<style>
  /* 容器查询而非媒体查询：左侧导航会吃掉窗口宽度，按整窗算断点会切错。
     容器查询管不到容器自己，栅格因此得单开一层。 */
  .stats {
    container-type: inline-size;
  }
  .bento {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    min-width: 0;
    padding: 12px 16px 14px;
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
  }

  /* 减碳那块占四格，是整面墙的主角 */
  .lead {
    grid-column: span 2;
    grid-row: span 2;
    justify-content: center;
    background: color-mix(in srgb, var(--accent) 12%, var(--card));
  }
  /* 还没成绩时收敛成普通卡面，免得空数据也一片绿 */
  .lead.blank {
    background: var(--card);
    justify-content: flex-start;
  }
  .wide {
    grid-column: span 2;
  }

  .ico {
    display: inline-flex;
    margin-bottom: 6px;
    color: var(--text-3);
  }
  .lead:not(.blank) .ico {
    color: var(--accent);
  }

  /* 等宽数字，轮询刷新时数值不会左右跳动 */
  .num {
    font-family: var(--font-display);
    font-size: 24px;
    line-height: 30px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .lead .num {
    font-size: 34px;
    line-height: 42px;
  }
  .unit {
    margin-left: 4px;
    font-size: 13px;
    font-weight: 400;
    color: var(--text-2);
  }
  .lead .unit {
    font-size: 16px;
  }

  .label {
    margin-top: 2px;
    font-size: 12px;
    line-height: 16px;
    color: var(--text-2);
  }
  .empty {
    font-size: 16px;
    line-height: 22px;
    font-weight: 600;
  }
  .hint {
    margin-top: 8px;
    font-size: 12px;
    line-height: 16px;
    color: var(--text-2);
  }
  .lead:not(.blank) .hint {
    color: var(--accent);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 8px 2px 0;
  }
  .note {
    font-size: 12px;
    line-height: 16px;
    color: var(--text-3);
  }

  .confirm {
    font-size: 13px;
    line-height: 1.6;
  }

  /* 窄到放不下四列就收成两列：主砖独占一整行，不再需要跨行 */
  @container (max-width: 520px) {
    .bento {
      grid-template-columns: repeat(2, 1fr);
    }
    .lead {
      grid-row: auto;
    }
  }
</style>
