<script>
  import { t } from "../lib/i18n.svelte.js";
  import WindowList from "./WindowList.svelte";
  import WindowRuleList from "./WindowRuleList.svelte";
  import ProcessRuleList from "./ProcessRuleList.svelte";
  import {
    addProcessRules,
    addWindowRules,
    applyListFilters,
    newProcessRegexRule,
    newWindowRegexRule,
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

  function addWindows() {
    const picked = pickedWindows();
    if (picked.length === 0) return toast(t("binding.pickWindowsFirst"), true);
    app.config.window_rules = addWindowRules(app.config.window_rules, picked);
    selectedAvail = [];
  }

  function addProcesses() {
    const picked = pickedWindows();
    if (picked.length === 0) return toast(t("binding.pickProcessesFirst"), true);
    app.config.process_rules = addProcessRules(app.config.process_rules, picked);
    selectedAvail = [];
  }

  // 新正则规则预填一条「包含式」正则，种子取自选中窗口。
  function addWindowRegex() {
    const seed = pickedWindows()[0];
    app.config.window_rules = [
      ...app.config.window_rules,
      newWindowRegexRule(seed?.title),
    ];
    selectedAvail = [];
  }

  function addProcessRegex() {
    const seed = pickedWindows()[0];
    app.config.process_rules = [
      ...app.config.process_rules,
      newProcessRegexRule(seed?.process),
    ];
    selectedAvail = [];
  }

  async function refresh() {
    await refreshWindows();
    selectedAvail = [];
    toast(t("binding.windowsRefreshed"));
  }
</script>

<div class="binding">
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
      <WindowRuleList
        bind:rules={app.config.window_rules}
        onadd={addWindows}
        onaddregex={addWindowRegex}
      />
      <ProcessRuleList
        bind:rules={app.config.process_rules}
        onadd={addProcesses}
        onaddregex={addProcessRegex}
      />
    </div>
  </div>
</div>

<style>
  .binding {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 0;
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

  @media (max-width: 720px) {
    .grid {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr 1fr;
    }
  }
</style>
