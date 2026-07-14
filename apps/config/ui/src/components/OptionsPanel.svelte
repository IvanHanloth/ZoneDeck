<script>
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, openRestoreTool, restartCore, startCore, setAutostart } from "../lib/state.svelte.js";

  let elevating = $state(false);

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

  // 增强冻结要求：核心以管理员身份运行 + 程序目录存在 pssuspend64.exe。任一不满足就置灰。
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
      description={enhancedDisabled
        ? `当前不可用：${enhancedBlocked.join("；")}。`
        : "改用 pssuspend64.exe 冻结。需在程序目录放置该文件，且核心以管理员身份运行。"}
    >
      {#snippet control()}
        <Toggle
          bind:checked={s.enhanced_freeze}
          disabled={enhancedDisabled}
          title={enhancedDisabled ? enhancedBlocked.join("；") : ""}
        />
      {/snippet}
    </SettingRow>
    <div class="note">
      增强冻结需下载
      <a href="https://download.sysinternals.com/files/PSTools.zip" target="_blank" rel="noreferrer">PSTools</a>
      并将 pssuspend64.exe 放入程序目录，且核心以管理员身份运行。
    </div>
  </section>

  <section class="fcard">
    <h3>启动与权限</h3>
    <SettingRow label="开机自启" description="随系统登录自动启动核心服务">
      {#snippet control()}
        <Toggle bind:checked={app.autostart} onchange={(e) => setAutostart(e.target.checked)} />
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
  .perm-ctl {
    display: flex;
    align-items: center;
    gap: 12px;
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
