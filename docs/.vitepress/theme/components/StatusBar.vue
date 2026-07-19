<script setup>
// 忠实复刻配置界面底部状态栏（apps/config/ui/src/components/StatusBar.svelte）。
// 图标直接取自 Lucide（与应用同源），因此显示效果与实际界面完全一致。
// 文案跟随文档站语言，与应用内 locales/ 下的同名文案保持一致。
import { computed } from 'vue'
import { useData } from 'vitepress'
import { Shield, Play, RotateCw, Power, ScrollText, Check } from 'lucide-vue-next'

const props = defineProps({
  // offline：核心未运行 / user：普通用户运行 / admin：管理员运行
  variant: { type: String, default: 'user' },
})

const { lang } = useData()

const TEXT = {
  'zh-CN': {
    coreRunning: '核心运行中', coreStopped: '核心未运行', startCore: '启动核心',
    startAdmin: '管理员启动', restartAdmin: '管理员身份重启', restartCore: '重启核心',
    quitCore: '退出核心', openLogDir: '打开日志目录', theme: '主题：跟随系统',
    monitorOnTitle: '核心正在监听热键与鼠标触发', monitorOn: '热键生效', saved: '已保存',
    example: '状态栏示例',
  },
  en: {
    coreRunning: 'Core running', coreStopped: 'Core stopped', startCore: 'Start core',
    startAdmin: 'Start as administrator', restartAdmin: 'Restart as administrator',
    restartCore: 'Restart core', quitCore: 'Stop core', openLogDir: 'Open log folder',
    theme: 'Theme: follow system',
    monitorOnTitle: 'The core is listening for hotkey and mouse triggers',
    monitorOn: 'Hotkeys active', saved: 'Saved', example: 'Status bar example',
  },
  'zh-TW': {
    coreRunning: '核心執行中', coreStopped: '核心未執行', startCore: '啟動核心',
    startAdmin: '以系統管理員啟動', restartAdmin: '以系統管理員身分重新啟動',
    restartCore: '重新啟動核心', quitCore: '結束核心', openLogDir: '開啟記錄檔資料夾',
    theme: '佈景主題：跟隨系統', monitorOnTitle: '核心正在監聽快速鍵與滑鼠觸發',
    monitorOn: '快速鍵生效', saved: '已儲存', example: '狀態列範例',
  },
}

const t = computed(() => TEXT[lang.value] ?? TEXT['zh-CN'])

const running = computed(() => props.variant !== 'offline')
const elevated = computed(() => props.variant === 'admin')

const statusText = computed(() => (running.value ? t.value.coreRunning : t.value.coreStopped))
const statusClass = computed(() => (running.value ? 'online' : 'offline'))
</script>

<template>
  <footer class="sb-statusbar" :aria-label="`${t.example}: ${statusText}`">
    <div class="sb-left">
      <span class="sb-status" :class="statusClass">
        <Shield v-if="running && elevated" :size="10" :stroke-width="2" class="sb-shield-dot" />
        <i v-else class="sb-dot"></i>
        {{ statusText }}
      </span>

      <template v-if="!running">
        <button class="sb-act sb-icon-only sb-ok" :title="t.startCore" type="button">
          <Play :size="14" :stroke-width="2" />
        </button>
        <button class="sb-act sb-icon-only sb-blue" :title="t.startAdmin" type="button">
          <Shield :size="14" :stroke-width="2" />
        </button>
      </template>
      <template v-else>
        <button v-if="!elevated" class="sb-act sb-icon-only sb-blue" :title="t.restartAdmin" type="button">
          <Shield :size="14" :stroke-width="2" />
        </button>
        <button class="sb-act sb-icon-only sb-warn" :title="t.restartCore" type="button">
          <RotateCw :size="14" :stroke-width="2" />
        </button>
        <button class="sb-act sb-icon-only sb-danger" :title="t.quitCore" type="button">
          <Power :size="14" :stroke-width="2" />
        </button>
      </template>
    </div>

    <div class="sb-right">
      <button class="sb-act sb-icon" :title="t.openLogDir" type="button">
        <ScrollText :size="14" :stroke-width="2" />
      </button>
      <button class="sb-act sb-icon" :title="t.theme" type="button">◐</button>

      <span v-if="running" class="sb-monitor" :title="t.monitorOnTitle">
        <i class="sb-dot sb-dot-ok"></i>
        {{ t.monitorOn }}
      </span>
      <span class="sb-save">
        <Check :size="12" :stroke-width="2" /> {{ t.saved }}
      </span>
    </div>
  </footer>
</template>

<style scoped>
/* 强调色取自应用状态栏（与主题无关，保持一致）；面板底色随文档主题走。 */
.sb-statusbar {
  --sb-ok: #2f9e63;
  --sb-warn: #d97706;
  --sb-danger: #e5484d;
  --sb-blue: #3b82f6;

  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  height: 34px;
  padding: 0 12px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
  font-size: 12px;
  color: var(--vp-c-text-1);
  user-select: none;
}
.sb-left,
.sb-right {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.sb-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--vp-c-text-2);
  white-space: nowrap;
}
.sb-status.online { color: var(--sb-ok); }
.sb-status.offline { color: var(--sb-danger); }

.sb-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
  display: inline-block;
}
.sb-dot-ok { background: var(--sb-ok); }
.sb-shield-dot { color: var(--sb-ok); }

.sb-act {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  border-radius: 5px;
  font-size: 12px;
  line-height: 1;
  color: var(--vp-c-text-1);
  border: 1px solid transparent;
  background: transparent;
  cursor: default;
}
.sb-icon {
  color: var(--vp-c-text-2);
  border-color: var(--vp-c-divider);
  background: var(--vp-c-bg);
}
.sb-ok { color: var(--sb-ok); }
.sb-blue { color: var(--sb-blue); }
.sb-warn { color: var(--sb-warn); }
.sb-danger { color: var(--sb-danger); }

.sb-monitor {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--vp-c-text-2);
  white-space: nowrap;
}
.sb-save {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--vp-c-text-2);
  white-space: nowrap;
}

@media (max-width: 560px) {
  .sb-save,
  .sb-monitor { display: none; }
}
</style>
