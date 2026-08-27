<script>
  // 启动时弹出的未读公告。
  import IconMegaphone from "~icons/lucide/megaphone";
  import Markdown from "./Markdown.svelte";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import { analyticsUnanswered, app, markAnnouncementSeen } from "../lib/state.svelte.js";
  import { formatTime } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  // 首次启动要先答完统计授权，别让两层弹窗叠在一起。
  const item = $derived(analyticsUnanswered() ? null : app.pendingAnnouncement);

  // 关闭（含 Esc / 点遮罩）即记为已读。
  let open = $state(false);
  $effect(() => {
    open = !!item;
  });
  function onOpenChange(v) {
    if (!v && item) markAnnouncementSeen(item.id);
    open = v;
  }
</script>

{#if item}
  <ContentDialog title={t("announce.title")} bind:open={() => open, onOpenChange}>
    <div class="ann">
      <h4>
        <IconMegaphone width="15" height="15" />
        {item.title}
        {#if item.is_pinned}<span class="pin">{t("announce.pinned")}</span>{/if}
      </h4>
      <p class="meta">
        {#if item.author}{item.author} ·{/if}
        {formatTime(item.published_at)}
      </p>
      <Markdown source={item.content} />
    </div>

    {#snippet footer()}
      <button class="btn primary" onclick={() => markAnnouncementSeen(item.id)}>{t("announce.gotIt")}</button>
    {/snippet}
  </ContentDialog>
{/if}

<style>
  .ann {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h4 {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 14px;
    font-weight: 600;
  }
  .pin {
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 600;
  }
  .meta {
    font-size: 12px;
    color: var(--text-2);
  }
</style>
