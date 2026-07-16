<script>
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, openRestoreTool, restartCore, startCore, setAutostart, refreshPssuspend, toast } from "../lib/state.svelte.js";
  import { openExternal } from "../lib/verhub.js";

  let elevating = $state(false);

  async function openLink(url) {
    try {
      await openExternal(url);
    } catch (err) {
      toast("打开链接失败：" + err, true);
    }
  }

  async function elevate() {
    elevating = true;
    try {
      if (app.status.running) await restartCore(true);
      else await startCore(true);
    } finally {
      elevating = false;
    }
  }

  const s = $derived(app.config.setting);

  // 自启注册方式的中文标注；仅在已开启自启时显示。
  const autostartMethodText = $derived(
    !app.autostart
      ? ""
      : app.autostartMethod === "task"
        ? "计划任务"
        : app.autostartMethod === "registry"
          ? "注册表"
          : "",
  );

  // 权限开关变更：绑定已更新 s.autostart_admin（自动保存）；若自启已开，按新权限重注册。
  async function onAutostartAdminChange(admin) {
    if (app.autostart) await setAutostart(true, admin);
  }

  // 「以管理员身份自启」的前置条件：必须先开启开机自启。
  const autostartOff = $derived(!app.autostart);

  // 未开启「隐藏窗口时冻结进程」时，冻结区下方的子选项一律置灰禁用。
  const freezeOff = $derived(!s.freeze_after_hide);

  // 增强冻结的前置条件；任一不满足即置灰。
  const enhancedBlocked = $derived.by(() => {
    const reasons = [];
    if (!app.status.running) reasons.push("核心未运行");
    else if (!app.status.elevated) reasons.push("核心需以管理员身份运行");
    if (!app.pssuspend) reasons.push("程序目录缺少 pssuspend64.exe");
    return reasons;
  });
  const enhancedDisabled = $derived(enhancedBlocked.length > 0);
</script>

<div class="panel-stack">
  <section class="fcard">
    <h3>常规</h3>
    <SettingRow label="隐藏窗口后静音" description="隐藏时静音目标进程的音频，恢复显示时自动取消静音。">
      {#snippet control()}<Toggle bind:checked={s.mute_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow label="同时隐藏当前活动窗口" description="按下热键时，除已绑定窗口外，同时隐藏当前正在使用的前台窗口。">
      {#snippet control()}<Toggle bind:checked={s.hide_current} />{/snippet}
    </SettingRow>
    <SettingRow label="单击托盘图标切换隐藏" description="左键单击托盘图标即可隐藏 / 显示，无需按热键。">
      {#snippet control()}<Toggle bind:checked={s.click_to_hide} />{/snippet}
    </SettingRow>
    <SettingRow label="隐藏后同时隐藏托盘图标" description="隐藏窗口时连托盘图标一起藏起，更隐蔽；再次触发热键可恢复。">
      {#snippet control()}<Toggle bind:checked={s.hide_icon_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow label="隐藏前发送暂停键（Beta）" description="隐藏前先发送媒体暂停键（暂停正在播放的视频 / 音乐），会带来约 0.2 秒延迟。">
      {#snippet control()}<Toggle bind:checked={s.send_before_hide} />{/snippet}
    </SettingRow>
    <!-- <SettingRow label="显示悬浮窗" description="在桌面显示一个可拖动的悬浮小窗，双击可快速切换隐藏（核心侧功能开发中）。">
      {#snippet control()}<Toggle bind:checked={s.show_float_window} />{/snippet}
    </SettingRow> -->
  </section>

  <section class="fcard">
    <h3>进程冻结</h3>
    <SettingRow label="隐藏窗口时冻结进程（Beta）" description="隐藏后挂起目标进程，降低其 CPU / 内存占用；恢复显示时自动解冻。">
      {#snippet control()}<Toggle bind:checked={s.freeze_after_hide} />{/snippet}
    </SettingRow>
    <SettingRow
      label="使用增强冻结"
      disabled={freezeOff || enhancedDisabled}
      description={!freezeOff && enhancedDisabled
        ? `当前不可用：${enhancedBlocked.join("；")}。`
        : "改用 pssuspend64.exe 冻结。需在程序目录放置该文件，且核心以管理员身份运行。"}
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.enhanced_freeze}
          disabled={freezeOff || enhancedDisabled}
          title={freezeOff
            ? "需先开启「隐藏窗口时冻结进程」"
            : enhancedDisabled
              ? enhancedBlocked.join("；")
              : ""}
        />
      {/snippet}
    </SettingRow>
    <SettingRow
      label="冻结完整进程（Beta）"
      disabled={freezeOff}
      description="递归冻结命中程序的整棵子进程树，冻结更彻底；对普通与增强冻结均生效。可能影响这些子进程的后台任务"
    >
      {#snippet control()}<Toggle bind:checked={s.freeze_whole_tree} disabled={freezeOff} />{/snippet}
    </SettingRow>
    <div class="note" class:disabled={freezeOff}>
      增强冻结需下载
      <button class="link" onclick={() => openLink("https://download.sysinternals.com/files/PSTools.zip")} disabled={freezeOff}>PSTools</button>
      并将 pssuspend64.exe 放入程序目录，且核心以管理员身份运行。
      <button class="link" onclick={refreshPssuspend} disabled={freezeOff}>重新检测</button>
    </div>
  </section>

  <section class="fcard">
    <h3>启动与权限</h3>
    <SettingRow label="开机自启" description="随系统登录自动启动核心服务">
      {#snippet control()}
        <div class="autostart-ctl">
          {#if autostartMethodText}
            <span class="method">当前方式：{autostartMethodText}</span>
          {/if}
          <Toggle bind:checked={app.autostart} onchange={(e) => setAutostart(e.target.checked, s.autostart_admin)} />
        </div>
      {/snippet}
    </SettingRow>
    <SettingRow
      label="以管理员身份自启"
      disabled={autostartOff}
      description="以最高权限的计划任务注册自启，供开机即用增强冻结等需管理员的功能；关闭则以普通权限自启。仅计划任务方式下生效。"
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.autostart_admin}
          disabled={autostartOff}
          title={autostartOff ? "需先开启「开机自启」" : ""}
          onchange={(e) => onAutostartAdminChange(e.target.checked)}
        />
      {/snippet}
    </SettingRow>
    <SettingRow
      label="核心运行权限"
      description="增强冻结与「计划任务最高权限自启」需要核心以管理员身份运行。"
    >
      {#snippet control()}
        <div class="perm-ctl">
          <strong>
            {app.status.running === null
              ? "检测中…"
              : !app.status.running
                ? "核心未运行"
                : app.status.elevated
                  ? "管理员"
                  : "普通用户"}
          </strong>
          <button
            class="btn"
            onclick={elevate}
            disabled={elevating || (app.status.running === true && app.status.elevated)}
          >
            {app.status.running ? "以管理员身份重启" : "以管理员身份启动"}
          </button>
        </div>
      {/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3>日志</h3>
    <SettingRow
      label="日志保留天数"
      description="核心日志按天保存在程序目录的 logs 文件夹内，启动时自动清理更早的日志；选择「关闭」则不记录日志。"
    >
      {#snippet control()}
        <select class="sel" bind:value={s.log_retention_days}>
          <option value={0}>关闭</option>
          <option value={3}>3 天</option>
          <option value={7}>7 天</option>
          <option value={14}>14 天</option>
          <option value={30}>30 天</option>
        </select>
      {/snippet}
    </SettingRow>
  </section>

  <section class="fcard">
    <h3>工具</h3>
    <SettingRow
      label="窗口恢复工具"
      description="找回被误隐藏、无法通过热键恢复的窗口：列出所有窗口，勾选后恢复显示。"
    >
      {#snippet control()}
        <button class="btn" onclick={openRestoreTool}>打开</button>
      {/snippet}
    </SettingRow>
  </section>
</div>

<style>
  .note {
    padding: 10px 14px;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.6;
    border-top: 1px solid var(--border);
  }
  .note.disabled {
    opacity: 0.45;
  }
  .note .link {
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    cursor: pointer;
  }
  .note .link:hover {
    text-decoration: underline;
  }
  .note .link:disabled {
    color: var(--muted);
    cursor: not-allowed;
    text-decoration: none;
  }
  .perm-ctl {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .autostart-ctl {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .autostart-ctl .method {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .sel {
    padding: 5px 10px;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 13px;
  }
  .sel:focus {
    outline: none;
    border-color: var(--accent);
  }
</style>
