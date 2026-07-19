<script>
  // 出错弹框：日志仅在用户确认后才上报，默认不上报。
  import IconTriangleAlert from "~icons/lucide/triangle-alert";
  import Modal from "./Modal.svelte";
  import { app, toast } from "../lib/state.svelte.js";
  import { recentLogTail, uploadLog } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const report = $derived(app.errorReport);

  let logTail = $state("");
  let sending = $state(false);
  let sent = $state(false);

  // 弹框出现时取本地日志尾部供用户过目。
  $effect(() => {
    if (!report) return;
    sent = false;
    logTail = "";
    recentLogTail(60)
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
  <Modal title={t("error.title")} bind:open={() => open, onOpenChange}>
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
  </Modal>
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
    color: var(--muted);
    word-break: break-word;
  }
  summary {
    font-size: 12.5px;
    color: var(--muted);
    cursor: pointer;
  }
  .payload {
    margin: 8px 0 0;
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 11.5px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow-y: auto;
  }
  .hint {
    font-size: 12px;
    color: var(--muted);
  }
</style>
