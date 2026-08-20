<script>
  // 更新提示；required=true 时关不掉，交给 ContentDialog 的 closable=false。
  import IconDownload from "~icons/lucide/download";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import InfoBar from "./fluent/InfoBar.svelte";
  import Markdown from "./Markdown.svelte";
  import { win } from "../lib/ipc.js";
  import { app, toast } from "../lib/state.svelte.js";
  import { downloadUrl, formatTime, openExternal } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const target = $derived(app.update?.target_version ?? app.update?.latest_version ?? null);
  const forced = $derived(!!app.update?.required);
  const url = $derived(downloadUrl(target));

  async function download() {
    if (!url) return toast(t("update.noDownloadUrl"), true);
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("update.openDownloadFailed", { err }), true);
    }
  }

  function setOpen(v) {
    if (forced) return; // 强制更新关不掉
    app.updateOpen = v;
  }
</script>

{#if target}
  <ContentDialog
    title={t(forced ? "update.forcedTitle" : "update.title")}
    closable={!forced}
    bind:open={() => app.updateOpen, setOpen}
  >
    <div class="body">
      {#if forced}
        <InfoBar severity="error">{t("update.forcedNote")}</InfoBar>
      {/if}

      <div class="ver">
        <span class="tag">{target.version}</span>
        {#if target.is_preview}<span class="tag preview">{t("update.preview")}</span>{/if}
        <span class="muted">
          {t("update.current", { version: app.info?.version ?? "…" })}
          {#if target.published_at}{t("update.publishedAt", { time: formatTime(target.published_at) })}{/if}
        </span>
      </div>

      {#if target.title}<p class="type-body-strong">{target.title}</p>{/if}
      {#if target.content}<div class="notes"><Markdown source={target.content} /></div>{/if}
    </div>

    {#snippet footer()}
      {#if forced}
        <button class="btn" onclick={() => win.close()}>{t("update.quitApp")}</button>
      {:else}
        <button class="btn" onclick={() => setOpen(false)}>{t("update.later")}</button>
      {/if}
      <button class="btn primary" onclick={download}>
        <IconDownload width="14" height="14" /> {t("update.download")}
      </button>
    {/snippet}
  </ContentDialog>
{/if}

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .ver {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .tag {
    padding: 2px 9px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 600;
  }
  .tag.preview {
    background: var(--warn);
    color: var(--solid-bg);
  }
  .muted {
    font-size: 12px;
    color: var(--text-2);
  }
  .notes {
    padding: 12px 14px;
    background: var(--card-2);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
    font-size: 12.5px;
    max-height: 240px;
    overflow-y: auto;
  }
</style>
