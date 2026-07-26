<script setup lang="ts">
import { computed } from 'vue'
import { useData } from 'vitepress'

interface Asset {
  name: string
  browser_download_url: string
}

const props = defineProps<{
  publishedAt: string
  htmlUrl: string
  assets: Asset[]
}>()

const { lang } = useData()

const TEXT = {
  'zh-CN': { publishedOn: '发布于', viewOnGitHub: '在 GitHub 上查看', downloads: '下载' },
  en: { publishedOn: 'Published', viewOnGitHub: 'View on GitHub', downloads: 'Downloads' },
  'zh-TW': { publishedOn: '發布於', viewOnGitHub: '在 GitHub 上檢視', downloads: '下載' },
} as const

const t = computed(() => TEXT[lang.value as keyof typeof TEXT] ?? TEXT['zh-CN'])

const publishedDate = computed(() =>
  new Date(props.publishedAt).toLocaleDateString(lang.value, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }),
)
</script>

<template>
  <p class="release-meta">
    {{ t.publishedOn }} {{ publishedDate }} ·
    <a :href="htmlUrl" target="_blank" rel="noopener">{{ t.viewOnGitHub }}</a>
  </p>

  <div v-if="assets.length" class="release-assets">
    <h2>{{ t.downloads }}</h2>
    <ul>
      <li v-for="asset in assets" :key="asset.browser_download_url">
        <a :href="asset.browser_download_url">{{ asset.name }}</a>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.release-meta {
  color: var(--vp-c-text-2);
  font-size: 14px;
}

.release-assets ul {
  padding-left: 1.2em;
}
</style>
