<script>
  import IconGlobe from "~icons/lucide/globe";
  import IconMail from "~icons/lucide/mail";
  import IconGithub from "~icons/lucide/github";
  import IconScale from "~icons/lucide/scale";
  import IconMegaphone from "~icons/lucide/megaphone";
  import IconRefreshCw from "~icons/lucide/refresh-cw";
  import IconStar from "~icons/lucide/star";
  import Card from "./Card.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import {
    app,
    checkForUpdate,
    loadAnnouncements,
    toast,
  } from "../lib/state.svelte.js";
  import { MIT_LICENSE } from "../lib/license.js";
  import { formatTime, openExternal, submitFeedback } from "../lib/verhub.js";

  const info = $derived(app.info);
  const v = $derived(app.config.verhub);
  const year = new Date().getFullYear();

  let content = $state("");
  let rating = $state(0);
  /** 鼠标悬停/滑过时预览的星级；0 表示没在悬停。点亮几颗看 hover||rating。 */
  let hoverRating = $state(0);
  let contact = $state("");
  let sending = $state(false);

  const litStars = $derived(hoverRating || rating);

  async function open(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast("打开链接失败：" + err, true);
    }
  }

  async function sendFeedback() {
    if (!content.trim()) return toast("请先写点什么", true);
    sending = true;
    try {
      await submitFeedback({
        content: content.trim(),
        rating: rating || null,
        contact: contact.trim(),
      });
      content = "";
      rating = 0;
      toast("反馈已提交，谢谢！");
    } catch (err) {
      toast("提交反馈失败：" + err, true);
    } finally {
      sending = false;
    }
  }

  // 进「关于」页就把公告拉一遍（不弹窗，只填列表）。
  $effect(() => {
    if (app.announcements.length === 0) loadAnnouncements();
  });

  const updateText = $derived.by(() => {
    if (app.updateChecking) return "检查中…";
    if (!app.update) return "";
    const t = app.update.target_version;
    return app.update.should_update
      ? `发现新版本 ${t?.version ?? ""}${app.update.required ? "（强制更新）" : ""}`
      : "已是最新版本";
  });

</script>

<div class="panel-stack">
  <div class="hero">
    <img class="logo" src="/icon.ico" alt="Boss Key" />
    <h2>{info?.name ?? "Boss Key"}</h2>
    <p class="muted">版本 {info?.version ?? "…"}</p>
    <p>老板来了？一键隐藏窗口，上班摸鱼必备神器。</p>
    <p>
        <button class="val link" onclick={() => open(info?.website ?? "https://github.com/IvanHanloth/Boss-Key")}>
          项目主页
        </button>
        
    </p>
    <p>Copyright © 2022-{year} 
        <button class="val link" onclick={() => open(info?.blog ?? "https://www.ivan-hanloth.cn/")}>
          Ivan Hanloth
        </button> All Rights Reserved.</p>
      
  </div>

  <Card title="更新">
    <SettingRow label="检查更新" description="手动检查是否有新版本。">
      {#snippet control()}
      <button class="btn" onclick={() => checkForUpdate(true)} disabled={app.updateChecking}>
        <IconRefreshCw width="14" height="14" /> 检查更新
      </button>
      <span class="muted result">{updateText}</span>
      {#if app.update?.should_update}
        <button class="btn primary" onclick={() => (app.updateOpen = true)}>查看详情</button>
      {/if}{/snippet}
    </SettingRow>

    <SettingRow label="接收预览版" description="允许接收预览版更新，可能有不稳定的情况。">
      {#snippet control()}<Toggle bind:checked={v.include_preview} />{/snippet}
    </SettingRow>
  </Card>

  <Card title="公告">
    {#if app.announcements.length === 0}
      <p class="empty">暂无公告</p>
    {:else}
      <ul class="anns">
        {#each app.announcements as a (a.id)}
          <li>
            <div class="ahead">
              <IconMegaphone width="13" height="13" />
              <span class="atitle">{a.title}</span>
              {#if a.is_pinned}<span class="pin">置顶</span>{/if}
              <span class="adate">{formatTime(a.published_at)}</span>
            </div>
            <pre class="acontent">{a.content}</pre>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>

  <Card title="反馈">
    <!-- 悬停/滑过第 n 颗星时，1..n 一并点亮（低等级自动填充）；再点已选的星取消评分。 -->
    <div
      class="stars"
      role="radiogroup"
      aria-label="评分"
      onpointerleave={() => (hoverRating = 0)}
    >
      {#each [1, 2, 3, 4, 5] as n (n)}
        <button
          type="button"
          class="star"
          class:on={litStars >= n}
          role="radio"
          aria-checked={rating === n}
          aria-label="{n} 星"
          onpointerenter={() => (hoverRating = n)}
          onfocus={() => (hoverRating = n)}
          onblur={() => (hoverRating = 0)}
          onclick={() => (rating = rating === n ? 0 : n)}
        >
          <IconStar width="20" height="20" fill={litStars >= n ? "currentColor" : "none"} />
        </button>
      {/each}
      <span class="muted">{rating ? `${rating} 星` : "评分（可选）"}</span>
    </div>

    <textarea
      class="fb"
      rows="4"
      maxlength="4000"
      placeholder="说说遇到的问题、想要的功能…"
      bind:value={content}
    ></textarea>
    <input
      type="text"
      maxlength="120"
      placeholder="联系方式（可选，邮箱 / QQ 等）"
      bind:value={contact}
    />

    <div class="fb-foot">
      <span class="muted">{content.length} / 4000</span>
      <button class="btn primary" onclick={sendFeedback} disabled={sending || !content.trim()}>
        {sending ? "提交中…" : "提交反馈"}
      </button>
    </div>
  </Card>

  <Card title="开源协议">
    <p class="card-hint">本项目基于 MIT 协议开源：</p>
    <pre class="license">{MIT_LICENSE}</pre>
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

  .links {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .links li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 9px 2px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .links li:last-child {
    border-bottom: none;
  }
  .k {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--muted);
    flex: none;
  }
  .val {
    min-width: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link {
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }

  .update-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .update-row .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .result {
    font-size: 12.5px;
  }

  .anns {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 300px;
    overflow-y: auto;
  }
  .anns li {
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .ahead {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
  }
  .atitle {
    font-weight: 600;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pin {
    flex: none;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 10.5px;
    font-weight: 600;
  }
  .adate {
    margin-left: auto;
    flex: none;
    font-size: 11.5px;
    color: var(--muted);
  }
  .acontent {
    margin: 6px 0 0;
    font-family: inherit;
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--muted);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .empty,
  .card-hint {
    font-size: 12.5px;
    color: var(--muted);
  }

  .stars {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .star {
    display: inline-flex;
    line-height: 1;
    color: var(--muted);
    padding: 2px;
    transition: color 0.12s;
  }
  .star.on {
    color: var(--warn);
  }
  .stars .muted {
    margin-left: 8px;
    font-size: 12px;
  }

  .license {
    margin: 0;
    padding: 12px 14px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
    font-size: 11.5px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--muted);
    max-height: 320px;
    overflow-y: auto;
  }

  .fb {
    resize: vertical;
    min-height: 84px;
    font-family: inherit;
    line-height: 1.6;
  }
  .fb-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .fb-foot .muted {
    font-size: 12px;
  }
</style>
