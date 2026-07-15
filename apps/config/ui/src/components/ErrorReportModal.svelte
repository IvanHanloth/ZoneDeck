<script>
  // 出错弹框：Boss Key 出问题时唯一会把日志发出去的地方。
  //
  // 日志默认**不上报**。这里先把将要发送的内容原样摆给用户看（错误信息 + 本地日志末尾），
  // 他点了「上报给开发者」才真的发。不点就只是个普通的错误提示。
  import IconTriangleAlert from "~icons/lucide/triangle-alert";
  import Modal from "./Modal.svelte";
  import { app, toast } from "../lib/state.svelte.js";
  import { recentLogTail, uploadLog } from "../lib/verhub.js";

  const report = $derived(app.errorReport);

  let logTail = $state("");
  let sending = $state(false);
  let sent = $state(false);

  // 弹框一出现就把本地日志尾巴取来给用户过目——上报什么，他事先看得见。
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
      `错误：${report?.message ?? ""}`,
      report?.detail ? `详情：${report.detail}` : "",
      logTail ? `\n最近日志：\n${logTail}` : "",
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
      toast("已上报，谢谢！");
      setTimeout(() => (app.errorReport = null), 700);
    } catch (err) {
      toast("上报失败：" + err, true);
    } finally {
      sending = false;
    }
  }
</script>

{#if report}
  <Modal title="出错了" bind:open={() => open, onOpenChange}>
    <div class="body">
      <p class="msg"><IconTriangleAlert width="15" height="15" /> {report.message}</p>
      {#if report.detail}<p class="detail">{report.detail}</p>{/if}

      <details>
        <summary>展开查看将要发送的内容（日志里可能含窗口标题与程序路径，请先过目）</summary>
        <pre class="payload">{payload}</pre>
      </details>
      <p class="hint">日志只在你点下面的按钮时才会发送，Boss Key 不会自动上报任何东西。</p>
    </div>

    {#snippet footer()}
      <button class="btn ghost" onclick={() => (app.errorReport = null)}>不上报</button>
      <button class="btn primary" onclick={send} disabled={sending || sent}>
        {sending ? "上报中…" : sent ? "已上报" : "上报给开发者"}
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
