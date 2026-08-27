<script>
  // 首次启动征求匿名使用统计的授权。必须由用户明确选择，关不掉也点不出去；
  // 在做出选择之前后端一个字节都不会采集或写入。
  import IconChartNoAxesColumn from "~icons/lucide/chart-no-axes-column";
  import IconCheck from "~icons/lucide/check";
  import IconBan from "~icons/lucide/ban";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import { app, analyticsUnanswered, setAnalyticsConsent } from "../lib/state.svelte.js";
  import { t } from "../lib/i18n.svelte.js";

  const open = $derived(analyticsUnanswered());
  let deciding = $state(false);

  async function decide(granted) {
    if (deciding) return;
    deciding = true;
    try {
      await setAnalyticsConsent(granted, "first_run");
    } finally {
      deciding = false;
    }
  }
</script>

{#if app.config}
  <ContentDialog title={t("consent.title")} closable={false} bind:open={() => open, () => {}}>
    <div class="body">
      <h4>
        <IconChartNoAxesColumn width="15" height="15" />
        {t("consent.heading")}
      </h4>
      <p>{t("consent.intro")}</p>

      <p class="label"><IconCheck width="13" height="13" />{t("consent.collectTitle")}</p>
      <ul>
        <li>{t("consent.collectFeatures")}</li>
        <li>{t("consent.collectScale")}</li>
        <li>{t("consent.collectEnv")}</li>
        <li>{t("consent.collectId")}</li>
      </ul>

      <p class="label"><IconBan width="13" height="13" />{t("consent.neverTitle")}</p>
      <ul>
        <li>{t("consent.neverHide")}</li>
        <li>{t("consent.neverContent")}</li>
        <li>{t("consent.neverIdentity")}</li>
      </ul>

      <p class="hint">{t("consent.control")}</p>
    </div>

    {#snippet footer()}
      <button class="btn" disabled={deciding} onclick={() => decide(false)}>
        {t("consent.decline")}
      </button>
      <button class="btn primary" disabled={deciding} onclick={() => decide(true)}>
        {t("consent.agree")}
      </button>
    {/snippet}
  </ContentDialog>
{/if}

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
    line-height: 1.6;
  }
  h4 {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 14px;
    font-weight: 600;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    font-weight: 600;
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-left: 18px;
    list-style: disc;
    color: var(--text-2);
  }
  .hint {
    margin-top: 4px;
    color: var(--text-2);
  }
</style>
