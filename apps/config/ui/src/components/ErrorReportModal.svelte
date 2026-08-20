<script>
  // 出错弹框：日志仅在用户确认后才上报，默认不上报。
  import IconTriangleAlert from "~icons/lucide/triangle-alert";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import { app, toast } from "../lib/state.svelte.js";
  import { currentSessionLog, uploadLog } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const report = $derived(app.errorReport);

  let logTail = $state("");
  let sending = $state(false);
  let sent = $state(false);

  $effect(() => {
    if (!report) return;
    sent = false;
    logTail = "";
    currentSessionLog()
      .then((t) => (logTail = t))
      .catch(() => (logTail = ""));
  });

  const payload = $derived(
    [
      t("error.payloadError", { message: report?.message ?? "" }),
      report?.detail ? t("error.payloadDetail", { detail: report.detail }) : "",
      logTail ? `\n${t("error.payloadLog")}\n${logTail}` : "",
    ]
      .filter(Boolean)
      .join("\n"),
  );

  let open = $state(false);
  $effect(() => {
    open = !!report;
  });
  function onOpenChange(v) {
    if (!v) app.errorReport = null;
    open = v;
  }

  async function send() {
    sending = true;
    try {
      await uploadLog(payload);
      sent = true;
      toast(t("error.reportThanks"));
      setTimeout(() => (app.errorReport = null), 700);
    } catch (err) {
      toast(t("error.reportFailed", { err }), true);
    } finally {
      sending = false;
    }
  }
</script>

{#if report}
  <ContentDialog title={t("error.title")} bind:open={() => open, onOpenChange}>
    <div class="body">
      <p class="msg"><IconTriangleAlert width="15" height="15" /> {report.message}</p>
      {#if report.detail}<p class="detail">{report.detail}</p>{/if}

      <details>
        <summary>{t("error.summary")}</summary>
        <pre class="payload">{payload}</pre>
      </details>
      <p class="hint">{t("error.hint")}</p>
    </div>

    {#snippet footer()}
      <button class="btn ghost" onclick={() => (app.errorReport = null)}>{t("error.dontReport")}</button>
      <button class="btn primary" onclick={send} disabled={sending || sent}>
        {sending ? t("error.reporting") : sent ? t("error.reported") : t("error.report")}
      </button>
    {/snippet}
  </ContentDialog>
{/if}

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .msg {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13.5px;
    color: var(--danger);
  }
  .detail {
    font-size: 12.5px;
    color: var(--text-2);
    word-break: break-word;
  }
  summary {
    font-size: 12.5px;
    color: var(--text-2);
    cursor: pointer;
  }
  .payload {
    margin: 8px 0 0;
    padding: 10px 12px;
    background: var(--card-2);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
    font-family: var(--font-mono);
    font-size: 11.5px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow-y: auto;
    user-select: text;
  }
  .hint {
    font-size: 12px;
    color: var(--text-2);
  }
</style>
