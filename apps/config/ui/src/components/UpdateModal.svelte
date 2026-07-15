<script>
  // 更新提示。强制更新（Verhub 的 required=true）时**不能关**：没有右上角的叉、
  // Esc 和点遮罩都不管用，只能去下载或退出程序——这正是「强制」的含义。
  // 因此这里不用通用的 Modal 组件（那个总是可关闭的）。
  import IconDownload from "~icons/lucide/download";
  import IconTriangleAlert from "~icons/lucide/triangle-alert";
  import IconX from "~icons/lucide/x";
  import { win } from "../lib/ipc.js";
  import { app, toast } from "../lib/state.svelte.js";
  import { downloadUrl, formatTime, openExternal } from "../lib/verhub.js";

  const target = $derived(app.update?.target_version ?? app.update?.latest_version ?? null);
  const forced = $derived(!!app.update?.required);
  const url = $derived(downloadUrl(target));

  async function download() {
    if (!url) return toast("该版本没有提供下载地址，请前往项目主页获取", true);
    try {
      await openExternal(url);
    } catch (err) {
      toast("打开下载页失败：" + err, true);
    }
  }

  function close() {
    if (forced) return; // 强制更新关不掉
    app.updateOpen = false;
  }

  function onKey(e) {
    if (e.key === "Escape") close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if app.updateOpen && target}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label="发现新版本">
      <header class="dhead" class:forced>
        <h3>
          {#if forced}<IconTriangleAlert width="15" height="15" />{/if}
          {forced ? "必须更新后才能继续使用" : "发现新版本"}
        </h3>
        {#if !forced}
          <button class="x" title="稍后再说" aria-label="稍后再说" onclick={close}>
            <IconX width="15" height="15" />
          </button>
        {/if}
      </header>

      <div class="dbody">
        <div class="ver">
          <span class="tag">{target.version}</span>
          {#if target.is_preview}<span class="tag preview">预览版</span>{/if}
          <span class="muted">
            当前 {app.info?.version ?? "…"}
            {#if target.published_at}· 发布于 {formatTime(target.published_at)}{/if}
          </span>
        </div>

        {#if target.title}<p class="title">{target.title}</p>{/if}
        {#if target.content}<pre class="notes">{target.content}</pre>{/if}

        {#if forced}
          <p class="warn">
            这是一个强制更新，旧版本已停止服务。请下载新版本后重新启动 Boss Key。
          </p>
        {/if}
      </div>

      <footer class="dfoot">
        {#if forced}
          <button class="btn ghost" onclick={() => win.close()}>退出程序</button>
        {:else}
          <button class="btn ghost" onclick={close}>稍后再说</button>
        {/if}
        <button class="btn primary" onclick={download}>
          <IconDownload width="14" height="14" /> 前往下载
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100; /* 压在其它弹窗之上：强制更新时别的都不该能操作 */
    padding: 24px;
  }
  .dialog {
    width: min(560px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    overflow: hidden;
  }
  .dhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .dhead h3 {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 14px;
    font-weight: 600;
  }
  .dhead.forced h3 {
    color: var(--danger);
  }
  .x {
    color: var(--muted);
    padding: 2px 6px;
    border-radius: 6px;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  .dbody {
    padding: 14px 16px;
    overflow-y: auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
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
  }
  .muted {
    font-size: 12px;
    color: var(--muted);
  }
  .title {
    font-size: 14px;
    font-weight: 600;
  }
  .notes {
    margin: 0;
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-family: inherit;
    font-size: 12.5px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 240px;
    overflow-y: auto;
  }
  .warn {
    font-size: 12.5px;
    color: var(--danger);
  }
  .dfoot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
  .btn.primary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
