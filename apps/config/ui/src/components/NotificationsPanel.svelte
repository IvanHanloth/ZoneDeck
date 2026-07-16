<script>
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";
  import IconBell from "~icons/lucide/bell";
  import { app } from "../lib/state.svelte.js";

  const n = $derived(app.config.notifications);
</script>

<div class="panel-stack">
  <section class="fcard">
    <h3><IconBell width="16" height="16" /> 托盘通知</h3>
    <SettingRow label="启动运行时通知" description="核心启动并开始服务时弹出通知">
      {#snippet control()}<Toggle bind:checked={n.on_start} />{/snippet}
    </SettingRow>
    <SettingRow label="退出时通知" description="核心停止服务时弹出通知">
      {#snippet control()}<Toggle bind:checked={n.on_quit} />{/snippet}
    </SettingRow>
    <SettingRow label="开机自启变更时通知" description="在托盘菜单开启 / 关闭开机自启时弹出通知">
      {#snippet control()}<Toggle bind:checked={n.on_autostart} />{/snippet}
    </SettingRow>
    <SettingRow label="隐藏窗口时通知" description="每次触发隐藏时弹出通知">
      {#snippet control()}<Toggle bind:checked={n.on_hide} />{/snippet}
    </SettingRow>
    <SettingRow label="显示窗口时通知" description="每次恢复显示时弹出通知">
      {#snippet control()}<Toggle bind:checked={n.on_show} />{/snippet}
    </SettingRow>
  </section>
</div>

<style>
  .fcard h3 {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
