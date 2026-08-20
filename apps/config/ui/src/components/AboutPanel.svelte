<script>
  import IconMegaphone from "~icons/lucide/megaphone";
  import IconRefreshCw from "~icons/lucide/refresh-cw";
  import IconStar from "~icons/lucide/star";
  import IconHelp from "~icons/lucide/circle-help";
  import IconFlask from "~icons/lucide/flask-conical";
  import SettingsGroup from "./fluent/SettingsGroup.svelte";
  import SettingsCard from "./fluent/SettingsCard.svelte";
  import ToggleSwitch from "./fluent/ToggleSwitch.svelte";
  import CheckBox from "./fluent/CheckBox.svelte";
  import TextBox from "./fluent/TextBox.svelte";
  import ContentDialog from "./fluent/ContentDialog.svelte";
  import Markdown from "./Markdown.svelte";
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

  // 链接以 Verhub 项目信息为准，拉不到时退回内置地址。
  const links = $derived(app.project);
  const homepageUrl = $derived(links?.website_url || "https://zonedeck.ivan-hanloth.cn/");
  const repoUrl = $derived(links?.repo_url || info?.website || "https://github.com/IvanHanloth/ZoneDeck");
  const docsUrl = $derived(links?.docs_url || "https://zonedeck.ivan-hanloth.cn/guide/");
  const authorUrl = $derived(links?.author_homepage_url || info?.blog || "https://www.ivan-hanloth.cn/");
  const authorName = $derived(links?.author || info?.author || "Ivan Hanloth");

  let content = $state("");
  let rating = $state(0);
  let hoverRating = $state(0);
  let contact = $state("");
  let sending = $state(false);
  // 项目未开放转换时不显示该选项；拉取失败按未开放处理。
  let forwardAvailable = $state(false);
  let forwardToGithub = $state(false);
  // 勾选转 Issue 时弹一次使用须知：转出去的内容是公开的，且占用问题列表。
  let issueGuideOpen = $state(false);

  const litStars = $derived(hoverRating || rating);
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

  // 问一次服务端有没有开放转换为 Issue；失败静默。
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

<!-- 不挂 .pad：hero 的内边距比通用卡片大一档，而 .surface.pad 和
     scoped 的 .hero 特异性相同、排在后面，会把它盖掉 -->
<div class="hero surface">
  <img class="logo" src="/logo.svg" alt="ZoneDeck" />
  <div class="meta">
    <h2 class="type-subtitle">{info?.name ?? "ZoneDeck"}</h2>
    <p class="muted">{t("about.version", { version: info?.version ?? "…" })}</p>
    <p>{t("about.tagline")}</p>
    <p class="links">
      <button class="link" onclick={() => open(homepageUrl)}>{t("about.homepage")}</button>
      <span class="sep">·</span>
      <button class="link" onclick={() => open(repoUrl)}>{t("about.repository")}</button>
      <span class="sep">·</span>
      <button class="link" onclick={() => open(docsUrl)}>{t("about.docs")}</button>
    </p>
    <p class="copyright">
      Copyright © 2022-{year}
      <button class="link" onclick={() => open(authorUrl)}>{authorName}</button>
      All Rights Reserved.
    </p>
  </div>
</div>

<SettingsGroup title={t("about.updateCard")}>
  <SettingsCard icon={IconRefreshCw} label={t("about.checkUpdate")} description={t("about.checkUpdateDesc")}>
    {#snippet control()}
      <span class="muted result">{updateText}</span>
      <button class="btn" onclick={() => checkForUpdate(true)} disabled={app.updateChecking}>
        <IconRefreshCw width="14" height="14" />
        {t("about.checkUpdate")}
      </button>
      {#if app.update?.should_update}
        <button class="btn primary" onclick={() => (app.updateOpen = true)}>
          {t("about.viewDetails")}
        </button>
      {/if}
    {/snippet}
  </SettingsCard>

  <SettingsCard icon={IconFlask} label={t("about.includePreview")} description={t("about.includePreviewDesc")}>
    {#snippet control()}<ToggleSwitch bind:checked={v.include_preview} />{/snippet}
  </SettingsCard>
</SettingsGroup>

<SettingsGroup title={t("about.announceCard")}>
  <div class="surface pad">
    {#if app.announcements.length === 0}
      <p class="hint">{t("about.noAnnouncements")}</p>
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
  </div>
</SettingsGroup>

<SettingsGroup title={t("about.feedbackCard")}>
  <div class="surface pad fb-card">
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
      <span class="muted rating-hint">
        {rating ? t("about.stars", { n: rating }) : t("about.ratingOptional")}
      </span>
    </div>

    <textarea
      class="fb elev"
      rows="5"
      maxlength="4000"
      placeholder={t("about.feedbackPlaceholder")}
      bind:value={content}
    ></textarea>

    <TextBox
      maxlength={120}
      invalid={forwardNeedsContact}
      placeholder={t(forwardToGithub ? "about.contactPlaceholderGithub" : "about.contactPlaceholder")}
      bind:value={contact}
    />

    <p class="hint">{t("about.contactNotice")}</p>

    {#if forwardAvailable}
      <span class="fb-issue" title={t("about.forwardToIssueDesc")}>
        <CheckBox
          bind:checked={forwardToGithub}
          label={t("about.forwardToIssue")}
          onchange={(e) => e.currentTarget.checked && (issueGuideOpen = true)}
        />
        <span class="help"><IconHelp width="13" height="13" /></span>
      </span>
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
  </div>
</SettingsGroup>

<SettingsGroup title={t("about.licenseCard")}>
  <div class="surface pad">
    <p class="hint">{t("about.licenseHint")}</p>
    <pre class="license">{MIT_LICENSE}</pre>
  </div>
</SettingsGroup>

<ContentDialog bind:open={issueGuideOpen} title={t("about.issueGuideTitle")}>
  <ul class="guide">
    <li>{t("about.issueGuideScope")}</li>
    <li>{t("about.issueGuideBot")}</li>
    <li>{t("about.issueGuideContact")}</li>
    <li>{t("about.issueGuidePrivacy")}</li>
  </ul>
  <p class="hint guide-why">{t("about.issueGuideWhy")}</p>

  {#snippet footer()}
    <button class="btn primary" type="button" onclick={() => (issueGuideOpen = false)}>
      {t("about.issueGuideAgree")}
    </button>
  {/snippet}
</ContentDialog>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 28px;
    padding: 24px;
  }
  .hero .logo {
    width: 64px;
    height: 64px;
    flex: none;
  }
  .meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .meta p {
    font-size: 12px;
    line-height: 18px;
  }
  .meta .muted,
  .copyright {
    color: var(--text-2);
  }
  .links {
    margin-top: 4px;
  }
  .sep {
    color: var(--text-3);
    margin: 0 4px;
  }

  .link {
    color: var(--accent);
    font: inherit;
    padding: 0;
  }
  .link:hover {
    text-decoration: underline;
  }

  .result {
    font-size: 12px;
  }

  .anns {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 300px;
    overflow-y: auto;
  }
  .anns li {
    padding: 10px 12px;
    background: var(--card-2);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
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
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 600;
  }
  .adate {
    margin-left: auto;
    flex: none;
    font-size: 11px;
    color: var(--text-2);
  }
  .acontent {
    margin-top: 6px;
    font-size: 12px;
    color: var(--text-2);
  }

  .fb-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .stars {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .star {
    display: inline-flex;
    line-height: 1;
    color: var(--text-3);
    padding: 2px;
    border-radius: var(--r-control);
    transition: color var(--dur-fast) var(--ease-standard);
  }
  .star:hover {
    color: var(--warn);
  }
  .star.on {
    color: var(--warn);
  }
  .rating-hint {
    margin-left: 8px;
    font-size: 12px;
  }

  .fb {
    resize: vertical;
    min-height: 112px;
    padding: 8px 10px;
    border-radius: var(--r-control);
    background: var(--control);
    color: var(--text);
    font: inherit;
    line-height: 20px;
    user-select: text;
  }
  .fb:focus {
    outline: none;
    background: var(--control-focus);
    border-bottom-color: var(--accent);
  }
  .fb::placeholder {
    color: var(--text-3);
  }

  .fb-issue {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    align-self: flex-start;
    cursor: help;
  }
  /* 只染这一个帮助图标 —— 早先写成 `.fb-issue :global(svg)`，
     把 CheckBox 内部的勾也一起染成了灰色，未选中时勾就露了出来 */
  .fb-issue .help {
    display: inline-flex;
    flex: none;
    color: var(--text-2);
  }

  .guide {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    line-height: 20px;
  }
  .guide li::marker {
    color: var(--text-3);
  }
  .guide-why {
    margin-top: 16px;
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

  .license {
    margin: 12px 0 0;
    padding: 12px 14px;
    background: var(--card-2);
    border: 1px solid var(--stroke);
    border-radius: var(--r-card);
    font-family: var(--font-mono);
    font-size: 11.5px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-2);
    max-height: 320px;
    overflow-y: auto;
    user-select: text;
  }
</style>
