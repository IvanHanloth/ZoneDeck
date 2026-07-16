<script setup lang="ts">
interface Asset {
  name: string
  browser_download_url: string
}

const props = defineProps<{
  publishedAt: string
  htmlUrl: string
  assets: Asset[]
}>()

const publishedDate = new Date(props.publishedAt).toLocaleDateString('zh-CN', {
  year: 'numeric',
  month: 'long',
  day: 'numeric',
})
</script>

<template>
  <p class="release-meta">
    发布于 {{ publishedDate }} ·
    <a :href="htmlUrl" target="_blank" rel="noopener">在 GitHub 上查看</a>
  </p>

  <div v-if="assets.length" class="release-assets">
    <h2>下载</h2>
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
