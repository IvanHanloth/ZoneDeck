<script>
  import IconGlobe from "~icons/lucide/globe";
  import IconMail from "~icons/lucide/mail";
  import IconGithub from "~icons/lucide/github";
  import IconScale from "~icons/lucide/scale";
  import IconMegaphone from "~icons/lucide/megaphone";
  import IconRefreshCw from "~icons/lucide/refresh-cw";
  import IconStar from "~icons/lucide/star";
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
  import { formatTime, openExternal, submitFeedback } from "../lib/verhub.js";
  import { t } from "../lib/i18n.svelte.js";

  const info = $derived(app.info);
  const v = $derived(app.config.verhub);
  const year = new Date().getFullYear();

  let content = $state("");
  let rating = $state(0);
  let hoverRating = $state(0);
  let contact = $state("");
  let sending = $state(false);

  const litStars = $derived(hoverRating || rating);

  async function open(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast(t("options.openLinkFailed", { err }), true);
    }
  }

  async function sendFeedback() {
    if (!content.trim()) return toast(t("about.writeSomething"), true);
    sending = true;
    try {
      await submitFeedback({
        content: content.trim(),
        rating: rating || null,
        contact: contact.trim(),
      });
      content = "";
      rating = 0;
      toast(t("about.feedbackThanks"));
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
      <button class="val link" onclick={() => open(info?.website ?? "https://boss-key.ivan-hanloth.cn/")}>
        {t("about.homepage")}
      </button>
      <span> • </span>
      <button class="val link" onclick={() => open("https://github.com/IvanHanloth/Boss-Key")}>
        {t("about.repository")}
      </button>
      <span> • </span>
      <button class="val link" onclick={() => open("https://boss-key.ivan-hanloth.cn/guide/")}>
        {t("about.docs")}
      </button>
      
    </p>
    <p>Copyright © 2022-{year}
      <button class="val link" onclick={() => open(info?.blog ?? "https://www.ivan-hanloth.cn/")}>
        Ivan Hanloth
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
      placeholder={t("about.contactPlaceholder")}
      bind:value={contact}
    />

    <div class="fb-foot">
      <span class="muted">{content.length} / 4000</span>
      <button class="btn primary" onclick={sendFeedback} disabled={sending || !content.trim()}>
        {t(sending ? "about.submitting" : "about.submitFeedback")}
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
</style>
