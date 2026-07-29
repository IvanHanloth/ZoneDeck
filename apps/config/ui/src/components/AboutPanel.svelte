<script>
  import IconMegaphone from "~icons/lucide/megaphone";
  import IconRefreshCw from "~icons/lucide/refresh-cw";
  import IconStar from "~icons/lucide/star";
  import IconHelp from "~icons/lucide/circle-help";
  import Card from "./Card.svelte";
  import Markdown from "./Markdown.svelte";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import {
    app,
    checkForUpdate,
    loadAnnouncements,
    toast,
  } from "../lib/state.svelte.js";
  import { MIT_LICENSE } from "../lib/license.js";
  import { feedbackOptions, formatTime, openExternal, submitFeedback } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const info = $derived(app.info);
  const v = $derived(app.config.verhub);
  const year = new Date().getFullYear();

  // 链接以 Verhub 项目信息为准（后端带缓存），拉不到时退回内置地址。
  const links = $derived(app.project);
  const homepageUrl = $derived(links?.website_url || "https://boss-key.ivan-hanloth.cn/");
  const repoUrl = $derived(links?.repo_url || info?.website || "https://github.com/IvanHanloth/Boss-Key");
  const docsUrl = $derived(links?.docs_url || "https://boss-key.ivan-hanloth.cn/guide/");
  const authorUrl = $derived(links?.author_homepage_url || info?.blog || "https://www.ivan-hanloth.cn/");
  const authorName = $derived(links?.author || info?.author || "Ivan Hanloth");

  let content = $state("");
  let rating = $state(0);
  let hoverRating = $state(0);
  let contact = $state("");
  let sending = $state(false);
  // 服务端说了算：项目未开放转换时不显示该选项。拉取失败按未开放处理。
  let forwardAvailable = $state(false);
  let forwardToGithub = $state(false);

  const litStars = $derived(hoverRating || rating);
  // 转成 Issue 后要靠 GitHub 账号跟进，缺了服务端也不受理。
  const forwardNeedsContact = $derived(forwardToGithub && !contact.trim());

  async function open(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("options.openLinkFailed", { err }), true);
    }
  }

  async function sendFeedback() {
    if (!content.trim()) return toast(t("about.writeSomething"), true);
    if (forwardNeedsContact) return toast(t("about.contactRequiredForIssue"), true);
    sending = true;
    const asIssue = forwardToGithub;
    try {
      await submitFeedback({
        content: content.trim(),
        rating: rating || null,
        contact: contact.trim(),
        forwardToGithub: asIssue,
      });
      content = "";
      rating = 0;
      forwardToGithub = false;
      toast(t(asIssue ? "about.issueThanks" : "about.feedbackThanks"));
    } catch (err) {
      toast(t("about.feedbackFailed", { err }), true);
    } finally {
      sending = false;
    }
  }

  // 进「关于」页拉取公告列表。
  $effect(() => {
    if (app.announcements.length === 0) loadAnnouncements();
  });

  // 进「关于」页问一次服务端有没有开放转换为 Issue；失败静默，选项不显示。
  $effect(() => {
    feedbackOptions()
      .then((o) => (forwardAvailable = !!o?.github_forward_available))
      .catch(() => (forwardAvailable = false));
  });

  const updateText = $derived.by(() => {
    if (app.updateChecking) return t("about.checking");
    if (!app.update) return "";
    if (!app.update.should_update) return t("about.upToDate");
    const found = t("about.updateFound", { version: app.update.target_version?.version ?? "" });
    return app.update.required ? found + t("about.updateForcedSuffix") : found;
  });

</script>

<div class="panel-stack">
  <div class="hero">
    <img class="logo" src="/icon.ico" alt="Boss Key" />
    <h2>{info?.name ?? "Boss Key"}</h2>
    <p class="muted">{t("about.version", { version: info?.version ?? "…" })}</p>
    <p>{t("about.tagline")}</p>
    <p>
      <button class="val link" onclick={() => open(homepageUrl)}>
        {t("about.homepage")}
      </button>
      <span> • </span>
      <button class="val link" onclick={() => open(repoUrl)}>
        {t("about.repository")}
      </button>
      <span> • </span>
      <button class="val link" onclick={() => open(docsUrl)}>
        {t("about.docs")}
      </button>

    </p>
    <p>Copyright © 2022-{year}
      <button class="val link" onclick={() => open(authorUrl)}>
        {authorName}
      </button> All Rights Reserved.</p>
  </div>

  <Card title={t("about.updateCard")}>
    <SettingRow label={t("about.checkUpdate")} description={t("about.checkUpdateDesc")}>
      {#snippet control()}
      <button class="btn" onclick={() => checkForUpdate(true)} disabled={app.updateChecking}>
        <IconRefreshCw width="14" height="14" /> {t("about.checkUpdate")}
      </button>
      <span class="muted result">{updateText}</span>
      {#if app.update?.should_update}
        <button class="btn primary" onclick={() => (app.updateOpen = true)}>{t("about.viewDetails")}</button>
      {/if}{/snippet}
    </SettingRow>

    <SettingRow label={t("about.includePreview")} description={t("about.includePreviewDesc")}>
      {#snippet control()}<Toggle bind:checked={v.include_preview} />{/snippet}
    </SettingRow>
  </Card>

  <Card title={t("about.announceCard")}>
    {#if app.announcements.length === 0}
      <p class="empty">{t("about.noAnnouncements")}</p>
    {:else}
      <ul class="anns">
        {#each app.announcements as a (a.id)}
          <li>
            <div class="ahead">
              <IconMegaphone width="13" height="13" />
              <span class="atitle">{a.title}</span>
              {#if a.is_pinned}<span class="pin">{t("about.pinned")}</span>{/if}
              <span class="adate">{formatTime(a.published_at)}</span>
            </div>
            <div class="acontent"><Markdown source={a.content} /></div>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>

  <Card title={t("about.feedbackCard")}>
    <div
      class="stars"
      role="radiogroup"
      tabindex="-1"
      aria-label={t("about.ratingAria")}
      onpointerleave={() => (hoverRating = 0)}
    >
      {#each [1, 2, 3, 4, 5] as n (n)}
        <button
          type="button"
          class="star"
          class:on={litStars >= n}
          role="radio"
          aria-checked={rating === n}
          aria-label={t("about.starAria", { n })}
          onpointerenter={() => (hoverRating = n)}
          onfocus={() => (hoverRating = n)}
          onblur={() => (hoverRating = 0)}
          onclick={() => (rating = rating === n ? 0 : n)}
        >
          <IconStar width="20" height="20" fill={litStars >= n ? "currentColor" : "none"} />
        </button>
      {/each}
      <span class="muted">{rating ? t("about.stars", { n: rating }) : t("about.ratingOptional")}</span>
    </div>

    <textarea
      class="fb"
      rows="5"
      maxlength="4000"
      placeholder={t("about.feedbackPlaceholder")}
      bind:value={content}
    ></textarea>
    <input
      type="text"
      maxlength="120"
      class:required={forwardNeedsContact}
      placeholder={t(forwardToGithub ? "about.contactPlaceholderGithub" : "about.contactPlaceholder")}
      bind:value={contact}
    />

    <p class="card-hint">{t("about.contactNotice")}</p>

    {#if forwardAvailable}
      <label class="fb-issue" title={t("about.forwardToIssueDesc")}>
        <input type="checkbox" bind:checked={forwardToGithub} />
        <span class="fb-issue-label">{t("about.forwardToIssue")}</span>
        <IconHelp width="13" height="13" />
      </label>
    {/if}

    <div class="fb-foot">
      <span class="muted">{content.length} / 4000</span>
      <button
        class="btn primary"
        onclick={sendFeedback}
        disabled={sending || !content.trim() || forwardNeedsContact}
      >
        {t(sending ? "about.submitting" : forwardToGithub ? "about.submitAsIssue" : "about.submitFeedback")}
      </button>
    </div>
  </Card>

  <Card title={t("about.licenseCard")}>
    <p class="card-hint">{t("about.licenseHint")}</p>
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
    margin-top: 6px;
    font-size: 12.5px;
    color: var(--muted);
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
    min-height: 112px;
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

  input.required {
    border-color: var(--warn);
  }

  .fb-issue {
    display: flex;
    align-items: center;
    gap: 7px;
    align-self: flex-start;
    cursor: help;
    font-size: 12.5px;
  }
  .fb-issue input {
    flex: none;
    cursor: pointer;
  }
  .fb-issue-label {
    font-weight: 600;
  }
  /* 说明走原生 title 气泡，这里只留一个可发现性提示。 */
  .fb-issue :global(svg) {
    flex: none;
    color: var(--muted);
  }
</style>
