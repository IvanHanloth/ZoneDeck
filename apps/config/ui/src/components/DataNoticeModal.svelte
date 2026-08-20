<script>
  // 便携版写不进程序目录时的提示：设置存到了哪里、如何改回去。
  import IconAlert from "~icons/lucide/triangle-alert";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import { app } from "../lib/state.svelte.js";
  import { t } from "../lib/i18n.svelte.js";

  const loc = $derived(app.dataLocation);
</script>

{#if loc}
  <ContentDialog title={t("dataNotice.title")} bind:open={app.dataNoticeOpen}>
    <div class="notice">
      <h4>
        <IconAlert width="15" height="15" />
        {t("dataNotice.heading")}
      </h4>
      <p>{t("dataNotice.reason", { dir: loc.program_dir })}</p>
      <p>{t("dataNotice.stored", { dir: loc.dir })}</p>
      <p class="hint">{t("dataNotice.fixTitle")}</p>
      <ul>
        <li>{t("dataNotice.fixMove")}</li>
        <li>{t("dataNotice.fixPermission")}</li>
        <li>{t("dataNotice.fixKeep")}</li>
      </ul>
    </div>

    {#snippet footer()}
      <button class="btn primary" onclick={() => (app.dataNoticeOpen = false)}>{t("dataNotice.gotIt")}</button>
    {/snippet}
  </ContentDialog>
{/if}

<style>
  .notice {
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
  .hint {
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
</style>
