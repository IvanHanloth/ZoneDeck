<script>
  // 顶部设置搜索：输入即列结果，选中后切到对应页并高亮那一条。
  import { tick } from "svelte";
  import IconSearch from "~icons/lucide/search";
  import IconX from "~icons/lucide/x";
  import Flyout from "./fluent/Flyout.svelte";
  import { search } from "../lib/search.js";
  import { app } from "../lib/state.svelte.js";
  import { t } from "../lib/i18n.svelte.js";

  let query = $state("");
  let anchor = $state(null);
  let input = $state(null);
  let active = $state(0);
  // 交给 Flyout 双向持有：它自己负责点外面、按 Esc、滚动时收起。
  let open = $state(false);

  const hits = $derived(search(query));

  // 有输入就展开（没结果也开着，好把「无匹配」说出来），清空则收起。
  // 换关键字后高亮回到第一条。
  $effect(() => {
    open = query.trim().length > 0;
    active = 0;
  });

  /** 高亮停留时长，与 app.css 的 search-hit 动画一致。 */
  const HIT_MS = 1600;
  /** 换页后等正文浮入到看得见再定位，滚动量才不是按动画中途的位置算的。 */
  const SETTLE_MS = 160;
  /** 展开器撑开的时长，同 --dur-slow。 */
  const EXPAND_MS = 350;

  let hitEl = null;
  let hitTimer = null;

  const wait = (ms) => new Promise((r) => setTimeout(r, ms));

  // 高亮走内联 animation 而不是加 class：卡片的 class 属性由 Svelte 托管，
  // 它任何一次重渲染都会整条重写，外部加上去的类会被无声地抹掉。
  function clearFlash(el) {
    if (!el) return;
    el.style.animation = "";
    el.style.borderRadius = "";
  }

  function flash(el) {
    clearTimeout(hitTimer);
    clearFlash(hitEl);
    // 断掉再重排，连着跳到同一元素时动画才会重播。
    el.style.animation = "none";
    void el.offsetWidth;
    el.style.borderRadius = "var(--r-card)";
    el.style.animation = `search-hit ${HIT_MS}ms var(--ease-standard)`;
    hitEl = el;
    hitTimer = setTimeout(() => {
      clearFlash(el);
      hitEl = null;
    }, HIT_MS);
  }

  async function go(hit) {
    query = "";
    input?.blur();
    const switched = app.tab !== hit.tab;
    app.tab = hit.tab;
    if (hit.kind === "page") return;

    // 等页面换完再找锚点；标题即锚点，不用逐项登记 id。
    await tick();
    if (switched) await wait(SETTLE_MS);

    const attr = hit.kind === "group" ? "data-group" : "data-setting";
    const el = [...document.querySelectorAll(`[${attr}]`)].find(
      (n) => n.getAttribute(attr) === hit.label,
    );
    if (!el) return;

    // 目标落在收起的展开器里时先撑开：收起态的子项高度为零，滚过去也看不见。
    const holder = el.closest(".expander");
    if (holder && !holder.classList.contains("open")) {
      holder.querySelector(".hit")?.click();
      await wait(EXPAND_MS);
    }

    el.scrollIntoView({ block: "center", behavior: "smooth" });
    flash(el);
  }

  function onKey(e) {
    if (e.key === "Escape") {
      query = "";
      input?.blur();
      return;
    }
    if (!open || !hits.length) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      active = (active + 1) % hits.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = (active - 1 + hits.length) % hits.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      go(hits[active]);
    }
  }
</script>

<!-- 标题栏整条是拖拽区且双击最大化，搜索框里的双击选词不能捅到上面去 -->
<div
  class="box elev"
  class:open
  bind:this={anchor}
  ondblclick={(e) => e.stopPropagation()}
  role="presentation"
>
  <IconSearch width="14" height="14" />
  <input
    bind:this={input}
    bind:value={query}
    type="text"
    spellcheck="false"
    placeholder={t("search.placeholder")}
    aria-label={t("search.placeholder")}
    onfocus={() => (open = query.trim().length > 0)}
    onkeydown={onKey}
  />
  {#if query}
    <button
      class="x"
      title={t("common.clear")}
      aria-label={t("common.clear")}
      onclick={() => {
        query = "";
        input?.focus();
      }}
    >
      <IconX width="12" height="12" />
    </button>
  {/if}
</div>

<Flyout
  bind:open
  {anchor}
  matchWidth
  role="listbox"
  ariaLabel={t("search.resultsAria")}
>
  {#if hits.length}
    {#each hits as hit, i (hit.labelKey)}
      <button
        class="res"
        class:active={i === active}
        role="option"
        aria-selected={i === active}
        onpointerenter={() => (active = i)}
        onclick={() => go(hit)}
      >
        <span class="res-text">
          <span class="res-label">{hit.label}</span>
          {#if hit.desc}<span class="res-desc">{hit.desc}</span>{/if}
        </span>
        {#if hit.kind !== "page"}<span class="res-page">{hit.page}</span>{/if}
      </button>
    {/each}
  {:else}
    <p class="empty hint">{t("search.noResults")}</p>
  {/if}
</Flyout>

<style>
  .box {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    padding: 0 8px 0 10px;
    border-radius: var(--r-control);
    background: var(--control);
    color: var(--text-3);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .box:hover {
    background: var(--control-hover);
  }
  .box:focus-within {
    background: var(--control-focus);
    color: var(--text-2);
  }
  /* 结果展开时压平下缘，和弹层连成一体 */
  .box.open {
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
  }

  input {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: 13px;
    color: var(--text);
    background: none;
    border: none;
    user-select: text;
    cursor: text;
  }
  input:focus {
    outline: none;
  }
  input::placeholder {
    color: var(--text-3);
  }

  .x {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: var(--r-control);
    color: var(--text-2);
    transition: background var(--dur-fast) var(--ease-standard);
  }
  .x:hover {
    background: var(--subtle-hover);
    color: var(--text);
  }
  .x:active {
    background: var(--subtle-pressed);
  }

  .res {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 7px 10px;
    border-radius: var(--r-control);
    text-align: start;
  }
  .res.active {
    background: var(--subtle-hover);
  }
  .res:active {
    background: var(--subtle-pressed);
  }

  .res-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .res-label,
  .res-desc,
  .res-page {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .res-desc {
    font-size: 12px;
    line-height: 16px;
    color: var(--text-2);
  }
  .res-page {
    flex: none;
    max-width: 40%;
    font-size: 12px;
    color: var(--text-3);
  }

  .empty {
    padding: 10px;
    text-align: center;
  }
</style>
