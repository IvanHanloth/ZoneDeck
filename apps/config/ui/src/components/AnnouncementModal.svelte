<script>
  // 启动时弹出的未读公告。
  import IconMegaphone from "~icons/lucide/megaphone";
  import Modal from "./Modal.svelte";
  import { app, markAnnouncementSeen } from "../lib/state.svelte.js";
  import { formatTime } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const item = $derived(app.pendingAnnouncement);

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
  <Modal title={t("announce.title")} bind:open={() => open, onOpenChange}>
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
      <pre class="content">{item.content}</pre>
    </div>

    {#snippet footer()}
      <button class="btn primary" onclick={() => markAnnouncementSeen(item.id)}>{t("announce.gotIt")}</button>
    {/snippet}
  </Modal>
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
    color: var(--muted);
  }
  .content {
    margin: 0;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.65;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
