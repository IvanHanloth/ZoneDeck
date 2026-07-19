---
editLink: false
---

<script setup>
import { useData } from 'vitepress'

const { params } = useData()
</script>

# {{ $params.title }}

<ReleaseMeta :published-at="params.publishedAt" :html-url="params.htmlUrl" :assets="params.assets" />

<!-- @content -->
