<script>
  import { t } from "../lib/i18n.svelte.js";
  import WindowList from "./WindowList.svelte";
  import WhitelistList from "./WhitelistList.svelte";
  import {
    addWhitelistRules,
    applyListFilters,
    hasProcessIdentity,
    newWhitelistRegexRule,
  } from "../lib/grouping.js";
  import { app, refreshWindows, toast } from "../lib/state.svelte.js";

  let selectedAvail = $state([]);

  const shownAvailable = $derived(
    applyListFilters(app.available, {
      showBackground: app.showBackground,
      showUntitled: app.showUntitled,
      search: app.search,
    }),
  );

  function pickedWindows() {
    return app.available.filter((w) => selectedAvail.includes(w.hwnd));
  }

  function addProcesses() {
    const picked = pickedWindows();
    if (picked.length === 0) return toast(t("whitelist.pickFirst"), true);
    const unknown = picked.filter((w) => !hasProcessIdentity(w)).length;
    app.config.whitelist = addWhitelistRules(app.config.whitelist, picked);
    selectedAvail = [];
    if (unknown) toast(t("binding.processUnidentified", { count: unknown }), true);
  }

  function addRegex() {
    const seed = pickedWindows()[0];
    app.config.whitelist = [...app.config.whitelist, newWhitelistRegexRule(seed?.process)];
    selectedAvail = [];
  }

  async function refresh() {
    await refreshWindows();
    selectedAvail = [];
    toast(t("binding.windowsRefreshed"));
  }
</script>

<div class="whitelist">
  <div class="grid">
    <div class="avail">
      <WindowList
        title={t("binding.availableWindows")}
        windows={shownAvailable}
        bind:selected={selectedAvail}
        bind:search={app.search}
        bind:showBackground={app.showBackground}
        bind:showUntitled={app.showUntitled}
        onrefresh={refresh}
      />
    </div>

    <div class="rules">
      <WhitelistList
        bind:rules={app.config.whitelist}
        onadd={addProcesses}
        onaddregex={addRegex}
      />
    </div>
  </div>
  <p class="intro">{t("whitelist.intro")}</p>
</div>

<style>
  /* 容器查询而非媒体查询：左侧导航会吃掉窗口宽度，按整窗算断点会切错 */
  .whitelist {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 0;
  }
  .intro {
    flex: none;
    font-size: 12px;
    line-height: 18px;
    color: var(--text-2);
  }
  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    align-items: stretch;
  }

  .avail {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    min-width: 0;
  }
  .rules {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    min-width: 0;
  }

  @container (max-width: 520px) {
    .grid {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr 1fr;
    }
  }
</style>
