<script>
  import Card from "./Card.svelte";
  import { app } from "../lib/state.svelte.js";

  let updateResult = $state({ text: "", error: false, checking: false });

  async function checkUpdate() {
    updateResult = { text: "检查中…", error: false, checking: true };
    try {
      const resp = await fetch(app.info?.update_feed, { cache: "no-store" });
      if (!resp.ok) throw new Error("HTTP " + resp.status);
      const releases = await resp.json();
      if (!Array.isArray(releases) || releases.length === 0) {
        throw new Error("未获取到版本信息");
      }
      releases.sort((a, b) => new Date(b.published_at) - new Date(a.published_at));
      const latest = releases[0];
      updateResult = {
        text: "最新版本：" + (latest.name || latest.tag_name || "未知"),
        url: latest.html_url || app.info?.website,
        error: false,
        checking: false,
      };
    } catch (err) {
      updateResult = { text: "检查更新失败：" + err.message, error: true, checking: false };
    }
  }
</script>

<div class="panel-stack">
  <div class="hero">
    <img class="logo" src="/icon.ico" alt="Boss Key" />
    <h2>{app.info?.name ?? "Boss Key"}</h2>
    <p class="muted">版本 {app.info?.version ?? "…"}</p>
    <p>老板来了？一键隐藏窗口，上班摸鱼必备神器。</p>
    <a href={app.info?.website} target="_blank" rel="noreferrer">项目主页</a>
  </div>

  <Card title="检查更新">
    <div class="update-row">
      <button class="btn" onclick={checkUpdate} disabled={updateResult.checking}>检查更新</button>
      <span class:error={updateResult.error} class="muted result">
        {#if updateResult.url}
          <a href={updateResult.url} target="_blank" rel="noreferrer">{updateResult.text}</a>
        {:else}
          {updateResult.text}
        {/if}
      </span>
    </div>
  </Card>
</div>

<style>
  .hero {
    text-align: center;
    padding: 22px 16px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: center;
  }
  .hero .logo {
    width: 56px;
    height: 56px;
  }
  .hero h2 {
    font-size: 20px;
  }
  .update-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .result.error {
    color: var(--danger);
  }
</style>
